//! A segment's exact key map. Its keys lie one after another in one buffer, so that freeing a map
//! is a few frees, not one for each key. One buffer for each key made that free stall the writer, whose allocations
//! share the buffers' allocator arena (issue 154).
use super::ActiveKey;
use std::{
    collections::HashMap,
    hash::{BuildHasher, BuildHasherDefault, Hasher, RandomState},
};

/// A map moves all its keys when it grows, and the writer waits. Each of many small maps moves
/// only its own part, at another time than the others (issue 200).
const INDEXES: usize = 64;

type Index = HashMap<u64, u32, BuildHasherDefault<Hashed>>;

/// Passes a slot through: it comes from a randomly keyed hash already.
#[derive(Default)]
struct Hashed(u64);

impl Hasher for Hashed {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, _: &[u8]) {
        unreachable!("an index hashes only u64 slots")
    }

    fn write_u64(&mut self, slot: u64) {
        self.0 = slot;
    }
}

// A segment's keys fit the `u32` ends.
const _: () =
    assert!(crate::codec::KEYED_RECORD_LIMIT * crate::codec::KEY_LEN_MAX <= u32::MAX as usize);

pub(in crate::engine) struct KeyMap {
    random: RandomState,
    /// The entry of each key, by the key's hash, or by the next free number after it when another
    /// key has that hash. No key is ever removed, so a lookup stops at the first free number.
    slots: Vec<Index>,
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
        Self::with_capacity(0, 0)
    }

    /// An empty map with room for as many keys and key bytes as `other` holds.
    pub fn sized_like(other: &Self) -> Self {
        Self::with_capacity(other.len(), other.bytes.len())
    }

    fn with_capacity(keys: usize, bytes: usize) -> Self {
        let index = || Index::with_capacity_and_hasher(keys.div_ceil(INDEXES), Default::default());
        Self {
            random: RandomState::new(),
            slots: (0..INDEXES).map(|_| index()).collect(),
            entries: Vec::with_capacity(keys),
            bytes: Vec::with_capacity(bytes),
        }
    }

    pub fn reserve(&mut self, additional: usize) {
        for index in &mut self.slots {
            index.reserve(additional.div_ceil(INDEXES));
        }
        self.entries.reserve(additional);
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[cfg(test)]
    pub fn capacity(&self) -> usize {
        let slots = self.slots.iter().map(Index::capacity).sum::<usize>();
        slots.min(self.entries.capacity())
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
        self.random.hash_one(key)
    }

    /// An index takes its buckets from a slot's low bits and its tags from the top ones, so other
    /// bits choose the index.
    fn index(slot: u64) -> usize {
        (slot >> 32) as usize % INDEXES
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
        while let Some(&entry) = self.slots[Self::index(slot)].get(&slot) {
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
                self.slots[Self::index(slot)].insert(slot, self.entries.len() as u32);
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

    fn indexes(map: &KeyMap) -> impl Iterator<Item = &Index> {
        map.slots.iter()
    }

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
            ..
        } = KeyMap::new();
        assert_eq!(slots.len(), INDEXES);
        assert!(!frees_each(slots.into_iter().flatten()));
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

    /// Given an empty map, and one sized for 226,165 keys as a roll leaves it (issue 200)
    /// When 120,000 keys go into the first, and 235,000 into the second
    /// Then no insert into a map of 10,000 keys or more moves over 1/32 of them in the index, so
    /// growth stays off the writer's latency at any key count
    ///
    /// Cases: KEY-MEMORY
    #[test]
    fn an_insert_moves_a_small_part_of_the_index() {
        let mut rolled = KeyMap::new();
        for i in 0..226_165u32 {
            rolled.insert(&i.to_le_bytes(), at(i));
        }
        for (mut map, keys) in [
            (KeyMap::new(), 120_000u32),
            (KeyMap::sized_like(&rolled), 235_000),
        ] {
            let mut moves = 0;
            for i in 0..keys {
                let before: Vec<_> = indexes(&map)
                    .map(|index| (index.capacity(), index.len()))
                    .collect();
                map.insert(&i.to_le_bytes(), at(i));
                let moved: usize = indexes(&map)
                    .zip(before)
                    .filter(|(index, (capacity, _))| index.capacity() != *capacity)
                    .map(|(_, (_, len))| len)
                    .sum();
                moves += usize::from(moved > 0);
                assert!(
                    i < 10_000 || moved <= i as usize / 32,
                    "{moved} of {i} keys moved"
                );
            }
            assert!(moves > 0, "the map outgrew its room");
        }
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
