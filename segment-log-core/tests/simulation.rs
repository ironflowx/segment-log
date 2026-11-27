//! The simulation API from outside the crate (ADR 0072): a storage model of this test's own, a
//! manual clock, and seeded runs that crash, inject faults and check the oracle (testing.md).
use segment_log_core::simulation::{
    Call, Clock, Lock, Op, Step, SteppedLog, Stop, Storage, Task, Value,
};
use segment_log_core::*;
use std::{
    collections::{BTreeMap, BTreeSet},
    io,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};

fn next(seed: &mut u64) -> u64 {
    *seed = seed.wrapping_add(0x9e3779b97f4a7c15);
    let mut x = *seed;
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d049bb133111eb);
    x ^ (x >> 31)
}

// ---- The storage model: durable and volatile bytes per file, and entries per directory. ----

#[derive(Clone, Default)]
struct File {
    durable: Vec<u8>,
    volatile: Vec<u8>,
    /// Bytes a successful flush makes durable. A failed flush clears them: the page cache may
    /// mark them clean, so only a new write makes them durable again.
    eligible: Vec<bool>,
}

#[derive(Clone, Default)]
struct Dir {
    durable: BTreeMap<PathBuf, u64>,
    volatile: BTreeMap<PathBuf, u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Crash {
    Kill,
    PowerLoss,
    PowerLossTorn,
}

#[derive(Clone, Copy, Debug)]
enum Fault {
    FlushError,
    ReadError,
    Short(usize),
    Full,
}

struct State {
    files: BTreeMap<u64, File>,
    /// A file's entry names its id; a directory's names 0.
    dirs: BTreeMap<PathBuf, Dir>,
    next_id: u64,
    /// After a failed directory flush, writes fail (volume requirement V3).
    metadata_failed: bool,
    rng: u64,
    boot: u64,
    /// The crash point has passed: every call fails until the crash image is built.
    dead: bool,
    locks: BTreeSet<PathBuf>,
    /// One in this many changing calls is a crash point, and one in `fault_rate` gets a fault.
    crash_rate: u64,
    fault_rate: u64,
    /// Whether the test is inside a call to the log (STEPPED-NO-HIDDEN-WORK).
    inside: bool,
    calls: u64,
    hidden: Vec<String>,
    trace: Vec<String>,
}

#[derive(Clone)]
struct Disk(Arc<Mutex<State>>);

impl Disk {
    fn new(seed: u64) -> Self {
        Self(Arc::new(Mutex::new(State {
            files: BTreeMap::new(),
            dirs: BTreeMap::from([(PathBuf::from("/"), Dir::default())]),
            next_id: 1,
            metadata_failed: false,
            rng: seed,
            boot: 0,
            dead: false,
            locks: BTreeSet::new(),
            crash_rate: 0,
            fault_rate: 0,
            inside: false,
            calls: 0,
            hidden: vec![],
            trace: vec![],
        })))
    }

    fn state(&self) -> std::sync::MutexGuard<'_, State> {
        self.0.lock().unwrap()
    }

