# Few-caller appends

Question: what closed-loop append rate and acknowledgment latency does the issue 49 harness report at 1, 4, 16 and 1,000 callers on one host? The 1,000-caller run is the anchor. The other counts are read beside it.

This host is a KVM cloud VM. The reference hosts behind the README table are an i7-7700 with a Samsung PM981 and the EPYC 7401P and 7502P machines. The absolute numbers here are not comparable to that table.

## What ran

Commit `e2f461f407f74ce009592def2fe2af59d2b766ae`. The library sources are that commit. [run.sh](run.sh) is the driver. It was untracked while the binary was built, and it changes no library code. Caller count was already `SEGMENT_LOG_BENCH_CALLERS` on `measured_appends_survive_reopen` (default 4). The driver only sets it.

The test is `acceptance::evidence::measured_appends_survive_reopen`, the closed loop in `scripts/m1-evidence-guest.sh`. The run leaves the defaults. The `CONFIG` line is:

```text
CONFIG callers=N rounds=R payload=256 records_per_submit=1 seed=none_literal_formula linger_ns=1000000 group_byte_cap=1048576 roll_bytes=67108864 queue_capacity=4096 pending_byte_budget=16777216
```

There is no paced interval. `DIAGNOSTIC measured_storage=true`. An acknowledgment returns from `Pending::wait` only after the group's `fdatasync` (`File::sync_data`), the same call the issue 49 capture uses.

Each measured run appends 1,000,000 records, the issue 49 closed-loop count. Rounds are 1,000,000 divided by the caller count: 1000000, 250000, 62500 and 1000. `SEGMENT_LOG_DEVICE_ROUNDS=1000`. Three measured runs of each count, interleaved, 1 then 4 then 16 then 1,000. A warmup of each count ran first (rounds 1000, 1000, 400 and 20, device rounds 8) and is not in the table. Every measured run started after `scripts/m1-quiet.py` reported `guest_busy_percent_before` of 0.0 or 0.2, with `quiet_seconds=2`.

Release build, `rustc 1.100.0 (0ef1bc386 2026-11-09)`. Binary sha256 `d98319ebb735590f633f02bd73bcbf9500f29e99ea284bfdce0436671f3f536d`.

Command, from the repository root, on that commit:

```sh
bash docs/evidence/few-callers/run.sh
```

`FEW_CALLERS_OUT` was unset, so the driver wrote `/tmp/few-callers`. The log directory was ext4 on `/dev/vdc`. Each measured run was:

```sh
SEGMENT_LOG_EVIDENCE_DIR=/tmp/few-callers/data \
SEGMENT_LOG_BENCH_CALLERS=N \
SEGMENT_LOG_BENCH_ROUNDS=R \
SEGMENT_LOG_DEVICE_ROUNDS=1000 \
SEGMENT_LOG_BENCH_DEADLINE_S=7200 \
"$binary" --exact acceptance::evidence::measured_appends_survive_reopen --nocapture
```

`$binary` is the release test binary from `cargo test -p segment-log-core --lib --release --no-run`.

## Host

2026-11-27. The capture is [host.md](host.md).

- CPU: `Intel(R) Xeon(R) Processor`, family 6, model 207, stepping 2. `CPU(s): 4`, 1 thread per core, 4 cores, 1 socket. BogoMIPS 4800.00. Hypervisor vendor KVM, virtualization type full. 4 vCPUs. `nproc` printed 4.
- Kernel: `Linux cursor 6.12.113+ #1 SMP PREEMPT_DYNAMIC Thu Nov 26 16:06:57 UTC 2026`.
- OS: Ubuntu 24.04.5 LTS.
- Filesystem: ext4 on `/dev/vdc`, `rw,relatime`. The log directory was on that mount.
- Block device: virtio disk `vdc`, 256G, `rotational` 1, scheduler `mq-deadline`, logical and physical block size 512. DMI vendor, product name and product version were empty.
- Write cache: the guest queue reports `write through`. Whether the host disk behind the virtio device has a volatile write cache is unknown.

The flush check on the same filesystem, before the runs:

