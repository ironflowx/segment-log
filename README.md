# segment-log

[![CI](https://github.com/ironflowx/segment-log/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/ironflowx/segment-log/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/ironflowx/segment-log)](https://github.com/ironflowx/segment-log/releases/latest)
[![crates.io](https://img.shields.io/crates/v/segment-log-core.svg)](https://crates.io/crates/segment-log-core)

An embedded, append-only event log in Rust. Many callers append records to many streams in one directory. One writer thread packs their submits into groups, and one `fsync` makes a whole group durable before any caller gets an acknowledgment. Each stream keeps revisions and checks the caller's expected revision. An idempotency key makes a retry safe after a crash. When the work ends, the log seals, and the sealed segments stay on disk as the history. A Rust program links the library; other languages reach the same library through `segment-log-server` over a Unix socket or vsock.

> **Status: 0.2.1** ([changelog](CHANGELOG.md)). 0.1 is the first production release. Milestones M0–M7 have passed ([M7 record](docs/evidence/m7/completion.md)). The [compatibility promise](docs/decisions/0064-the-0-1-compatibility-promise.md) applies: every later release reads 0.1 logs and serves 0.1 clients.

![Without segment-log, every agent flushes its own file and the disk does one flush per agent. With segment-log, the same agents put their appends in the accept queue, one writer thread packs them into one group, and one flush covers the group. Acks go out only after that fsync returns.](docs/images/one-fsync-per-group.svg)

## Use cases

- **Agent runs in short-lived guests.** Each run appends its history from many threads and seals on exit. Another system reads the sealed history after the guest is gone ([handoff record](docs/evidence/m7/handoff.md)).
- **Many writers, one disk.** Many callers share one `fsync` per group, so flush cost does not grow with the caller count.
- **Optimistic concurrency per entity.** A stream per entity or run, with an expected revision on each append.
- **Safe retries across crashes.** A caller that lost its answer, and gave every record a key, retries with the same keys and gets the original log offsets back.

It is not a broker. It has no replication, consumer groups, retention or compaction ([PRD non-goals](docs/prd.md#non-goals)).

![A Rust program calls append in its own process. A Go program sends requests over a Unix socket, or vsock from a guest, to segment-log-server, one process per log, which runs the same core. Both ways lead to the accept queue. One writer thread packs appends into a group and one fsync makes it durable in the segment files, on a volume that outlives the guest. After the guest exits, LogReader reads the sealed history.](docs/images/two-ways-in-one-writer.svg)

## Benchmarks

The targets and their conditions are in the [PRD](docs/prd.md#footprint-and-performance-targets).

| Measure | Result | Evidence |
| --- | --- | --- |
| Append throughput, in process, bare-metal local NVMe | 62,203 records/s (closed-loop median) | [issue 49](docs/evidence/issue49/README.md) |
| Open time, keyed, warm page cache, 100 sealed segments | 38.6 ms (p50) | [M4](docs/evidence/m4/completion.md) |
| Server binary, stripped musl | 1,049,384 bytes | [M6](docs/evidence/m6/completion.md) |
| Server idle resident set, empty log | 1,240 KiB | [M6](docs/evidence/m6/completion.md) |
| Guest append throughput, Firecracker over hybrid vsock | 52,860 records/s (target ≥ 50,000), 1,000 callers over 16 connections | [guest benchmark](docs/evidence/m7/qualification.md#guest-benchmark-and-49) |
| Guest append latency p99, paced at the target rate | 69.2 ms; misses the target, a known 0.1 limit ([ADR 0063](docs/decisions/0063-the-guest-benchmark-shares-pipelined-connections.md)) | [guest benchmark](docs/evidence/m7/qualification.md#guest-benchmark-and-49) |
| Soak, in process, 60 million keyed records across 265 segments | 127,914 records/s, reopen in 219 ms, 0 correctness errors | [issue 47](docs/evidence/issue47/README.md) |
| Sequential scan, one reader, 64-record calls, 43 sealed segments | 191 µs p50 (392,102 records/s) | [issue 177](docs/evidence/issue177/README.md#reads-one-reader-64-records-per-call) |
| Random read, one reader, 64-record calls, 43 sealed segments | 209 µs p50 | [issue 177](docs/evidence/issue177/README.md#reads-one-reader-64-records-per-call) |
| Storage qualification of the guest stack | Qualified: every MUST requirement passes on one host, including 30 host power losses | [qualification record](docs/evidence/m7/qualification.md) |

The issue 49 host's paced p99 missed its bound because sealing at roll stalled the writer ([issue 107](https://github.com/ironflowx/segment-log/issues/107)). [ADR 0061](docs/decisions/0061-a-sealer-thread-builds-the-key-frames-off-the-writer.md) moved that work to a sealer thread. On the M7 requalification host (EPYC 7401P) every roll kept the writer for 0.42–0.80 ms, and paced p99 was 29.1 ms against a 1.94 ms bound, 0 of 5 runs ([qualification record](docs/evidence/m7/qualification.md#guest-benchmark-and-49)). The miss was the free of the rolled segment's key map on another thread, which stalled the writer in glibc malloc ([issue 154](https://github.com/ironflowx/segment-log/issues/154)). [#198](https://github.com/ironflowx/segment-log/pull/198) keeps a segment's keys in one buffer, so that free is a few frees. On an EPYC 7502P slowed to 24 cores at 1.5 GHz, where the code before the fix misses the bound (p99 12.7 ms against 2.13 ms, 0 of 5 runs), the fix meets it (p99 1.50 ms against 2.12 ms, 5 of 5) ([issue 154 record](docs/evidence/issue154/README.md#the-fix-where-the-base-misses-the-bound)). The 7401P itself has not run with the fix.

Reader numbers are from the [issue 177 record](docs/evidence/issue177/README.md), after [ADR 0069](docs/decisions/0069-reads-keep-sparse-frame-tables-under-a-byte-budget.md) replaced the 16 frame tables with sparse tables. Measured on 2026-11-20 on an Apple M4 Mac mini (10 cores, 24 GiB, Darwin 27.2.0, APFS on an external Samsung PSSD P7, firmware SXGD0PCQ, PCI-Express), rustc 1.100.0, release build. One reader through `LogReader` read a sealed log of 10,000,000 keyed 256-byte records (43 segments) for 10 s, 64 records per call, 3 runs. A scan took 191 µs p50 (392,102 records/s). A random read took 209 µs p50. On the same host, before the change, that random read took 1,191 µs p50. The [issue 156 record](docs/evidence/issue156/README.md#frame-tables) measured 1,466 µs p50 for a random read of 43 sealed segments on the Mac's internal SSD, when a log kept 16 frame tables. A Mac on APFS is not the deployment stack. On an EPYC 7502P with Samsung PM9A3 drives, every log resident, the 43-segment log took 301 µs p50 for both a random read and a scan (199,791 records/s) ([Linux rerun](docs/evidence/issue177/README.md#linux-rerun)). No EPYC 7401P was on offer, so that rerun is a confirmation on similar hardware, not on the qualified host.

## Quickstart

```sh
cargo add segment-log-core                  # the library
cargo install segment-log-server            # the server; or the Linux musl binary from the GitHub release
go get github.com/ironflowx/segment-log/clients/go@v0.2.1   # the Go client
```

```rust
use segment_log_core::{ExpectedRevision, NewRecord, Options, SegmentLog, Submit};

fn main() {
    let log = SegmentLog::open("data/log", Options::default()).expect("open");
    let committed = log
        .append(Submit {
            records: vec![NewRecord {
                stream: b"order-42".to_vec(),
                key: Some(b"order-42/created".to_vec()), // a retry with this key is safe
                payload: b"{\"total\":10}".to_vec(),
                expected: ExpectedRevision::Empty, // the stream must not exist yet
            }],
        })
        .expect("admitted")
        .wait() // returns after the group's fsync
        .expect("durable");
    println!("offset {}", committed.records[0].offset);
    for record in log.read(0, 100).expect("read") {
        println!("{} {:?}", record.offset, record.payload);
    }
    log.seal().expect("seal");
}
```

The signatures and defaults are in [api.md](docs/architecture/api.md).

## Build

The toolchain is pinned in [`rust-toolchain.toml`](rust-toolchain.toml); `rustup` installs it on the first build.

```sh
cargo build --release                       # the library and the server
rustup target add x86_64-unknown-linux-musl
cargo build --release -p segment-log-server --target x86_64-unknown-linux-musl   # the guest binary, on Linux x86_64
```

`segment-log-core` depends only on `std`. `segment-log-server` adds only `libc` ([ADR 0058](docs/decisions/0058-one-server-process-serves-one-log.md)).

## Run the server

One process serves one log directory, over a Unix socket or, on Linux, a vsock port:

```sh
mkdir -p data   # open creates the log directory, not its parents
target/release/segment-log-server --dir data/log --unix data/log.sock
target/release/segment-log-server --dir /mnt/log --vsock-port 5000             # in a Linux guest
target/release/segment-log-server --dir /mnt/log --vsock-port 5000 --read-only # serve a sealed log
```

The [server README](segment-log-server/README.md) lists every flag and exit status. The bytes on the socket are in [protocol.md](docs/architecture/protocol.md). Operators start at the [operator docs](docs/operators.md).

## Test

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
env SEGMENT_LOG_TEST_ROLL_BYTES=256 cargo test --workspace   # the scenarios again, rolling often
```

`make check` runs the first three. A release is `make release VERSION=X.Y.Z` ([scripts/release.sh](scripts/release.sh)); try it first with `make release-dry-run VERSION=X.Y.Z`.

The Go client runs the shared conformance cases against a server binary (the same lines work in fish, zsh and bash):

```sh
cargo build -p segment-log-server
env SEGMENT_LOG_SERVER=$PWD/target/debug/segment-log-server go -C clients/go test -count=1 ./...
```

[testing.md](docs/architecture/testing.md) covers the crash model, the fuzz targets and the benchmarks.

## FAQ

**What does an acknowledgment promise?**
The submit's records are durable: the group's `fsync` returned success and the watermark moved past it (R4, [ADR 0002](docs/decisions/0002-group-commit-and-the-watermark.md)). That holds only if the storage meets the [volume requirements](docs/architecture/volume-requirements.md); if it does, recovery never loses an acknowledged group, leaves a hole or changes the order. A submit without an acknowledgment might be durable.

**Which platforms are supported?**
Linux on a local ext4 or xfs filesystem ([ADR 0005](docs/decisions/0005-flush-calls-and-platforms.md)). Every qualification so far used ext4; xfs is untested and a deployment on it needs its own qualification. macOS is for development: its tests run in CI, and its numbers are not used for the targets. vsock is Linux only.

**What is the compatibility promise?**
From the 0.1 tag, every later build reads every format version from 3 and every server speaks every protocol version from 2, until a major release. The crates, the server's flags and exit statuses, and the Go client follow 0.x semver. An upgrade of the format is one-way: an older build refuses a newer segment ([ADR 0064](docs/decisions/0064-the-0-1-compatibility-promise.md)).

**What happens on an I/O error, such as a failed `fsync`?**
The writer is poisoned and never retries the `fsync`. Submits in flight get `Indeterminate`, and later submits get `Poisoned`. Reopening the log recovers it ([ADR 0004](docs/decisions/0004-io-errors-poison-the-writer.md)). The server exits with status 2 after any poisoned answer, and its supervisor restarts it ([ADR 0058](docs/decisions/0058-one-server-process-serves-one-log.md)).

**How do retries and idempotency keys work?**
A key is scoped to the whole log. A retry with the same keys, streams and payloads returns the original log offsets and revisions and appends nothing. A key reused with other bytes gets `KeyConflict`; a submit that mixes matched and new records gets `PartialRetry`. A key stays valid while its segment is retained (R10–R13). To resolve an unknown outcome, see [retry after an unknown outcome](docs/architecture/api.md#retry-after-an-unknown-outcome).

**Does the log delete old data?**
No. The log keeps its complete history. The deployment archives or expires whole logs ([ADR 0037](docs/decisions/0037-retain-complete-logs-through-current-milestones.md)). A full volume poisons the writer, so plan capacity ([V9](docs/architecture/volume-requirements.md#v9-capacity-and-a-timely-alert)).

**Can two processes write one log, or one server serve many logs?**
No. One writer thread owns a log, and the `LOCK` file keeps a second writer out under one kernel. Across guests, the deployment fences the volume ([V5](docs/architecture/volume-requirements.md#v5-one-writing-kernel)). One server process serves one log; run one process per log ([ADR 0058](docs/decisions/0058-one-server-process-serves-one-log.md)).

**Who can reach the server?**
Anyone who can reach its socket. The protocol has no authentication. The socket file's mode (`0600`), its directory, or the vsock topology decide access ([ADR 0058](docs/decisions/0058-one-server-process-serves-one-log.md)).

**What if open fails with `Corrupt`?**
Open fails closed and never truncates a sealed segment, nor an unsealed one whose damage has acknowledged records after it ([ADR 0068](docs/decisions/0068-open-refuses-damage-with-acknowledged-groups-after-it.md)). Run `segment-log-server repair verify --dir <path>` to diagnose it. `repair truncate` repairs only the newest segment, shows its plan first, and copies the segment aside before it changes anything ([ADR 0065](docs/decisions/0065-the-repair-tool-truncates-only-the-newest-segment.md); flags in [api.md](docs/architecture/api.md#operator-repair)).

**Is there a Go client?**
[`clients/go/`](clients/go/) holds the Go client for the server. It also runs the conformance cases. From the 0.1 tag it follows 0.x semver and is tagged `clients/go/vX.Y.Z` ([ADR 0064](docs/decisions/0064-the-0-1-compatibility-promise.md) rule 15). `DialPool(network, address, n)` shares n pipelined connections among goroutines ([issue 134](https://github.com/ironflowx/segment-log/issues/134)). That pool is in the tagged module at `clients/go/v0.2.1`. On a connection, requests queued while a write is in progress go out together in the next write, and each answer returns to its caller in request order. A connection holds at most 64 requests in flight; a request beyond that waits. When a connection fails, a request already written and not answered gets `ErrUnknown` and is not retried, and a request not yet written gets `ErrNotSent`. The next request for that connection dials a new one, and gets `ErrNotSent` if that dial fails. The guest throughput needs callers that share pipelined connections ([ADR 0063](docs/decisions/0063-the-guest-benchmark-shares-pipelined-connections.md)). A `Client` is one connection that goroutines cannot share, so one connection per caller gets a lower rate.

**Is the log replicated?**
No. It keeps one copy. Surviving host or device loss needs replicated storage under it ([V7](docs/architecture/volume-requirements.md#v7-a-declared-failure-domain), [PRD open item 12](docs/prd.md#open-items)).

## Documents

- [Product requirements](docs/prd.md) and the [architecture overview](docs/architecture/overview.md)
- [Byte layout](docs/architecture/format.md) · [API](docs/architecture/api.md) · [Wire protocol](docs/architecture/protocol.md) · [Testing](docs/architecture/testing.md) · [Prior art](docs/architecture/prior-art.md)
- [Operator docs](docs/operators.md) · [Volume requirements](docs/architecture/volume-requirements.md)
- [Implement a behavior with acceptance tests](docs/acceptance-tests.md) · [Glossary](docs/glossary.md) · [Decisions](docs/decisions/README.md)

## License

MIT. See [LICENSE](LICENSE).