    fn file<'a>(s: &'a mut State, path: &Path) -> Option<&'a mut File> {
        let id = *s.dirs.get(path.parent()?)?.volatile.get(path)?;
        s.files.get_mut(&id).filter(|_| id != 0)
    }

    /// The bytes of each segment, by base offset, as a reader would see them now.
    fn segments(&self) -> Vec<(String, Vec<u8>)> {
        let mut s = self.state();
        let names: Vec<PathBuf> = s
            .dirs
            .get(Path::new("/log"))
            .map(|d| d.volatile.keys().cloned().collect())
            .unwrap_or_default();
        names
            .into_iter()
            .filter(|p| p.extension().is_some_and(|e| e == "seg"))
            .map(|p| {
                let bytes = Self::file(&mut s, &p)
                    .map(|f| f.volatile.clone())
                    .unwrap_or_default();
                (p.display().to_string(), bytes)
            })
            .collect()
    }

    fn crash(&self, mode: Crash, mut seed: u64) {
        let mut s = self.state();
        s.trace.push(format!("crash {mode:?} {seed}"));
        s.dead = false;
        s.boot += 1;
        s.locks.clear();
        s.metadata_failed = false;
        if mode == Crash::Kill {
            return;
        }
        for file in s.files.values_mut() {
            let len = file.volatile.len().max(file.durable.len());
            let mut bytes = file.durable.clone();
            bytes.resize(len, 0);
            for page in (0..file.volatile.len()).step_by(4096) {
                if next(&mut seed) & 1 == 0 {
                    continue;
                }
                let end = (page + 4096).min(file.volatile.len());
                bytes[page..end].copy_from_slice(&file.volatile[page..end]);
                if mode == Crash::PowerLossTorn {
                    // A sector that did not change never tears.
                    for sector in (page..end).step_by(512) {
                        let end = (sector + 512).min(end);
                        let changed = |i: usize| {
                            file.volatile[i] != file.durable.get(i).copied().unwrap_or(0)
                        };
                        if (sector..end).any(changed) {
                            let start = sector + next(&mut seed) as usize % (end - sector);
                            for (i, byte) in bytes.iter_mut().enumerate().take(end).skip(start) {
                                if changed(i) {
                                    *byte = next(&mut seed) as u8;
                                }
                            }
                        }
                    }
                }
            }
            file.volatile = bytes;
            file.eligible = vec![false; len];
        }
        for dir in s.dirs.values_mut() {
            if next(&mut seed) & 1 == 0 {
                dir.volatile = dir.durable.clone();
            }
        }
        // A directory whose own entry was lost is gone, and so is every file in it.
        let mut reachable = BTreeSet::from([PathBuf::from("/")]);
        loop {
            let before = reachable.len();
            for (path, dir) in &s.dirs {
                if reachable.contains(path) {
                    reachable.extend(dir.volatile.keys().cloned());
                }
            }
            if before == reachable.len() {
                break;
            }
        }
        s.dirs.retain(|path, _| reachable.contains(path));
        let ids: BTreeSet<u64> = s
            .dirs
            .values()
            .flat_map(|d| d.volatile.values().copied())
            .collect();
        s.files.retain(|id, _| ids.contains(id));
    }

    fn apply(s: &mut State, call: &Call, fault: Option<Fault>) -> io::Result<Value> {
        if matches!(fault, Some(Fault::ReadError)) {
            return Err(io::ErrorKind::Other.into());
        }
        let path = &call.path;
        let changing = !matches!(call.op, Op::Read(..) | Op::Open | Op::List | Op::Random(_));
        if s.metadata_failed && changing {
            return Err(io::ErrorKind::Other.into());
        }
        let parent = path.parent().unwrap_or(Path::new("/")).to_owned();
        let exists = |s: &State| {
            s.dirs.contains_key(path)
                || s.dirs
                    .get(&parent)
                    .is_some_and(|d| d.volatile.contains_key(path))
        };
        match &call.op {
            Op::CreateDir | Op::Create => {
                if exists(s) {
                    return Err(io::ErrorKind::AlreadyExists.into());
                }
                let id = match call.op {
                    Op::CreateDir => 0,
                    _ => s.next_id,
                };
                s.dirs
                    .get_mut(&parent)
                    .ok_or(io::ErrorKind::NotFound)?
                    .volatile
                    .insert(path.clone(), id);
                if id == 0 {
                    s.dirs.insert(path.clone(), Dir::default());
                } else {
                    s.next_id += 1;
                    s.files.insert(id, File::default());
                }
                Ok(Value::Unit)
            }
            Op::List => {
                let dir = s.dirs.get(path).ok_or(io::ErrorKind::NotFound)?;
                Ok(Value::Names(
                    dir.volatile
                        .keys()
                        .map(|p| PathBuf::from(p.file_name().unwrap()))
                        .collect(),
                ))
            }
            Op::SyncDir => {
                if matches!(fault, Some(Fault::FlushError)) {
                    s.metadata_failed = true;
                    return Err(io::ErrorKind::Other.into());
                }
                let dir = s.dirs.get_mut(path).ok_or(io::ErrorKind::NotFound)?;
                dir.durable = dir.volatile.clone();
                Ok(Value::Unit)
            }
            Op::Remove => {
                let dir = s.dirs.get_mut(&parent).ok_or(io::ErrorKind::NotFound)?;
                dir.volatile.remove(path).ok_or(io::ErrorKind::NotFound)?;
                Ok(Value::Unit)
            }
            Op::Random(len) => Ok(Value::Bytes(
                (0..*len).map(|_| next(&mut s.rng) as u8).collect(),
            )),
            op => {
                let file = Self::file(s, path).ok_or(io::ErrorKind::NotFound)?;
                match op {
                    Op::Open => Ok(Value::Length(file.volatile.len() as u64)),
                    Op::Read(pos, len) => {
                        let start = (*pos as usize).min(file.volatile.len());
                        let end = start.saturating_add(*len).min(file.volatile.len());
                        Ok(Value::Bytes(file.volatile[start..end].to_vec()))
                    }
                    Op::Append(bytes) | Op::Write(_, bytes) => {
                        let pos = match op {
                            Op::Write(pos, _) => *pos as usize,
                            _ => file.volatile.len(),
                        };
                        let len = match fault {
                            Some(Fault::Short(n)) => n.min(bytes.len()),
                            Some(Fault::Full) => 0,
                            _ => bytes.len(),
                        };
                        if len > 0 {
                            let end = pos + len;
                            if end > file.volatile.len() {
                                file.volatile.resize(end, 0);
                                file.eligible.resize(end, true);
                            }
                            file.volatile[pos..end].copy_from_slice(&bytes[..len]);
                            file.eligible[pos..end].fill(true);
                        }
                        match fault {
                            Some(Fault::Full) => Err(io::ErrorKind::StorageFull.into()),
                            Some(Fault::Short(_)) => Err(io::ErrorKind::Other.into()),
                            _ => Ok(Value::Unit),
                        }
                    }
                    Op::SyncData | Op::SyncAll => {
                        if matches!(fault, Some(Fault::FlushError)) {
                            file.eligible.fill(false);
                            return Err(io::ErrorKind::Other.into());
                        }
                        let old = file.durable.len();
                        file.durable.resize(file.volatile.len().max(old), 0);
                        for (i, eligible) in file.eligible.iter().enumerate() {
                            if *eligible {
                                file.durable[i] = file.volatile[i];
                            }
                        }
                        let floor = old.min(file.volatile.len());
                        let len = file
                            .eligible
                            .iter()
                            .rposition(|b| *b)
                            .map_or(floor, |i| (i + 1).max(floor));
                        file.durable.truncate(len);
                        file.eligible.fill(false);
                        Ok(Value::Unit)
                    }
                    Op::Truncate(len) => {
                        file.volatile.resize(*len as usize, 0);
                        file.eligible.resize(*len as usize, true);
                        Ok(Value::Unit)
                    }
                    other => Err(io::Error::other(format!("unknown op {other:?}"))),
                }
            }
        }
    }
}

