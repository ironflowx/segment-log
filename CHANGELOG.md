# Changelog

Each release lists its API breaks with a migration note, its format version bumps and the protocol versions it speaks ([ADR 0064](docs/decisions/0064-the-0-1-compatibility-promise.md)). The crates and the Go client (`clients/go/vX.Y.Z`) share one version.

## 0.3.0 - 2026-11-27

- **Speaks protocol versions:** 2.

### Added

- **A `simulation` feature for deterministic simulation** ([issue 206](https://github.com/ironflowx/segment-log/issues/206), [ADR 0072](docs/decisions/0072-a-simulator-drives-the-log-through-public-storage-a-clock-and-steps.md), [api.md](docs/architecture/api.md#simulation)). Off by default, it adds `segment_log_core::simulation`: the `Storage` trait that every file operation goes through, a `Clock` trait for every time the log reads, and `SteppedLog`, a log that starts no thread. Its writer, sealer and key filter builder run only when the caller calls `step(Task)`, so one seed that fixes the calls, the storage's results and the clock's readings fixes a whole run. Dropping a `SteppedLog` is a crash, and `SteppedLog::open` on the same storage recovers. The feature adds items only: with it on or off, `SegmentLog` behaves the same. A new `Op` variant will come only in a minor release and be named here, because a storage must handle it.

### Changed

- **`SegmentLog`'s writer thread runs the same steps as `SteppedLog`.** The writer loop is now a state machine that ends each step where the thread would wait; the thread loops over its steps. The storage calls and their order are unchanged ([record](docs/evidence/issue206/README.md)).
- **A segment that outgrows its key index no longer moves every key at once** ([issue 200](https://github.com/ironflowx/segment-log/issues/200)). The active key map's index is 64 small maps instead of one, so an insert that grows one moves about 1/64 of the keys on the writer. An index can take up to about 2 MiB more in a full segment. On a slowed EPYC 7502P, the longest pause when a segment outgrows its index falls from 18 to 1.3 ms ([record](docs/evidence/issue200/README.md)). No API or format change.

### Fixed

- **Open now makes a rolled segment's closing run durable when it finds it only in the page cache** ([ADR 0073](docs/decisions/0073-open-makes-the-second-newest-segments-closing-run-durable.md)). If the process died after the sealer wrote a closing run's footer and locator but before their flush, or that flush failed, the next open read them from the page cache, trusted them and never flushed them. Once the log rolled past that segment, a power loss could drop them, and every later open failed with `Corrupt`. Open now rewrites the second-newest segment's footer and locator in place and calls `sync_all`, as it already rewrites the newest segment's tail ([ADR 0015](docs/decisions/0015-open-makes-the-recovered-tail-durable.md)). That costs one file open, one small read, one small write and one flush at each writable open of a log with more than one segment. A log that an older build already rolled past such a run is not repaired by this open; if power is lost, run the repair tool. No API or format change.

No API break. No format change. The protocol versions are unchanged.

## 0.2.1 - 2026-11-23

- **Speaks protocol versions:** 2.

### Changed

- **A segment's keys share one buffer in its key map** ([issue 154](https://github.com/ironflowx/segment-log/issues/154)). The active key map kept one heap buffer per key, about 226,000 in a full segment of keyed 256-byte records, and freeing them after a roll stalled the writer's own allocations in glibc malloc. Freeing the map is now a few frees. On an EPYC 7502P slowed until the old code misses the paced p99 bound (12.7 ms against 2.1 ms), p99 is now 1.5 ms. Not measured on the 7401P. No API or format change.
- **The first segment after open sizes its key map** for as many keys as the newest sealed segment, as a roll already does (#168), so it does not grow the map on the writer until it holds more keys than that ([issue 154](https://github.com/ironflowx/segment-log/issues/154), #194). The size is capped by the keys this open's `roll_bytes` can hold, so reopening with a smaller roll size does not reserve memory for the old segment's keys. A log with no sealed segment still starts from an empty map, and a segment that outgrows its map still grows it on the writer ([issue 200](https://github.com/ironflowx/segment-log/issues/200)). No API or format change.

## 0.2.0 - 2026-11-20

- **Speaks protocol versions:** 2.

### Format

- **Format version 4** adds the close frame (kind 7) and changes nothing else ([ADR 0071](docs/decisions/0071-a-clean-drop-writes-a-close-frame.md), [format.md](docs/architecture/format.md)). This build reads versions 3 and 4 and writes 4. Its first writable open of a 0.1 log closes the active version 3 segment and rolls into a version 4 segment, or rewrites a header-only one (ADR 0064, rule 12). **The upgrade is one-way:** a 0.1 build refuses the log with `UnsupportedVersion(4)` once this build has opened it for writing, so roll back only before that. `LogReader` changes no file.

### Changed

- **Random reads over many sealed segments no longer reread frame headers** on nearly every call ([issue 177](https://github.com/ironflowx/segment-log/issues/177), [ADR 0069](docs/decisions/0069-reads-keep-sparse-frame-tables-under-a-byte-budget.md)). A log, or a `LogReader`, used to keep the frame tables of at most 16 sealed segments. It now keeps a sparse table per sealed segment, one frame for each 64 KiB, under a 24 MiB budget per view, about 64 GiB of sealed history ([api.md](docs/architecture/api.md#opening-a-directory)). A read of a frame that is not kept reads one window of at most 64 KiB. Header damage found in a sealed segment after its first walk now fails reads of that whole window, not only of the damaged frame.

### Fixed

- **Open no longer drops an acknowledged last group after a clean drop** when its bytes are damaged and nothing follows it ([issue 183](https://github.com/ironflowx/segment-log/issues/183), [ADR 0071](docs/decisions/0071-a-clean-drop-writes-a-close-frame.md)). A clean drop now appends a close frame after its last group and syncs it; it writes none when the writer is poisoned, when a pending closing run failed, or when the segment already ends with one. Open counts a close frame past the damage as proof that the damaged group was acknowledged, and fails with `OpenError::Corrupt { segment, position }` instead of truncating it. A crash, unlike a clean drop, writes no close frame, so damage to the last group before the next group or clean drop is still truncated. No API change.
- **`segment-log-server repair`:** a class F plan whose only proof is a close frame first copies it past the seal run, and then seals, instead of refusing when the run does not fit. The printed plan names the copy; the JSON is unchanged.
- **Go client: a vsock `Dial` no longer fails with `interrupted system call`** when a signal reaches the dialing thread during the connect, as the Go runtime's own `SIGURG` can under a burst of dials ([issue 187](https://github.com/ironflowx/segment-log/issues/187)). The connect is now non-blocking and waits on Go's poller, as `net.Dial` does.

No API change. Format version 4 (one-way upgrade). The protocol versions are unchanged.

## 0.1.1 - 2026-11-20

- **Go client:** `Pool` (`DialPool(network, address, n)`) shares *n* pipelined connections among goroutines. Requests queued on a connection while a write is in progress go out together in the next write, and each answer returns to its caller in request order. A connection holds no more requests in flight than the server reads ahead ([protocol.md](docs/architecture/protocol.md), step 3); a request beyond that waits for an answer. When a connection fails, each request written to it and not answered gets `ErrUnknown` and is never retried by the pool; requests not yet written get `ErrNotSent`. The next request for that connection dials a new one, and gets `ErrNotSent` if that fails. After `Close`, requests get `ErrPoolClosed`. The pool refuses `Hello` ([issue 134](https://github.com/ironflowx/segment-log/issues/134), [ADR 0063](docs/decisions/0063-the-guest-benchmark-shares-pipelined-connections.md)). Additive; no wire change.
- **Speaks protocol versions:** 2.

### Fixed

- **Open no longer drops acknowledged records** when a bad frame in the newest unsealed segment has valid group frames of the log after it ([issue 178](https://github.com/ironflowx/segment-log/issues/178), [ADR 0068](docs/decisions/0068-open-refuses-damage-with-acknowledged-groups-after-it.md)). Open used to truncate at the bad frame and return `Ok`, so the next writer reused those offsets and a retry of a dropped key ran twice. It now fails with `OpenError::Corrupt { segment, position }` at the bad frame and changes no file. A torn tail is still truncated. The same damage in a second-newest segment without its closing run now fails with `Corrupt` at the bad frame instead of `Gap` at the next segment, and `LogReader` on such a log returns `Corrupt` instead of `NotSealed`. No API change.
- **`segment-log-server repair` gains class F** for that case. `truncate` plans it as lossy: `--apply --drop-acknowledged` writes a seal run at the bad frame and leaves the log sealed, and refuses when the run would reach the last acknowledged group frame, or while the second-newest segment lacks its closing run. Class F in a second-newest segment is diagnosed only. The JSON `class` field can now be `"F"`; `schema` stays 1.

No format version change. The protocol versions are unchanged.

## 0.1.0 - 2026-11-18

The first production release. From this release, every later build reads every format version from 3 and speaks every protocol version from 2; no 0.x release removes either. API breaks go only in minor releases.

- **Format version 3**, the baseline. Each group carries a u64 append time that never decreases within a log. Logs written before 0.1 (format versions 0, 1 and 2) are refused with `UnsupportedVersion`.
- **Protocol version 2**, the baseline. `Hello` negotiates a version range. Clients written before 0.1 (protocol version 1) are refused with `UnsupportedVersion`.
- **Speaks protocol versions:** 2.
- **License:** MIT, for the crates and the Go client. The `0.0.1` placeholder crates on crates.io stay Apache-2.0.
- **`segment-log-core`:** append with group commit (one `fsync` per group), per-stream revisions and expected-revision checks, idempotency keys, roll, seal, keyed lookup through each sealed segment's key filter and index, `LogReader` for sealed history, and crash recovery ([api.md](docs/architecture/api.md)).
- **`segment-log-server`:** one process serves one log over a Unix socket or vsock; `--read-only` serves a sealed log; `repair verify|truncate` for a log that opens as `Corrupt` ([server README](segment-log-server/README.md), [ADR 0065](docs/decisions/0065-the-repair-tool-truncates-only-the-newest-segment.md)).
- **Writer tail latency** ([issue 154](https://github.com/ironflowx/segment-log/issues/154)): the active key map is a hash map, sized at each roll so it never grows on the writer (#163, #168); the log filter is built off the writer (#164); the sealer thread writes and flushes a rolled segment's closing run, so appends no longer wait for it ([ADR 0067](docs/decisions/0067-the-sealer-writes-and-flushes-the-closing-run.md), #170). Measurements are in the [issue 154 record](docs/evidence/issue154/README.md).
- **Go client** `github.com/ironflowx/segment-log/clients/go`: appends and reads over either transport.
- **Qualified stack:** Firecracker v1.18.0 on a dedicated local-NVMe host passes every MUST volume requirement ([qualification record](docs/evidence/m7/qualification.md)). Any other stack needs its own qualification.

### Known limits

- In a Firecracker guest, append throughput reaches 52,860 records/s, but the paced p99 at 50,000 records/s is 69 ms, above the latency target ([ADR 0063](docs/decisions/0063-the-guest-benchmark-shares-pipelined-connections.md)).
- On an AMD EPYC 7401P host, the in-process paced p99 at 50,000 records/s was 29 ms against a 1.94 ms bound, measured before the writer tail-latency fixes above. On an EPYC 7502P after #168, but before ADR 0067, it was 4.1 ms against a 1.8 ms bound. With ADR 0067, measured on 2026-11-23 on another EPYC 7401P, it is 13.5 ms against a 2.2 ms bound. The cause is fixed in 0.2.1 ([issue 154](https://github.com/ironflowx/segment-log/issues/154)).
- A Go `Client` is one connection that goroutines cannot share; there is no pooled client yet ([issue 134](https://github.com/ironflowx/segment-log/issues/134)).
- No replication, retention or compaction ([PRD non-goals](docs/prd.md#non-goals)).
