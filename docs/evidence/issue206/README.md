# Issue 206: does the writer state machine slow the real path?

Question: [ADR 0072](../../decisions/0072-a-simulator-drives-the-log-through-public-storage-a-clock-and-steps.md) turns `SegmentLog`'s writer loop into steps that a `SteppedLog` also runs. Does the default path stay as fast as `main`, within the noise rule the ADR set before the run? **No, not by that rule.** The second capture fails it only on paced p50, by 3 µs. The first capture also fails on closed-loop throughput and on one paced run. The syscall checks pass: the real path makes the same flushes in the same order.

## Host and commits

GitHub-hosted `ubuntu-26.04` runners, 2026-11-27, through the `m1-compare` and `m3-evidence` captures of the [evidence workflow](../../../.github/workflows/evidence.yml). Each one is a shared VM with 4 vCPUs of an AMD EPYC 9V45 and a 150 GB "MSFT NVMe Accelerator v1.0" disk. The workflow does not report its firmware. The two `m1-compare` captures ran on different VMs, so compare base and head within one capture only. The toolchain is rustc 1.100.0. [`scripts/m1-compare-refs.sh`](../../../scripts/m1-compare-refs.sh) builds both commits and runs five alternating pairs of the closed loop, then five pairs paced at 50,000 records/s ([ADR 0045](../../decisions/0045-latency-target-is-measured-at-a-paced-rate.md)). No new driver script was written for this record.

- **Base:** `66a86a3`, `main` at the start of the work.
- **Capture 1 head:** `5434249`, the first implementation (run 46047259362).
- **Capture 2 head:** `127e976`, the same code minus an options `Arc` clone per classified request that capture 1 pointed to (run 46049095295).

## Rule set before the run (ADR 0072)

For closed-loop records/s and for paced p50 and p99, the head's median lies within the range of the base's five runs. The head also has no fewer paced runs within the ADR 0045 bound than the base.

## Results

| Measure | Capture 1 base, median (range) | Capture 1 head, median | Capture 2 base, median (range) | Capture 2 head, median |
|---|---|---|---|---|
| Closed-loop records/s | 347,572 (345,970–355,328) | 342,847 **outside** | 317,370 (300,258–377,690) | 342,368 within |
| Paced p50 ms | 1.077 (1.027–1.078) | 1.080 **outside** | 0.988 (0.983–0.993) | 0.996 **outside** |
| Paced p99 ms | 4.804 (2.598–8.738) | 6.423 within | 1.492 (1.470–2.213) | 1.508 within |
| Paced runs within the bound | 5 of 5 | **4 of 5** | 5 of 5 | 5 of 5 |
| Paced mean flush µs, range | 317–349 | 319–387 | 267–279 | 277–287 |

The paced p50 differences are 3 µs and 8 µs, about 1% of one millisecond. The head's mean flush time is up by about the same amount in both captures. The code does not set that number: it is the device's time for each group's `fdatasync`. Capture 1's failed paced run had a p99 of 68.8 ms, and its base had one at 8.7 ms, which shows the scale of the runner noise. This record does not claim that noise explains the misses. The rule says they fail.

## Syscall checks

| Check | Commit | Result |
|---|---|---|
| `m1-verify-syscalls.py`: one `fdatasync` per group commit, none for a deduplicated retry, one at drop for the close frame | `5434249` | Pass (run 46047262264) |
| `m3-verify-roll-syscalls.py`: the roll's create, header write, flushes and directory `fsync` before the next group frame, and the closing run once | `127e976` | Pass (run 46049098503) |

Capture 1's m3 run (46047265506) and a run on `main` (46048389862) both failed the same way. The verifier still expected no segment write after the workload, but since 0.2.0 a clean drop writes a close frame ([ADR 0071](../../decisions/0071-a-clean-drop-writes-a-close-frame.md)). The verifier now expects the close frame's write and its `fdatasync`. Its fixture is `main`'s capture from run 46048389862.

## Verdict

The real path keeps its calls. By the rule set before the run, capture 2 misses only paced p50, by 3 µs, and capture 1 misses three measures. The owner decides whether to accept that. A paired run on a dedicated host, as for [issue 200](../issue200/README.md), would separate the code from the runner.