struct Guard {
    disk: Disk,
    path: PathBuf,
    boot: u64,
}
impl Lock for Guard {}
impl Drop for Guard {
    fn drop(&mut self) {
        let mut s = self.disk.state();
        if s.boot == self.boot {
            s.locks.remove(&self.path);
        }
    }
}

impl Storage for Disk {
    fn call(&self, call: Call) -> io::Result<Value> {
        let mut s = self.state();
        s.calls += 1;
        if !s.inside {
            s.hidden.push(format!("{call:?}"));
        }
        let summary = match &call.op {
            Op::Append(b) => format!("Append({})", b.len()),
            Op::Write(p, b) => format!("Write({p}, {})", b.len()),
            op => format!("{op:?}"),
        };
        let label = format!("{} {:?} {summary}", call.path.display(), call.purpose);
        if s.dead {
            s.trace.push(format!("{label} -> dead"));
            return Err(io::ErrorKind::Interrupted.into());
        }
        let changing = !matches!(call.op, Op::Read(..) | Op::Open | Op::List | Op::Random(_));
        if changing && s.crash_rate > 0 && next(&mut s.rng).is_multiple_of(s.crash_rate) {
            // The crash point: the call may or may not have reached the device.
            if next(&mut s.rng) & 1 == 0 {
                let _ = Self::apply(&mut s, &call, None);
            }
            s.dead = true;
            s.trace.push(format!("{label} -> crash point"));
            return Err(io::ErrorKind::Interrupted.into());
        }
        let reading = matches!(call.op, Op::Read(..));
        let fault = ((changing || reading)
            && s.fault_rate > 0
            && next(&mut s.rng).is_multiple_of(s.fault_rate))
        .then(|| match &call.op {
            Op::Read(..) => Fault::ReadError,
            Op::Append(b) | Op::Write(_, b) => match next(&mut s.rng) % 2 {
                0 => Fault::Short(next(&mut s.rng) as usize % b.len().max(1)),
                _ => Fault::Full,
            },
            _ => Fault::FlushError,
        });
        let result = Self::apply(&mut s, &call, fault);
        let shown = match &result {
            Ok(Value::Bytes(b)) => format!("{} bytes", b.len()),
            other => format!("{other:?}"),
        };
        s.trace.push(format!("{label} {fault:?} -> {shown}"));
        result
    }

