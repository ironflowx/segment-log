//! A segment's exact key map. Its keys lie one after another in one buffer, so that freeing a map
//! is a few frees. One buffer for each key made that free stall the writer, whose allocations
//! share the buffers' allocator arena (issue 154).
use super::ActiveKey;
use std::{collections::HashMap, hash::BuildHasher};

// A segment's keys fit the `u32` ends.
const _: () =
    assert!(crate::codec::KEYED_RECORD_LIMIT * crate::codec::KEY_LEN_MAX <= u32::MAX as usize);

#[derive(Default)]
pub(in crate::engine) struct KeyMap {
    /// The entry of each key, by the key's hash, or by the next free number after it when another
    /// key has that hash. No key is ever removed, so a lookup stops at the first free number.
    slots: HashMap<u64, u32>,
    entries: Vec<Entry>,
    bytes: Vec<u8>,
}

struct Entry {
    /// Where the key ends in `bytes`. It starts where the entry before it ends.
    end: u32,
    at: ActiveKey,
}

impl KeyMap {
    pub fn new() -> Self {
        Self::default()
    }

    /// An empty map with room for as many keys and key bytes as `other` holds.
    pub fn sized_like(other: &Self) -> Self {
        Self {
            slots: HashMap::with_capacity(other.len()),
            entries: Vec::with_capacity(other.len()),
            bytes: Vec::with_capacity(other.bytes.len()),
        }
    }

    pub fn reserve(&mut self, additional: usize) {
        self.slots.reserve(additional);
        self.entries.reserve(additional);
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[cfg(test)]
    pub fn capacity(&self) -> usize {
        self.slots.capacity().min(self.entries.capacity())
    }

    pub fn get(&self, key: &[u8]) -> Option<ActiveKey> {
        self.find(self.hash(key), key)
            .ok()
            .map(|entry| self.entries[entry].at)
    }

    /// A key that the map already holds gets the new place.
    pub fn insert(&mut self, key: &[u8], at: ActiveKey) {
        self.insert_hashed(self.hash(key), key, at);
    }

    pub fn iter(&self) -> impl Iterator<Item = (&[u8], ActiveKey)> {
        (0..self.entries.len()).map(|entry| (self.key(entry), self.entries[entry].at))
    }

    fn hash(&self, key: &[u8]) -> u64 {
        self.slots.hasher().hash_one(key)
    }

    fn key(&self, entry: usize) -> &[u8] {
        let start = match entry {
            0 => 0,
            _ => self.entries[entry - 1].end,
        };
        &self.bytes[start as usize..self.entries[entry].end as usize]
    }

    /// The key's entry, or the free slot where it belongs.
    fn find(&self, mut slot: u64, key: &[u8]) -> Result<usize, u64> {
        while let Some(&entry) = self.slots.get(&slot) {
            if self.key(entry as usize) == key {
                return Ok(entry as usize);
            }
            slot = slot.wrapping_add(1);
        }
        Err(slot)
    }

    fn insert_hashed(&mut self, hash: u64, key: &[u8], at: ActiveKey) {
        match self.find(hash, key) {
            Ok(entry) => self.entries[entry].at = at,
            Err(slot) => {
                self.slots.insert(slot, self.entries.len() as u32);
                self.bytes.extend_from_slice(key);
                self.entries.push(Entry {
                    end: self.bytes.len() as u32,
                    at,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(record: u32) -> ActiveKey {
        ActiveKey {
            position: 7,
            record,
        }
    }

    /// Given a key map (issue 154)
    /// When it is freed
    /// Then no key is freed by itself: only the map's few buffers are
    ///
    /// Cases: KEY-MEMORY
    #[test]
    fn freeing_the_map_frees_no_key_by_itself() {
        fn frees_each<T>(_: impl Iterator<Item = T>) -> bool {
            std::mem::needs_drop::<T>()
        }
        let KeyMap {
            slots,
            entries,
            bytes,
        } = KeyMap::new();
        assert!(!frees_each(slots.into_iter()));
        assert!(!frees_each(entries.into_iter()));
        assert!(!frees_each(bytes.into_iter()));
    }

    /// Given keys of different lengths, where some start with another, and one is inserted twice
    /// When each is looked up, and the map is read in full
    /// Then each gives its newest place, and an absent key, a prefix or a join of two gives none
    ///
    /// Cases: KEY-MEMORY
    #[test]
    fn a_key_gives_its_newest_place_and_no_other_key_does() {
        let mut map = KeyMap::new();
        for (record, key) in [&b"a"[..], b"ab", b"abc", &[0; 256], b"a"]
            .iter()
            .enumerate()
        {
            map.insert(key, at(record as u32));
        }
        let got = |key: &[u8]| map.get(key).map(|at| at.record);
        assert_eq!(map.len(), 4);
        assert_eq!(
            [got(b"a"), got(b"ab"), got(b"abc"), got(&[0; 256])],
            [Some(4), Some(1), Some(2), Some(3)]
        );
        assert_eq!(
            [got(b""), got(b"b"), got(b"aab"), got(&[0; 255])],
            [None; 4]
        );
        let all: Vec<_> = map
            .iter()
            .map(|(key, at)| (key.to_vec(), at.record))
            .collect();
        assert_eq!(
            all,
            [
                (b"a".to_vec(), 4),
                (b"ab".to_vec(), 1),
                (b"abc".to_vec(), 2),
                (vec![0; 256], 3)
            ]
        );
    }

    /// Given three keys with one hash, at the end of the hash range, and a key whose own hash is
    /// the slot that one of them took
    /// When each is looked up and one is inserted again
    /// Then each is found by its bytes, and a fourth key with that hash is absent
    ///
    /// Cases: KEY-COLLISION
    #[test]
    fn keys_that_share_a_hash_stay_apart() {
        let mut map = KeyMap::new();
        for (record, key) in [b"x", b"y", b"z"].iter().enumerate() {
            map.insert_hashed(u64::MAX, *key, at(record as u32));
        }
        map.insert_hashed(0, b"w", at(3));
        map.insert_hashed(u64::MAX, b"y", at(4));
        let got = |hash, key: &[u8]| map.find(hash, key).ok().map(|i| map.entries[i].at.record);
        assert_eq!(map.len(), 4);
        assert_eq!(
            [
                got(u64::MAX, b"x"),
                got(u64::MAX, b"y"),
                got(u64::MAX, b"z"),
                got(0, b"w")
            ],
            [Some(0), Some(4), Some(2), Some(3)]
        );
        assert_eq!(got(u64::MAX, b"v"), None);
    }
}