```text
sync=False size=4096 n=200 min=1164 p50=1665 p99=3643 max=11960 clean_fdatasync_ns=3964913
sync=True size=4096 n=200 min=84698 p50=93562 p99=164773 max=166613 clean_fdatasync_ns=1726
sync=True size=262144 n=100 min=339474 p50=454192 p99=2189853 max=2256065 clean_fdatasync_ns=1436
```

A 4 KiB write plus `fdatasync` has p50 93562 ns. The same write without `fdatasync` has p50 1665 ns. A `fdatasync` with nothing dirty, after those synced writes, took 1726 ns. The guest waits for `fdatasync`. The host cache is unknown, so this does not show that a flush reached stable media.

## Results

Medians of 3 runs, with the range. Every figure is a harness token. Acknowledgment latency is `METRIC append_ns`. Group `fdatasync` is `METRIC group_fdatasync_ns`, the flush of each group this run wrote. Device `fdatasync` is `METRIC device_fdatasync_ns`, 1,000 appends of 262144 bytes then `fdatasync`, measured before the appends. Records/s is `APPEND records_per_second`. Append elapsed is `APPEND elapsed_ns`, the interval that rate uses. `finished in` is the whole test: device flushes, appends, read-back and reopen.

| Callers | Records/s | Ack p50 ns | Ack p99 ns | Ack p999 ns | Append elapsed ns | Test s |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | 565.151 (550.628–588.238) | 1792982 (1690880–1832232) | 2742171 (2699223–2747492) | 3152691 (3099432–3165276) | 1769437786513 (1699991119144–1816107057332) | 1771.92 (1702.38–1819.13) |
| 4 | 2513.059 (2484.555–2541.291) | 1578814 (1576119–1650820) | 2473002 (2442643–2544789) | 2890203 (2858779–3033960) | 397921352393 (393500741987–402486567398) | 400.94 (396.42–405.37) |
| 16 | 9864.942 (9798.052–9908.190) | 1630635 (1596998–1662044) | 2503087 (2482924–2524754) | 2914047 (2913848–2958392) | 101369068810 (100926607064–102061100128) | 104.33 (104.04–104.85) |
| 1000 | 277218.842 (270485.566–289753.059) | 3409981 (3390601–3657435) | 5807757 (5560648–7168937) | 9403344 (7429191–9980802) | 3607258416 (3451214638–3697054942) | 6.26 (6.15–6.31) |

| Callers | Group fdatasync p50 ns | Group fdatasync p99 ns | Device fdatasync p50 ns | Device fdatasync p99 ns | Group submits p50 |
| --- | --- | --- | --- | --- | --- |
| 1 | 670301 (566764–700966) | 1555087 (1510805–1562693) | 279262 (264229–294503) | 1365774 (1088450–1552124) | 1 (1–1) |
| 4 | 504675 (501660–583554) | 1300553 (1272967–1356987) | 333277 (290549–456430) | 1736067 (1640018–1926694) | 4 (4–4) |
| 16 | 554207 (505423–588856) | 1290763 (1265647–1308779) | 320098 (314233–390706) | 1699826 (1528413–1767901) | 16 (16–16) |
| 1000 | 518073 (279465–696048) | 1224528 (1130624–1314197) | 301088 (272512–319804) | 1466347 (1230750–1476335) | 500 (499–501) |

Every measured run reported `records=1000000`, `full=0`, `other_admission_errors=0`, `commit_errors=0` and `correctness_errors=0` on append and on reopen, and `test result: ok. 1 passed`.

Group submits p50 equals the caller count at 1, 4 and 16, so each group is one submit from each caller. At 1,000 callers a group holds 500 submits at the median, and one `fdatasync` covers that group. One caller ended at 6 segments and 354104596 bytes in every run. The other counts ended at 5 segments. The `END` byte totals are in [runs.md](runs.md).

The guest clock stepped during the session. The UTC stamps in [host.md](host.md) are not durations. `elapsed_ns` and `finished in` are the harness monotonic clock.

## Raw output

[runs.md](runs.md) is the stdout of the 12 measured runs. Stderr was empty. [host.md](host.md) is `host.txt`, the flush check, the binary sha256 and the driver progress lines.