    fn lock(&self, path: &Path) -> io::Result<Box<dyn Lock>> {
        let mut s = self.state();
        if !s.inside {
            s.hidden.push(format!("lock {path:?}"));
        }
        if !s.locks.insert(path.to_owned()) {
            return Err(io::ErrorKind::WouldBlock.into());
        }
        Ok(Box::new(Guard {
            disk: self.clone(),
            path: path.to_owned(),
            boot: s.boot,
        }))
    }
}

/// The test moves both readings; each reading is traced.
struct Manual {
    disk: Disk,
    now: Mutex<(Duration, u64)>,
}
impl Clock for Manual {
    fn monotonic(&self) -> Duration {
        let now = self.now.lock().unwrap().0;
        let mut s = self.disk.state();
        if !s.inside {
            s.hidden.push("monotonic".into());
        }
        s.trace.push(format!("monotonic {now:?}"));
        now
    }
    fn wall_millis(&self) -> u64 {
        let now = self.now.lock().unwrap().1;
        let mut s = self.disk.state();
        if !s.inside {
            s.hidden.push("wall".into());
        }
        s.trace.push(format!("wall {now}"));
        now
    }
}

// ---- The driver and its oracle. ----

#[derive(Clone, Debug, PartialEq)]
enum Fate {
    Waiting,
    Acked(Vec<(u64, u64)>),
    /// Known to have appended nothing.
    Rejected,
    Unknown,
}

/// Stream, key, payload and expected revision.
type Sending = (Vec<u8>, Option<Vec<u8>>, Vec<u8>, ExpectedRevision);

struct Sent {
    records: Vec<Sending>,
    fate: Fate,
    /// The highest watermark seen before it was sent: a new record never goes below it (I6).
    floor: u64,
}

struct Run {
    seed: u64,
    rng: u64,
    disk: Disk,
    clock: Arc<Manual>,
    log: Option<SteppedLog>,
    sent: Vec<Sent>,
    waiting: Vec<(usize, Pending)>,
    /// Every record a read returned, by offset.
    seen: BTreeMap<u64, Vec<u8>>,
    high: u64,
    heads: BTreeMap<Vec<u8>, Option<u64>>,
    sealed: bool,
}

const STREAMS: [&[u8]; 3] = [b"s0", b"s1", b"s2"];

fn options() -> Options {
    Options {
        linger: Duration::from_millis(2),
        roll_bytes: 600,
        queue_capacity: 8,
        ..Options::default()
    }
}

fn expected(head: Option<u64>) -> ExpectedRevision {
    head.map_or(ExpectedRevision::Empty, ExpectedRevision::At)
}

fn copy(e: &ExpectedRevision) -> ExpectedRevision {
    match e {
        ExpectedRevision::Empty => ExpectedRevision::Empty,
        ExpectedRevision::At(n) => ExpectedRevision::At(*n),
    }
}

impl Run {
    fn new(seed: u64) -> Self {
        let disk = Disk::new(seed);
        let clock = Arc::new(Manual {
            disk: disk.clone(),
            now: Mutex::new((Duration::from_secs(1), 1_700_000_000_000)),
        });
        Self {
            seed,
            rng: seed ^ 0x5eed,
            disk,
            clock,
            log: None,
            sent: vec![],
            waiting: vec![],
            seen: BTreeMap::new(),
            high: 0,
            heads: BTreeMap::new(),
            sealed: false,
        }
    }

    fn roll(&mut self, n: u64) -> u64 {
        next(&mut self.rng) % n
    }

    /// Runs `f` as a call to the log: storage calls and clock readings are allowed only here.
    fn inside<T>(&self, f: impl FnOnce() -> T) -> T {
        self.disk.state().inside = true;
        let result = f();
        self.disk.state().inside = false;
        result
    }

    fn faults(&self, on: bool) {
        let mut s = self.disk.state();
        (s.crash_rate, s.fault_rate) = if on { (120, 150) } else { (0, 0) };
    }

    fn submit(&self, index: usize) -> Submit {
        Submit {
            records: self.sent[index]
                .records
                .iter()
                .map(|(stream, key, payload, e)| NewRecord {
                    stream: stream.clone(),
                    expected: copy(e),
                    key: key.clone(),
                    payload: payload.clone(),
                })
                .collect(),
        }
    }

