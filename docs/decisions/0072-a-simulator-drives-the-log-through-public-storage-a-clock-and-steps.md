---
id: "0072"
title: "A simulator drives the log through a public storage trait, a clock and caller-scheduled steps, behind a feature"
status: accepted
date: 2026-11-27
deciders: [Altug Sahin]
supersedes: null
superseded_by: null
tags: [testing, simulation, storage, threads, api, release-0.3]
touches: [segment-log-core/src/storage/**, segment-log-core/src/engine.rs, segment-log-core/src/engine/writer.rs, segment-log-core/src/engine/keys.rs, segment-log-core/src/engine/keys/log_filter.rs, segment-log-core/src/lib.rs, segment-log-core/Cargo.toml, segment-log-core/tests/**, docs/architecture/api.md, docs/architecture/testing.md, docs/glossary.md, .github/workflows/ci.yml, CHANGELOG.md]
enforced_by: test
---

# 0072. A simulator drives the log through a public storage trait, a clock and caller-scheduled steps, behind a feature

## Context

[Issue #206](https://github.com/ironflowx/segment-log/issues/206). `ironflowx/foundation` will test its engine under a seeded, deterministic simulator. The simulator controls the clock, the scheduler and the disk under the segment log. It injects lost writes, torn writes, failed `fsync` calls and crashes at every point, and a failing seed must replay exactly. Foundation runs 10,000 seeds per pull request and pins `segment-log-core` exactly. Its first correctness gate, three releases from now, needs 10,000 seeded fault runs to pass.

These facts are fixed:

- **A private storage seam already carries every file operation.** `segment-log-core/src/storage/mod.rs` defines a `pub(crate) trait Storage: Send + Sync`. It has one command method, `call(&self, Call) -> io::Result<Value>`, plus `lock` and `now_millis`. A `Call` is a path, a `Purpose` and an `Op`. The ops are `CreateDir`, `Create`, `Open` (returns the length), `Append`, `Write` at a position, `Read` at a position, `SyncData`, `SyncAll`, `SyncDir`, `Truncate`, `Remove`, `List` and `Random` (the log id's bytes). The purposes are `Open`, `Group`, `Recovery`, `Lookup`, `Roll`, `Summary`, `KeyFilter`, `KeyIndex`, `Seal`, `LogFilter`, `Repair` and `Close`. The log renames no file, so there is no rename op. `SegmentLog` holds its storage as `Arc<dyn Storage>`. Outside `storage/real.rs`, no code in `segment-log-core/src/engine` touches `std::fs`.
- **The in-crate simulator proves the seam covers the log.** `storage/sim.rs` (compiled under `cfg(any(test, fuzzing))`) models durable and volatile bytes per file and per directory, crashes in `Kill`, `PowerLoss` and `PowerLossTorn` modes, and injects errors, short writes, `ENOSPC`, read corruption and panics by matching a call. After a crash it fails every call still in flight from the old boot. Every acceptance scenario on the write and recovery path runs on it ([ADR 0014](0014-crash-testing-strategy.md), [ADR 0039](0039-scenario-first-acceptance-tests-in-native-rust.md)).
- **ADR 0039 rejected a public testing API** (its option C): the seam and the simulator would become published surface, and every test-only change an API decision. testing.md says the trait "and both implementations stay private to the crate".
- **The log starts threads the caller cannot see.** A writable open spawns the writer thread (`engine.rs`). Each roll spawns a sealer thread that builds and writes the closing run ([ADR 0061](0061-a-sealer-thread-builds-the-key-frames-off-the-writer.md), [ADR 0067](0067-the-sealer-writes-and-flushes-the-closing-run.md)), and a second thread that frees the sealed key map (`writer.rs`). The log key filter builder is a thread that reads key index frames through the storage ([ADR 0051](0051-a-log-key-filter-in-memory-in-front-of-the-segment-filters.md)). Open loads the sealed key filters on up to `available_parallelism` scoped threads (`keys.rs`, `SealedKeys::load_all`). There is no separate flusher: the writer calls `sync_data` itself. Recovery runs inside `open`, on the caller's thread.
- **Two of those threads already have an inline mode, for tests.** `Hooks` builds a roll's closing run on the writer (`background_seal: false`) and the log key filter on the writer (`LogFilterMode::Inline`), so that traces stay the same from run to run. With the writer still a thread, the seeded oracle cannot replay how caller threads and the writer interleave. It replays only the crash state and recovery, from a saved storage trace (testing.md, Crash model).
- **The log reads time in five places.** `append` stamps each request with `Instant::now()`. The writer computes a group's linger from that stamp with `elapsed` ([ADR 0046](0046-linger-runs-from-the-first-submits-admission.md)), notes `Instant::now()` when it first sees the linger over, and waits with `recv_timeout`. The group's append time comes from `Storage::now_millis` ([ADR 0066](0066-append-time-is-advisory-and-versions-never-go-back.md)).
- **Nothing else in the log is random.** The log id comes from `Op::Random`. The key hash is SipHash keyed by the log id. The active key map hashes with a `RandomState`, but it iterates in insertion order. The frame tables keep a `HashMap` with a `RandomState`, which is never iterated, and evict by a `BTreeMap` of last use. So both states change only memory layout, never an answer, a byte or a call order.
- **Two more waits block.** `seal` sends its marker with a blocking `send` into the bounded accept queue, which can wait for queue room. A poisoned writer answers queued requests with `for message in receiver`, which blocks until every sender is gone.
- **`Pending::try_wait` exists** and returns without blocking (api.md). `Pending::wait` parks the calling thread.
- **The real path is measured.** The `m1-compare` capture in `.github/workflows/evidence.yml` builds two commits and runs five alternating pairs of the closed-loop and the paced benchmark on one host (`scripts/m1-compare-refs.sh`). `m1-evidence` checks one `fdatasync` per group (`scripts/m1-verify-syscalls.py`), and `m3-evidence` checks the roll's syscall order.
- **[ADR 0064](0064-the-0-1-compatibility-promise.md), rule 15:** API breaks go only in minor releases, with a migration note; patch releases never break. [ADR 0012](0012-no-log-crate-dependency.md): the core uses only `std`.

The async acknowledgment for tokio, the watermark notification and the Windows, iOS and Android ports are out of scope. Each gets its own ADR.

## Options & trade-offs

### The storage seam

#### Option A - publish the existing command seam

Make `Storage`, `Lock`, `Call`, `Op`, `Purpose` and `Value` public, with the enums `#[non_exhaustive]`.

- **For:** no call site changes, and the real path keeps its `dyn` call. The in-crate simulator already shows that the seam carries every operation the log performs, including recovery, the repair tool and the log key filter build. One method makes recording, replaying and matching a call trivial, which is what a simulator does with it. The purpose lets a simulator aim a fault at "the next group's flush" without knowing file positions, as the scenarios do today.
- **Against:** `Purpose` names internal work, so a refactor can add a purpose. A new `Op` compiles for implementers because of `#[non_exhaustive]`, but an implementer that does not handle it fails that call. Implementers must keep the per-op contract (what `Read` returns at the end of a file, what `Open` returns), which a typed method would show.

#### Option B - a method per operation with file handles, like a VFS

- **For:** familiar. Each method's signature states its contract.
- **Against:** rewrites every I/O call site and the in-crate simulator, and adds handle state that a simulator must track. Recording a trace needs one wrapper per method. It changes the real path for no gain the simulator needs.

#### Option C - publish the in-crate simulator instead of a trait

- **For:** the least code for foundation, and a crash model that the `SIM-MODEL` and `SIM-CONFORMANCE` cases already check.
- **Against:** foundation needs one disk model shared by its whole engine, driven by its own seed. The in-crate simulator's fault API is shaped for scenarios: it holds calls with channels and five-second timeouts. Publishing it is ADR 0039's objection in full: every test-only change becomes an API decision.

### Stepping

#### Option D - keep the threads, publish the seam and the clock, replay from storage traces

This is the cheapest option: no change to the writer.

- **For:** no refactor and no new type. ADR 0039 already replays crash states from traces.
- **Against:** a seed cannot fix how caller threads, the writer, the sealer and the builder interleave, so a failing seed does not replay. Foundation's whole engine is under one seed; a trace of the log's storage calls does not replay foundation's own state. It fails the issue's first requirement.

#### Option E - the caller supplies the threads

The log asks a caller's spawner for every thread, and the simulator schedules them.

- **For:** the log's code keeps its shape.
- **Against:** the threads block in `mpsc` receives, `recv_timeout`, `join` and `park`. A simulator controls none of those unless the log also takes its channels, locks and parking from the caller. That is an async runtime in all but name, which the write path rules out.

#### Option F - a stepped log: the caller runs the writer, the sealer and the builder as tasks

The writer's loop becomes a state machine that does one step at a time. A stepped log starts no thread. The caller calls `step(task)` for the writer, the sealer or the key filter builder. The threaded log runs the same writer steps on its own thread.

- **For:** one thread, so one seed fixes one order. The sealer and the builder are separate tasks, so a seed explores the orders that production allows, such as a closing run written before, between or after later groups (ADR 0067 rule 6). Production and simulation run the same writer code.
- **Against:** the writer loop's state (the draft group, the deferred request, the linger mark) moves into a struct. That refactor touches the hot path, and it must be measured. A stepped log is a second public type with its own rules for seal and drop.

#### Option G - a stepped log with everything inline in the writer task

As F, but the sealer and the builder run inside writer steps, as the inline test modes do today.

- **For:** less code than F: one task, and the inline modes exist.
- **Against:** the closing run is always written at the first group boundary after the roll, so a seed never explores the orders production allows. It costs F's writer refactor and saves only two small task wrappers.

### Time

#### Option H - keep time on the storage trait, and add a monotonic reading

- **For:** one object to pass.
- **Against:** `append` reads time on the caller's thread, which is not a storage operation. A simulator's disk and clock are separate components.

#### Option I - a separate `Clock` trait

- **For:** each trait is about one thing. The threaded log takes the system clock, and nothing else changes for it.
- **Against:** one more object to pass. The clock reading at admission becomes a `dyn` call.

### Stability

#### Option J - public in `segment-log-core`, with no feature

- **For:** nothing to enable.
- **Against:** every user's documentation and public surface grows with a testing API that only a simulator needs.

#### Option K - public behind a `simulation` feature, under rule 15

- **For:** the default build's public surface does not change. Breaks still go only in minor releases, so an exact pin and a caret requirement both stay safe.
- **Against:** Cargo unifies features, so any crate in a build that enables it shows the API to all. Two feature sets to check in CI.

#### Option L - a separate crate

- **Against:** the stepped log needs the writer's internals, so the core must make them public anyway. Foundation would pin two crates in lockstep.

#### Option M - behind a feature, and exempt from semver

- **Against:** Cargo would hand a break in a patch release to anyone who enables the feature with a caret requirement. Rule 15 says patches never break.

## Decision

We will build options A, F, I and K.

**The storage seam.** `Storage`, `Lock`, `Call`, `Op`, `Purpose` and `Value` become public as they are today, with three changes: `now_millis` moves to the clock, `Op`, `Purpose` and `Value` are `#[non_exhaustive]`, and api.md states each op's contract. That contract: `Create` and `CreateDir` fail with `AlreadyExists` if the path exists, which open and the repair tool rely on; `Open` returns `Value::Length`; `Read` returns `Value::Bytes` of at most the asked length, short only at the end of the file; `List` returns the file names sorted; `Random` returns that many bytes; every other op returns `Value::Unit`; `lock` fails with `WouldBlock` when the lock is held, and dropping the guard releases it. A wrong `Value` is an I/O error. The log never retries a failed `SyncData`, `SyncAll` or `SyncDir` ([ADR 0004](0004-io-errors-poison-the-writer.md)), so a storage may model writeback loss after one. The real filesystem implementation stays private and is what `SegmentLog::open` uses. The in-crate simulator stays private and test-only.

A fault is the result a storage returns: an error, a short write (write some bytes, then return an error), `StorageFull`, or bytes that differ from what was written. A crash point is a call that the storage fails, after which it fails every call of that boot, as the in-crate simulator does today. The caller then drops the log. The storage owns its crash model: which unsynced bytes and directory entries survive, and which sectors tear. The log's promises (I1–I10) hold for faults within the [storage contract](../architecture/overview.md#storage-contract). A storage that breaks it, for example a `sync_data` that returns success and keeps nothing, gets only the log's CRC checks.

**Stepping.** A new type, `SteppedLog`, opens a log with a caller's `Arc<dyn Storage>` and `Arc<dyn Clock>` and starts no thread. It has `append`, `read`, `watermark` and `stream_head` with the same meaning as on `SegmentLog`, and adds:

- `step(task) -> Step`, where `Task` is `Writer`, `Sealer` or `KeyFilter`.
- `seal()`, which closes admission and marks the log for sealing. It returns at once. The mark skips the queue's bound: the writer acts on it once the queue is empty, so every admitted submit still comes before the seal, as with `SegmentLog::seal`. Later writer steps do the work.
- `close()`, which closes admission as a drop of `SegmentLog` does. Later writer steps drain the queue, write a pending closing run and write the close frame ([ADR 0071](0071-a-clean-drop-writes-a-close-frame.md)).

`Step` is `Worked`, `Idle { until }` or `Stopped(Closed | Sealed | Poisoned)`. `Idle` says that the task has nothing to do until the caller appends, closes or seals, or, for the writer, until the clock reaches `until`, the end of the gathering group's linger. After `Stopped`, every step of every task returns the same value.

Each task does what its thread does today, cut where that thread would block:

1. **Writer.** A step takes queued requests and classifies them as the writer loop does, and returns `Idle` where the thread waits in `recv` or `recv_timeout`. Otherwise it returns after it has written and flushed one group, rolled, sealed, closed or poisoned. A roll ends a step, so a sealer step can run between a roll and the group that follows it, as the sealer thread can. A step that poisons the writer answers the requests still queued with `Poisoned` from the queue's `try_iter`, where the thread waits for every sender to go. Where the thread would wait for the sealer (`write_pending` with `wait`: a roll with a run still pending, `seal` and `close`), the step runs the sealer's job itself. That is the order the threaded writer produces, because it blocks until the sealer is done.
2. **Sealer.** A step builds one pending closing run and writes and flushes it (ADR 0067). The writer finds it durable at its next group boundary, as it checks a finished sealer thread today. If the step fails, it keeps the error, and the writer poisons itself at its next group boundary (ADR 0067, rule 4). A caller that never schedules the sealer gets the run written where the writer would wait for it.
3. **Key filter.** A step catches the log key filter up over the segments it has been told about (ADR 0051, rule 2). The writer takes the new filter at a group boundary, as today. A failure stops the builder and does not poison the writer (ADR 0051, rule 5).

The sealed key map is freed inline, because that thread does no I/O. Open loads the sealed key filters on the caller's thread, in segment order. Recovery is not a task: it runs inside `open`, and its storage calls are crash points like any other.

**The threaded log runs the same writer.** The writer loop becomes the state machine above. `SegmentLog`'s writer thread loops over writer steps and blocks where the step returns `Idle`: in `recv` with no `until`, in `recv_timeout` until `until`. The sealer, the key filter builder and open's parallel load stay threads in `SegmentLog`. `SegmentLog` keeps its API, and it takes no caller storage or clock.

**Time.** A public `Clock: Send + Sync` trait gives two readings: `monotonic() -> Duration`, from an origin the clock chooses, which never decreases, and `wall_millis() -> u64`, milliseconds since the Unix epoch, which may step back. `append` stamps each request with `monotonic()`. The stamp is a `Duration`, the same 16 bytes as the `Instant` it replaces on the supported targets, so the admission charge does not change. The writer computes the linger and its "linger over" mark from it ([ADR 0046](0046-linger-runs-from-the-first-submits-admission.md)). A group's append time is `wall_millis()`, raised to the previous group's time as today ([ADR 0066](0066-append-time-is-advisory-and-versions-never-go-back.md)). `SegmentLog` uses a private system clock: `Instant` since open, and `SystemTime`. No other code in the core reads time, except under `cfg(test)`.

**One seed fixes one order.** A stepped log's behavior is a function of three things: the sequence of public calls (`open`, `append`, `step`, `seal`, `close`, `read`, `try_wait`, drop), the results its storage returns, and its clock's readings. It adds no randomness and no concurrency of its own. So a simulator that draws all three from one seed replays a run exactly. A caller of a stepped log polls answers with `Pending::try_wait`. `Pending::wait` on an unanswered request blocks forever, because no thread will answer it. A storage or clock must not call back into the log.

**Crash and reopen.** Dropping a `SteppedLog` makes no storage call and reads no clock: it is a crash. It drops the storage's lock guard, whose own code releases the lock. It writes no close frame. Unanswered `Pending`s then return `Indeterminate`, as they do when a writer stops. A clean drop is `close()` followed by writer steps until `Stopped(Closed)`. After a crash, the simulator builds its crash image and calls `SteppedLog::open` on the same storage. Open runs recovery as `SegmentLog::open` does, and `read`, `watermark`, `stream_head` and keyed retries then show what recovery kept. A storage panic unwinds out of the step and poisons the log, as it poisons the writer thread today. After that the log is only fit to drop.

**The cost to the real path.** The storage call is already `dyn` on the real path, so publishing the trait changes no call. The clock adds one `dyn` call at each admission and at each linger check, beside the `Instant::now()` it wraps, and `now_millis` moves from one `dyn` call to another. The writer refactor keeps every storage call, in the same order, on the threaded path. The implementation pull request proves it with the existing suites, unchanged:

- `cargo test --workspace` passes with no edit to an existing scenario's assertions, including the threaded seeded oracle, `tests/real_storage.rs` and `tests/process_crash.rs`. The CI run with a small `SEGMENT_LOG_TEST_ROLL_BYTES` passes too.
- An `m1-compare` capture of `main` against the head, on one host. For closed-loop records/s and for paced p50 and p99, the head's median lies within the range of the base's five runs. The head has no fewer paced runs within the [ADR 0045](0045-latency-target-is-measured-at-a-paced-rate.md) bound than the base. That range is this ADR's measure of noise.
- `m1-evidence`'s syscall check (one `fdatasync` per group) and `m3-evidence`'s roll syscall check pass.
- Open is not measured again: `SegmentLog`'s open keeps its parallel load, and its path does not change.
- A record in `docs/evidence/issue206/` states the numbers and the verdict, with a row in the evidence README.

**Stability.** The API lives in `segment-log-core`, in a module `segment_log_core::simulation`, behind a `simulation` feature that is off by default and adds no dependency. The writer's state machine is shared code and is always compiled; the feature gates only the public items: `Storage` and its types, `Clock`, `SteppedLog`, `Task` and `Step`. Rule 15 of ADR 0064 covers the module like the rest of the API: breaks go only in minor releases, with a migration note in the changelog. A new `Op` variant compiles for implementers but changes what they must handle, so it also goes only in a minor release, and the changelog names it as an API change. `Purpose` may gain variants in any minor release; a storage should treat an unknown purpose like any other. It first ships in 0.3.0, not a 0.2.x patch, because the writer refactor reaches every user and Cargo takes patch releases without asking. Foundation pins `=0.3.0`.

We chose a stepped log over the cheaper trace replay of option D because only one thread lets one seed fix one order, which is what foundation's gate needs. We chose separate sealer and builder tasks over option G because they cost little and let a seed explore the orders production allows. We accept a refactor of the writer loop on the hot path, proved by measurement, and a public testing surface, which ADR 0039 rejected, kept behind a feature and under the same semver rule as the rest of the API.

## Consequences

- Accepted by the owner on merge of its pull request, for 0.3.0 ([issue #206](https://github.com/ironflowx/segment-log/issues/206)).
- Amends ADR 0039: the storage seam, a clock and a stepped log become public behind the `simulation` feature. The simulator stays private and test-only, and scenarios still reach it through the private constructor. testing.md's "Storage trait" section changes to match.
- Amends ADR 0046: a request's admission stamp and the linger read the log's clock, not `Instant` directly.
- Amends ADR 0051, ADR 0061 and ADR 0067: in a stepped log, the key filter builder and the sealer are tasks that the caller schedules, not threads. Where ADR 0061 rules 4 and 7 make the writer wait for the sealer, the writer's step runs the sealer's job. In `SegmentLog` they stay threads, and the in-crate inline test modes stay.
- Amends ADR 0071: dropping a `SteppedLog` is a crash and writes no close frame. Only `close()` writes one.
- api.md gains the `simulation` module's signatures and each op's contract. testing.md gains the scenarios below as `Planned`. The glossary gains *stepped log*, *step*, *task* and *crash point*. The changelog's 0.3.0 section states the feature and that `SegmentLog`'s API is unchanged. No format change and no protocol change.
- CI runs the workspace tests, and clippy, a second time with `--features segment-log-core/simulation`.
- A simulator writes its own crash model against the trait. The in-crate simulator's model is the reference, but it is not public.
- A stepped log interleaves other actors only between steps. Within one step, no append or read runs. Readers see only the published watermark, which moves only after a group's flush, so this hides no state a reader could see. The threaded seeded oracle keeps covering real thread interleavings.
- `LogReader`, the repair tool and the server take no caller storage. Each can gain it later without a break.
- Each new piece of background work, each new time read and each new file operation must now fit the seam: it runs as a step of a task, reads the clock, or is an `Op`. A reviewer checks this as I1–I10 are checked.

## Enforcement

Scenarios in [testing.md](../architecture/testing.md). The first three are integration tests in `segment-log-core/tests/simulation.rs`, built only with the `simulation` feature (`required-features`). They use only public items and a storage of their own that models durable and volatile bytes per file and directory entries, with page-level loss and torn sectors. So they also prove that a crate outside the core can implement the trait.

- `STEPPED-CRASH`: a seeded run. One generator chooses each action: an append (keyed or not, with expected revisions), a step of a task, a clock advance, a read, a seal or a close. Its storage injects each fault class: lost unsynced writes, torn writes, failed `fsync`s with writeback loss, short writes and `StorageFull`. It crashes at a random storage call in `Kill`, `PowerLoss` and `PowerLossTorn` modes, reopens with `SteppedLog::open` on the crash image, and checks oracle checks 1–7, 9 and 10. Rolls and the closing runs of both orders are reached with a small `roll_bytes`. A failure prints its seed and saves its storage trace, as CLAUDE.md requires.
- `STEPPED-REPLAY`: the same seed run twice gives the same storage call sequence, the same clock readings and the same answers. A saved failing seed is kept as a regression case and replays to the same result.
- `STEPPED-NO-HIDDEN-WORK`: every storage call and clock reading happens inside a public call the test made. The test storage fails the test if a call arrives between them. Dropping a stepped log makes no storage call; it only drops the lock guard.
- `STEPPED-ORDERS` (in-crate, on the simulator): after a roll, a sealer step before the next group, a sealer step after it, and no sealer step (the writer runs the job at the next roll, seal or close) give three orders. A crash at every storage call of each recovers, with oracle checks 1–6. A seal with a full queue returns at once, and the seal follows every queued submit.
- The real path: the existing suites above, unchanged, and the `issue206` evidence record.
- Stability: the `semver` CI job (`cargo-semver-checks`) checks the public API with the `simulation` feature on, from the first release tag that has it, so a patch release cannot break the module.

## Revisit if

- Foundation needs an append or a read inside a step, for example between a group's write and its flush. Then consider finer steps or a storage that may call back.
- Foundation needs the threaded `SegmentLog` on simulated storage, to test its own threads against ours.
- The `m1-compare` capture shows the refactor slows the real path beyond the base's range. Then the threaded writer keeps its own loop, and a shared-code test must prove the two loops agree.
- An out-of-scope ADR (the tokio acknowledgment, the watermark notification, a platform port) changes the seam or adds background work.