    fn append(&mut self) {
        let index = self.sent.len();
        let count = 1 + self.roll(3) as usize;
        let keyed = self.roll(3);
        let mut records = vec![];
        let mut heads = self.heads.clone();
        for i in 0..count {
            let stream = STREAMS[self.roll(STREAMS.len() as u64) as usize].to_vec();
            let head = heads.get(&stream).copied().flatten();
            let key =
                (keyed > 0 && (keyed == 2 || i == 0)).then(|| format!("k{index}:{i}").into_bytes());
            let payload =
                format!("{index}:{i}/{count}:{}", "x".repeat(self.roll(80) as usize)).into_bytes();
            records.push((stream.clone(), key, payload, expected(head)));
            heads.insert(stream, Some(head.map_or(0, |h| h + 1)));
        }
        self.heads = heads;
        self.sent.push(Sent {
            records,
            fate: Fate::Waiting,
            floor: self.high,
        });
        let submit = self.submit(index);
        let log = self.log.as_ref().unwrap();
        match self.inside(|| log.append(submit)) {
            Ok(pending) => self.waiting.push((index, pending)),
            Err(_) => self.sent[index].fate = Fate::Rejected,
        }
    }

    fn fate(&mut self, index: usize, result: Result<Committed, CommitError>) {
        let fate = match result {
            Ok(committed) => {
                let positions: Vec<_> = committed
                    .records
                    .iter()
                    .map(|p| (p.offset, p.revision))
                    .collect();
                for ((stream, ..), (_, revision)) in self.sent[index].records.iter().zip(&positions)
                {
                    let head = self.heads.entry(stream.clone()).or_default();
                    *head = Some(head.map_or(*revision, |h| h.max(*revision)));
                }
                if !committed.deduplicated {
                    let floor = self.sent[index].floor;
                    assert!(
                        positions.iter().all(|(o, _)| *o >= floor),
                        "ORACLE-10: offset below {floor}"
                    );
                }
                Fate::Acked(positions)
            }
            Err(CommitError::Indeterminate) => Fate::Unknown,
            Err(CommitError::WrongRevision { index: at, actual }) => {
                self.heads
                    .insert(self.sent[index].records[at].0.clone(), actual);
                Fate::Rejected
            }
            Err(_) => Fate::Rejected,
        };
        self.sent[index].fate = fate;
    }

    /// Takes every answer that is ready.
    fn collect(&mut self) {
        for (index, pending) in std::mem::take(&mut self.waiting) {
            match pending.try_wait() {
                Ok(result) => self.fate(index, result),
                Err(pending) => self.waiting.push((index, pending)),
            }
        }
    }

    fn read(&mut self) {
        let watermark = self.log.as_ref().unwrap().watermark();
        let from = self.roll(watermark + 1);
        let max = 1 + self.roll(8) as usize;
        let log = self.log.as_ref().unwrap();
        let records = self.inside(|| log.read(from, max));
        if let Ok(records) = records {
            for r in records {
                assert!(
                    r.offset < log.watermark(),
                    "ORACLE-8: read past the watermark"
                );
                self.seen.insert(r.offset, r.payload);
            }
        }
        self.high = self.high.max(log.watermark());
    }

    fn step(&mut self, task: Task) -> Step {
        let log = self.log.as_mut().unwrap();
        let disk = self.disk.clone();
        disk.state().inside = true;
        let step = log.step(task);
        let mut s = disk.state();
        s.inside = false;
        s.trace.push(format!("step {task:?} -> {step:?}"));
        drop(s);
        self.high = self.high.max(log.watermark());
        step
    }

    /// Opens the log on the current image with faults and crash points on, crashing and retrying
    /// when the open itself crashes or fails, and checks that recovery left sealed segments as
    /// they were (ORACLE-6). It leaves faults off.
    fn open(&mut self) {
        for attempt in 0..100 {
            // Later attempts run without faults, so that every run ends.
            self.faults(attempt < 10);
            let before = self.disk.segments();
            let (disk, clock) = (self.disk.clone(), self.clock.clone());
            match self.inside(|| SteppedLog::open("/log", options(), Arc::new(disk), clock)) {
                Ok(log) => {
                    let after = self.disk.segments();
                    let keep = before.len().saturating_sub(2);
                    assert_eq!(
                        after.get(..keep),
                        Some(&before[..keep]),
                        "ORACLE-6: a sealed segment changed"
                    );
                    // The second-newest may only have its closing run cut or completed: its
                    // header and groups stay.
                    if let (true, Some(pending), Some(now)) =
                        (before.len() >= 2, before.get(keep), after.get(keep))
                        && pending.0 == now.0
                    {
                        assert_eq!(now.1.get(..64), pending.1.get(..64), "ORACLE-6");
                    }
                    self.log = Some(log);
                    self.faults(false);
                    return;
                }
                Err(OpenError::Io(_)) => {
                    let (dead, failed) = {
                        let s = self.disk.state();
                        (s.dead, s.metadata_failed)
                    };
                    // A crash during recovery, or a failed directory flush, which the volume
                    // refuses writes after until it restarts (V3).
                    if dead || failed {
                        let mode = if dead { self.crash_mode() } else { Crash::Kill };
                        let seed = next(&mut self.rng);
                        self.disk.crash(mode, seed);
                    }
                }
                Err(error) => panic!("open failed: {error:?}"),
            }
        }
        panic!("open kept failing");
    }

    fn records(&self) -> Vec<Record> {
        let log = self.log.as_ref().unwrap();
        let mut all = vec![];
        loop {
            let batch = self.inside(|| log.read(all.len() as u64, 64)).unwrap();
            if batch.is_empty() {
                return all;
            }
            all.extend(batch);
        }
    }

    /// Drops the log: a crash for a stepped log, so it must make no storage call.
    fn drop_log(&mut self) {
        let calls = self.disk.state().calls;
        self.log = None;
        assert_eq!(
            self.disk.state().calls,
            calls,
            "STEPPED-NO-HIDDEN-WORK: drop made a storage call"
        );
        self.collect();
        assert!(
            self.waiting.is_empty(),
            "a dropped log answers every pending submit"
        );
    }

    /// After a crash, a poison or a close: reopen and check oracle checks 1–5 and 8–10.
    fn verify(&mut self, crash: Option<Crash>) {
        self.drop_log();
        // After a failed directory flush the volume refuses writes until it restarts (V3).
        let crash = crash.or(self.disk.state().metadata_failed.then_some(Crash::Kill));
        if let Some(mode) = crash {
            let seed = next(&mut self.rng);
            self.disk.crash(mode, seed);
        }
        self.open();
        let records = self.records();
        // ORACLE-2 and ORACLE-3: whole submits, in order, each at most once.
        let mut found: BTreeMap<usize, u64> = BTreeMap::new();
        let mut at = 0;
        while at < records.len() {
            let text = String::from_utf8(records[at].payload.clone()).unwrap();
            let (index, rest) = text.split_once(':').unwrap();
            let index: usize = index.parse().unwrap();
            let count: usize = rest
                .split_once('/')
                .unwrap()
                .1
                .split_once(':')
                .unwrap()
                .0
                .parse()
                .unwrap();
            let sent = &self.sent[index];
            assert_ne!(
                sent.fate,
                Fate::Rejected,
                "seed {}: a rejected submit {index} was kept",
                self.seed
            );
            assert!(
                found.insert(index, at as u64).is_none(),
                "submit {index} kept twice"
            );
            assert!(
                at + count <= records.len(),
                "ORACLE-3: submit {index} kept in part"
            );
            for (i, (stream, key, payload, _)) in sent.records.iter().enumerate() {
                let r = &records[at + i];
                assert_eq!(
                    (&r.stream, &r.key, &r.payload),
                    (stream, key, payload),
                    "ORACLE-3"
                );
                assert_eq!(r.offset, (at + i) as u64);
            }
            at += count;
        }
        // ORACLE-1: every acknowledged record is kept where it was acknowledged.
        for (index, sent) in self.sent.iter().enumerate() {
            if let Fate::Acked(positions) = &sent.fate {
                let first = found.get(&index).copied();
                assert_eq!(
                    first,
                    Some(positions[0].0),
                    "seed {}: ORACLE-1 lost submit {index}",
                    self.seed
                );
                for (i, (offset, revision)) in positions.iter().enumerate() {
                    assert_eq!(records[*offset as usize].revision, *revision, "ORACLE-1");
                    assert_eq!(*offset, first.unwrap() + i as u64);
                }
            }
        }
        // ORACLE-8 and ORACLE-9: what readers saw is kept, and the watermark never moves back.
        for (offset, payload) in &self.seen {
            assert_eq!(
                records.get(*offset as usize).map(|r| &r.payload),
                Some(payload),
                "ORACLE-8"
            );
        }
        let log = self.log.as_ref().unwrap();
        assert!(log.watermark() >= self.high, "ORACLE-9");
        self.high = log.watermark();
        // ORACLE-4: a fully keyed submit of unknown outcome that was kept dedupes on retry.
        let retries: Vec<usize> = (0..self.sent.len())
            .filter(|i| self.sent[*i].fate == Fate::Unknown && found.contains_key(i))
            .filter(|i| self.sent[*i].records.iter().all(|r| r.1.is_some()))
            .collect();
        for index in retries {
            let submit = self.submit(index);
            let log = self.log.as_ref().unwrap();
            if let Ok(pending) = self.inside(|| log.append(submit)) {
                while self.step(Task::Writer) == Step::Worked {}
                let Ok(Ok(committed)) = pending.try_wait() else {
                    panic!("ORACLE-4: a retry of a kept submit was not answered")
                };
                assert!(committed.deduplicated, "ORACLE-4");
                assert_eq!(committed.records[0].offset, found[&index], "ORACLE-4");
            }
        }
        for (index, sent) in self.sent.iter_mut().enumerate() {
            if matches!(sent.fate, Fate::Unknown | Fate::Waiting) {
                sent.fate = match found.get(&index) {
                    Some(first) => Fate::Acked(
                        (0..sent.records.len() as u64)
                            .map(|i| (first + i, records[(first + i) as usize].revision))
                            .collect(),
                    ),
                    None => Fate::Rejected,
                };
            }
        }
        // ORACLE-5: recovery is durable.
        self.drop_log();
        let seed = next(&mut self.rng);
        self.disk.crash(Crash::PowerLoss, seed);
        self.open();
        let again = self.records();
        assert_eq!(again.len(), records.len(), "ORACLE-5");
        assert!(
            again
                .iter()
                .zip(&records)
                .all(|(a, b)| a.payload == b.payload),
            "ORACLE-5"
        );
        let log = self.log.as_ref().unwrap();
        for stream in STREAMS {
            self.heads
                .insert(stream.to_vec(), log.stream_head(stream).map(|h| h.revision));
        }
        self.sealed = matches!(self.step(Task::Writer), Step::Stopped(Stop::Sealed));
        self.faults(true);
    }

    fn crash_mode(&mut self) -> Crash {
        [Crash::Kill, Crash::PowerLoss, Crash::PowerLossTorn][self.roll(3) as usize]
    }

    /// After the writer stops on its own: a fault poisoned it (ORACLE-7), or a close or a seal.
    fn stopped(&mut self, stop: Stop) {
        self.collect();
        if self.disk.state().dead {
            let mode = self.crash_mode();
            return self.verify(Some(mode));
        }
        if stop == Stop::Poisoned {
            // ORACLE-7: nothing waits, and admission refuses.
            assert!(
                self.waiting.is_empty(),
                "seed {}: ORACLE-7: a request waits after poison",
                self.seed
            );
            let calls = self.disk.state().calls;
            assert_eq!(self.step(Task::Writer), Step::Stopped(Stop::Poisoned));
            assert_eq!(
                self.disk.state().calls,
                calls,
                "ORACLE-7: a poisoned writer made a call"
            );
            let submit = Submit {
                records: vec![NewRecord {
                    stream: b"s0".to_vec(),
                    expected: ExpectedRevision::Empty,
                    key: None,
                    payload: vec![],
                }],
            };
            let log = self.log.as_ref().unwrap();
            assert!(
                matches!(
                    self.inside(|| log.append(submit)),
                    Err(AppendError::Poisoned)
                ),
                "ORACLE-7"
            );
        }
        self.verify(None);
    }

    fn run(&mut self, actions: usize) {
        self.open();
        self.faults(true);
        self.disk.state().trace.push("start".into());
        for _ in 0..actions {
            if self.sealed {
                return;
            }
            let step = match self.roll(100) {
                0..35 => {
                    self.append();
                    None
                }
                35..65 => Some(self.step(Task::Writer)),
                65..71 => Some(self.step(Task::Sealer)),
                71..75 => Some(self.step(Task::KeyFilter)),
                75..92 => {
                    let advance = self.roll(5);
                    let mut now = self.clock.now.lock().unwrap();
                    now.0 += Duration::from_millis(advance);
                    now.1 += advance;
                    None
                }
                92..97 => {
                    self.read();
                    None
                }
                97 => {
                    let mode = self.crash_mode();
                    self.verify(Some(mode));
                    None
                }
                98 => {
                    self.log.as_mut().unwrap().close();
                    None
                }
                _ => {
                    if self.roll(16) == 0 {
                        self.log.as_mut().unwrap().seal();
                    }
                    None
                }
            };
            self.collect();
            if let Some(Step::Stopped(stop)) = step {
                self.stopped(stop);
            } else if self.disk.state().dead {
                let mode = self.crash_mode();
                self.verify(Some(mode));
            }
        }
        let log = self.log.as_mut().unwrap();
        log.close();
        loop {
            if let Step::Stopped(stop) = self.step(Task::Writer) {
                return self.stopped(stop);
            }
        }
    }

    fn trace(&self) -> Vec<String> {
        self.disk.state().trace.clone()
    }
}

/// Runs one seed. On a failure, prints the seed and saves the trace.
fn run_seed(seed: u64) -> Vec<String> {
    let trace = Arc::new(Mutex::new(None::<Disk>));
    let shared = trace.clone();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
        let mut run = Run::new(seed);
        *shared.lock().unwrap() = Some(run.disk.clone());
        run.run(800);
        assert!(
            run.disk.state().hidden.is_empty(),
            "STEPPED-NO-HIDDEN-WORK: {:?}",
            run.disk.state().hidden
        );
        run.trace()
    }));
    match result {
        Ok(trace) => trace,
        Err(panic) => {
            let disk = trace.lock().unwrap_or_else(|e| e.into_inner()).take();
            let path = std::env::temp_dir().join(format!("segment-log-stepped-{seed}.trace"));
            if let Some(disk) = disk {
                let lines = disk
                    .0
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .trace
                    .join("\n");
                let _ = std::fs::write(&path, lines);
            }
            eprintln!("STEPPED-CRASH seed {seed} failed; trace {}", path.display());
            std::panic::resume_unwind(panic)
        }
    }
}

fn seeds() -> u64 {
    std::env::var("SEGMENT_LOG_STEPPED_SEEDS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(48)
}

/// Seeds that once failed, kept as regression cases.
const REGRESSIONS: &[u64] = &[];

/// Given a stepped log on a storage model of this test's own, with a manual clock
/// When one seeded generator appends, steps each task, moves the clock, reads, closes and seals,
/// while the storage injects short writes, full disks and failed flushes and crashes at random
/// calls in every crash mode
/// Then each reopen keeps every acknowledged and every read record, whole submits in order, with
/// sealed segments unchanged and a durable recovery, keyed retries of kept submits deduplicate,
/// poison answers every request, the watermark never moves back and no offset is used again
///
/// Cases: STEPPED-CRASH, ORACLE-1, ORACLE-2, ORACLE-3, ORACLE-4, ORACLE-5, ORACLE-6, ORACLE-7, ORACLE-8, ORACLE-9, ORACLE-10
#[test]
fn seeded_runs_crash_and_recover_what_they_acknowledged() {
    let mut seen = BTreeMap::<&str, usize>::new();
    for seed in (0..seeds()).chain(REGRESSIONS.iter().copied()) {
        let trace = run_seed(seed);
        for what in [
            "-> crash point",
            "FlushError",
            "Short(",
            "Full)",
            "ReadError",
            "Roll Create",
            "step Sealer -> Worked",
            "step KeyFilter -> Worked",
            "Recovery Write",
            "Stopped(Sealed)",
            "Stopped(Closed)",
            "Stopped(Poisoned)",
        ] {
            *seen.entry(what).or_default() += trace.iter().filter(|l| l.contains(what)).count();
        }
    }
    // Every fault class and every task is reached, so the oracle is not checking empty runs.
    for (what, count) in &seen {
        assert!(*count > 0, "no run reached {what}: {seen:?}");
    }
}

/// Given one seed
/// When the same seeded run is made twice
/// Then both make the same storage calls with the same results, read the clock the same way and
/// reach the same answers
///
/// Cases: STEPPED-REPLAY
#[test]
fn a_seed_replays_to_the_same_trace() {
    for seed in [3, 17, 40] {
        assert_eq!(run_seed(seed), run_seed(seed), "seed {seed}");
    }
}

/// Given a stepped log with queued submits, a gathering group and a pending closing run
/// When the test checks where storage calls and clock readings happen, and then drops the log
/// Then each one happens inside a call to the log, and the drop makes no storage call
///
/// Cases: STEPPED-NO-HIDDEN-WORK
#[test]
fn every_storage_call_and_clock_reading_happens_inside_a_call() {
    let mut run = Run::new(99);
    run.open();
    for _ in 0..40 {
        run.append();
        run.step(Task::Writer);
        run.collect();
    }
    run.append();
    run.append();
    assert!(
        run.disk.state().hidden.is_empty(),
        "{:?}",
        run.disk.state().hidden
    );
    run.drop_log();
    assert!(
        run.disk.state().hidden.is_empty(),
        "{:?}",
        run.disk.state().hidden
    );
}
