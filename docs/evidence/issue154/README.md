# Issue 154: paced p99 on the EPYC host

The requalified host (EPYC 7401P, 2x PM983 EDA5502Q) missed the paced p99 bound in 5 of 5 runs at
82bb5b1 (p99 29.1 ms; docs/evidence/m7/requalified-host/m1-capture). This rerun measured three
commits with [run.sh](run.sh) on 2026-11-18.

## Host

Hetzner auction #3116374: EPYC 7502P, 128 GB, 2x Samsung PM983 MZQLB1T9HAJR EDA5502Q, write cache
write through, Debian 12 (6.1.0-54), the second NVMe as ext4 on /tmp. No 7401P was offered, so
the drive matches the requalified host and the CPU does not. Each commit's `host-extra.txt` has
the rest.

## Results ([epyc7502p](epyc7502p))

| Commit | Closed-loop records/s | Paced p99 ms | Paced runs within the bound |
|---|---|---|---|
| 82bb5b1 (the requalified capture) | 178,810 | 5.47 (1.69–7.18) | 2 of 5 |
| 87ea195 (#163, hash map) | 203,976 | 27.26 (25.19–28.35) | 0 of 5 |
| 49ee5c8 (#163 and #164) | 210,175 | 9.03 (8.35–9.43) | 0 of 5 |

Medians of 5 runs, with the range.

## What the groups show

The paced group traces give each group's submits and frame bytes. A group far above the median
of about 50 submits follows a writer stall. Counting frame bytes places each stall in its segment:

- At every roll (64 MiB), on all three commits: the closing run's writes and flushes. On 49ee5c8
  the traced runs (`8b1a2d1/traced-*/stall.txt`) time it at 8–9 ms on the writer, then 30–37 ms
  without a group write; about 20% of the slowest 1% of requests start in that window.
- At about 8, 16 and 32 MiB into every segment, on 87ea195 and 49ee5c8 only: the active key map
  doubling. A `HashMap` moves every key at once when it grows; the `BTreeMap` it replaced did
  not. 32 MiB of 292-byte records is 114,688 keys, where a std `HashMap` grows from 2^17 buckets.

The 14 commits from 82bb5b1 to 87ea195 change the write path only in #163; #147 and #149 touch
the core off it. #164 then took the log filter's build off the writer, as `SegmentLog::open` does,
which is why 49ee5c8 beats 87ea195.

## Fix

The roll sizes the next segment's map for as many keys as the rolled one, so a steady load never
grows it on the writer. The roll-time closing-run stall remains; moving its flushes off the group
path needs an ADR.

## Rerun with the fix ([epyc7502p-fix](epyc7502p-fix))

Hetzner auction #3116582, 2026-11-19: EPYC 7502P, 2x Samsung PM9A3 MZQL21T9HCJR GDC5A02Q (not the
PM983 of the first box), otherwise the same setup. Both commits ran on this box.

| Commit | Closed-loop records/s | Paced p99 ms | Paced runs within the bound | Mid-segment large groups per run |
|---|---|---|---|---|
| a0c13b1 (main) | 218,022 | 8.95 (8.40–9.16) | 0 of 5 | 13–14 |
| 8effa54 (main + the fix) | 219,213 | 4.14 (2.49–5.04) | 0 of 5 | 3 |

The bound is about 1.8 ms. The 3 large groups left in the middle of a segment fall in the first
segment after open, whose map has no rolled map to take its size from. The rest of the gap to the
bound is the closing run at each roll.

## The sealer writes the closing run ([ryzen3900-sealer](ryzen3900-sealer))

Hetzner auction #3116708, 2026-11-19: Ryzen 9 3900, 2x Toshiba KXD51RUE1T92 1CEE6111, write cache
write through, otherwise the same setup. No EPYC was offered. Both commits ran on this box.

| Commit | Closed-loop records/s | Paced p99 ms | Paced p999 ms | Paced max ms | Runs within the bound | Large groups at rolls per run |
|---|---|---|---|---|---|---|
| 2d4652b (main) | 312,232 | 1.36 (1.35–1.37) | 23.3 (22.9–23.5) | 28.9 (28.0–29.7) | 5 of 5 | 13–16 |
| 50c2125 (ADR 0067) | 315,150 | 1.35 (1.34–1.37) | 15.2 (14.7–15.9) | 21.1 (20.1–21.7) | 5 of 5 | 8–11 |

The bound is about 1.9 ms. On this faster CPU main already passes, because the roll stalls hold
fewer than 1% of the records, so p99 does not move. The stalls show in p999 and the maximum, which
fall by about a third. On main the traced runs time the writer's closing run at 7.9 ms per roll.

## EPYC 7401P rerun with ADR 0067

Question: does paced p99 meet the bound on the issue's CPU with ADR 0067 (#170) and the open-time
map sizing (#194, #195)? **No.** 0 of 5 runs on main.

Hetzner auction #3119456, 2026-11-23: EPYC 7401P, 128 GB, 2x Samsung PM963 MZQLW960HMJP-00003
CXV8601Q (not the PM983 of the requalified host), write cache write through, governor schedutil,
Debian 12 (6.1.0-56), the second NVMe as ext4 on /tmp. Both commits ran on this box, 0d8ed88 first.

| Commit | Closed-loop records/s | Paced p99 ms | Paced p999 ms | Paced max ms | Bound ms | Runs within the bound |
|---|---|---|---|---|---|---|
| 0d8ed88 (#168, before ADR 0067) | 152,801 | 22.2 (17.5–27.0) | 57.4 (48.6–58.1) | 69.9 (57.7–72.7) | 2.46 (2.04–12.0) | 0 of 5 |
| 115a60b (main) | 157,701 | 13.5 (11.2–14.8) | 47.4 (42.9–48.6) | 56.1 (55.5–58.0) | 2.21 (1.92–2.31) | 0 of 5 |

Medians of 5 runs, with the range. Runs 1 to 3 of 0d8ed88 are disturbed: 104 to 246 of their group
flushes took more than 3 ms, against 0 to 2 in every other run. The cause is not known. Runs 4 and 5
are not disturbed, and their p99 is 22.9 and 21.0 ms, so the fall on main does not rest on runs 1 to 3.

### Where the stalls are

Groups of at least twice the median submits, placed by frame bytes as above, per undisturbed run
(of 1,000,000 records):

| Place | 0d8ed88, runs 4–5 | 115a60b, runs 1–5 |
|---|---|---|
| After a roll (first 3.5 MiB of a later segment) | 34–37 groups, 15,341–16,519 records | 33–39 groups, 13,024–14,408 records |
| First segment at 4, 8, 16 and 32 MiB | 6 groups, 1,864–1,939 records | 6–7 groups, 1,855–2,071 records |
| Elsewhere | 6–7 groups, 802–937 records | 3–8 groups, 425–1,348 records |

- **Rolls decide p99.** On main the groups after a roll hold 1.3–1.4% of the records, more than the
  1% that p99 allows. The writer's own roll takes 0.61–0.90 ms, and main's traced runs time the
  pending step at 0.1 ms, but they show gaps of 17–51 ms between group writes around a roll. The
  next sections have the cause.
- **#194 and #195 do not reach this benchmark.** It opens an empty log (`records=0 segments=1`), so
  no sealed segment gives a size, and the first segment's map still grows on the writer. The same
  holds for the 3 groups in the 2026-11-19 rerun, which #194 named as its reason. The change only
  helps a log that has a sealed segment at open, which no capture here measures.

### Changes to the capture

The first capture of main stopped before its latency runs. ADR 0071's close frame adds one
`fdatasync` to a clean drop, and `m1-verify-syscalls.py` expected none. The main capture ran with the
corrected verifier and its test trace laid over 115a60b; no measured code differs. `run.sh` also
printed `capture exit 0` for that failure, because `$(git rev-parse)` replaced `$?`.

## What stalls the writer after a roll

Question: what holds the writer in those gaps? **The free of the rolled segment's key map, through
glibc's allocator.** The container runs here show it for one part of the stall; the EPYC runs in the
next section confirm it.

When the closing run is durable, the writer hands the old map to a thread that frees it (#170,
`write_pending`). That is about 226,000 key buffers that the writer allocated. glibc frees a buffer
into the arena it came from, so the thread works in the writer's arena while the writer allocates.

On the EPYC 7401P, in the 3 traced runs of 115a60b, all 12 rolls have a stall that starts 0.5–37 ms
after the closing run is durable. In the 60 ms after it the longest gap between group writes is
16.7–50.5 ms, against 1.2–8.6 ms in a control window 200–260 ms before it. One group's step from
formed to encoded takes 1.4–21.2 ms, against 0.0–0.3 ms. That step does no I/O and no roll work.

[arena.sh](arena.sh) ran three variants of 115a60b on 2026-11-23: rust:1.100.0-bookworm (glibc 2.36)
under OrbStack on an Apple silicon Mac, 10 CPUs, /data on tmpfs. Same load, 12 rolls of 64 MiB each.

| Variant | Longest formed-to-encoded step per roll | Same, control window | Writer time in allocator calls over 0.3 ms |
|---|---|---|---|
| As it is | 0.3–0.8 ms | 0.1 ms | 40.1 ms |
| glibc without fastbins | 0.1–0.2 ms | 0.0–0.1 ms | 28.1 ms |
| Old map never freed | 0.0–0.1 ms | 0.1–0.2 ms | 18.4 ms |

- **Shown:** without the free, the encode step after a roll does not stall. Two earlier runs gave the
  same order: 0.6–2.2, 0.3–1.8 and 0.0–0.1 ms at 64 MiB, and 0.5–0.8, 0.2–0.4 and 0.0–0.1 ms at
  256 MiB (6 rolls each, 177, 117 and 35 ms in slow allocator calls).
- **Expected and not seen:** four times the keys in a segment did not make one stall longer. So it is
  not one merge whose cost follows the key count. Some slow calls use CPU and others wait.
- **Not shown:** the longest gap, the records in large groups and paced p99 do not differ between the
  variants here. This host does not have the EPYC's stalls, and its disk and scheduler noise is as
  large as the effect.
- The builder of the log filter also starts when the closing run is durable. The next section rules
  it out.

## The map free decides p99 on EPYC

Question: how much of paced p99 does the map free cause, and does the log filter take part?
**The map free causes the miss. Without it p99 is 1.63 ms and 5 of 5 runs are within the bound. The
log filter takes no part.**

[variants.sh](variants.sh) ran four builds of 115a60b, interleaved, on Hetzner auction #3119465,
2026-11-23: EPYC 7401P, 128 GB, 2x Samsung PM963 MZQLW960HMJP-00003 CXV8601Q, Debian 12 (6.1.0-56,
glibc 2.36), the second NVMe as ext4 on /tmp. It waited until mkfs's background inode-table writes
(`ext4lazyinit`) were done. Nothing else was busy before any run.

| Variant | Paced p99 ms | Paced p999 ms | Paced max ms | Bound ms | Runs within the bound | Records in large groups after rolls, per run |
|---|---|---|---|---|---|---|
| As it is | 9.90 (6.94–12.00) | 40.0 (39.8–44.0) | 55.5 (48.5–56.2) | 2.21 | 0 of 5 | 10,694–13,688 |
| Old map never freed | 1.63 (1.62–1.64) | 16.3 (13.9–19.8) | 53.6 (33.9–73.1) | 2.20 | 5 of 5 | 716–919 |
| Log filter off | 11.04 (9.21–11.72) | 44.1 (37.4–49.7) | 53.5 (44.2–65.8) | 2.20 | 0 of 5 | 11,687–13,273 |
| Both | 1.61 (1.60–1.65) | 16.4 (15.2–18.6) | 44.1 (36.8–94.6) | 2.20 | 5 of 5 | 572–1,060 |

Medians of 5 runs, with the range. In the 3 traced runs of each variant (12 rolls), the longest gap
between group writes in the 60 ms after a closing run is durable is 20.3–44.9 ms as it is, and
1.3–6.8 ms when the map is never freed. The control window has 1.3–1.5 ms in both.

- Never freeing the map is a test, not a fix: it leaks the map of every rolled segment.
- What stays in p999 and the maximum is not the map free. The first segment's map growth (1,816–2,666
  records in large groups per run, 4–18 ms each in the traces above) is in all four variants. The
  cause of the largest maxima is not known.
- Runs 1 to 3 of 0d8ed88 above may have been disturbed by `ext4lazyinit`, which was running on this
  box after the same setup. That was not checked on the first box.

## The fix on an EPYC 7502P

Question: does one shared key buffer per map (#198) remove the stalls on EPYC? **Yes: p999 28.2 → 1.56
ms, and the stall after a roll is gone.** It does not show that the p99 bound is met on a 7401P: no
7401P with two NVMe drives was offered, and on this box the base commit already meets it.

[refs.sh](refs.sh) ran 115a60b (base) and 0f384cd (#198), interleaved, on Hetzner auction #3117926,
2026-11-23: EPYC 7502P, 128 GB, Debian 12 (6.1.0-56, glibc 2.36), /tmp as ext4 on a Samsung PM983
MZQLB960HAJR-00007 EDA5502Q (the other drive is a PM963). It waited for `ext4lazyinit` to finish.

| | Base 115a60b | Fix 0f384cd |
|---|---|---|
| Paced p99 ms | 1.49 (1.47–1.54) | 1.36 (1.35–1.36) |
| Paced p999 ms | 28.2 (26.9–28.7) | 1.56 (1.54–1.90) |
| Paced max ms | 34.2 (32.7–36.5) | 4.45 (4.33–5.30) |
| Bound ms | 1.83 (1.69–1.85) | 1.84 (1.78–1.86) |
| Runs within the bound | 5 of 5 | 5 of 5 |
| Records in large groups after rolls, per run | 7,267–7,754 | 0–371 |
| Records in large groups at 4–32 MiB of the first segment, per run | 1,000–1,128 | 302–428 |
| Closed-loop records/s | 212,418 (210,621–218,680) | 228,592 (225,755–228,968) |
| Closed-loop p99 ms | 7.19 (6.54–7.98) | 5.67 (5.40–5.72) |
| Peak resident MiB, closed loop | 519 (518–533) | 507 (497–509) |

Medians of 5 runs, with the range. In the 3 traced runs of each (12 rolls), the longest gap between
group writes in the 60 ms after a closing run is durable is 17.6–31.5 ms on the base and 1.2–2.8 ms
with the fix. The control window has 1.2–1.3 ms.

- The base meets the bound here because the groups after rolls hold 0.73–0.78% of the records, under the 1% that p99
  allows. On the 7401P they hold 1.1–1.4%.
- The large groups at the first segment's map growth also fall. A likely reason, not tested apart:
  growth now rehashes a small index of hashes, and no longer reads each key from its own buffer.

## The fix where the base misses the bound

Question: does #198 meet the p99 bound on an EPYC where the code before it misses? **Yes: 5 of 5 runs
at both speeds, against 0 of 5 before it.**

No 7401P with two NVMe drives was offered, and a 7502P at full speed meets the bound without the fix.
So [slow.sh](slow.sh) slowed a 7502P: 24 of 32 cores online (48 threads, as a 7401P has), boost off, and
a top speed of 2.2 GHz, then 1.5 GHz. The rule set before the run: a speed counts only if the base
misses the bound at it. Both count. [slow.py](slow.py) gives the rows.

Hetzner auction #3117912, 2026-11-23: EPYC 7502P, 128 GB, Debian 12 (6.1.0-56, glibc 2.36), /tmp as
ext4 on a Samsung PM983 MZQLB960HAJR-00007 EDA5502Q. 115a60b (base) and 5aa9ddb (main with #198),
interleaved, after `ext4lazyinit` was done. A fixed loop took 0.344 s as the host comes, 0.488 s at
2.2 GHz and 0.747 s at 1.5 GHz.

| | Base, 2.2 GHz | Fix, 2.2 GHz | Base, 1.5 GHz | Fix, 1.5 GHz |
|---|---|---|---|---|
| Paced p99 ms | 3.06 (2.84–3.58) | 1.40 (1.38–1.42) | 12.7 (11.6–14.3) | 1.50 (1.49–1.53) |
| Paced p999 ms | 31.7 (31.2–33.5) | 2.27 (2.13–2.41) | 40.9 (38.2–42.3) | 3.50 (3.40–3.64) |
| Paced max ms | 39.6 (38.1–41.9) | 8.16 (7.39–12.6) | 47.6 (43.7–50.9) | 11.5 (11.3–12.1) |
| Bound ms | 1.96 | 1.97 | 2.13 | 2.12 |
| Runs within the bound | 0 of 5 | 5 of 5 | 0 of 5 | 5 of 5 |
| Records in large groups after rolls, per run | 8,685–9,194 | 118–468 | 11,915–13,303 | 493–835 |
| Same, at 4–32 MiB of the first segment | 1,454–1,670 | 599–666 | 2,215–2,464 | 989–1,117 |
| Longest gap after a closing run, 8 rolls | 21.3–35.6 ms | 1.0–2.8 ms | 23.0–40.9 ms | 1.1–2.4 ms |

Medians of 5 runs, with the range. At 1.5 GHz the base's p99, p999 and maximum are close to the 7401P's
(9.90–13.5, 40.0–47.4 and 55.5–56.1 ms).

- This is a 7502P slowed down, not a 7401P: one NUMA node against four, and a newer core. The 7401P
  itself has not run with the fix.
- The first segment's map growth still holds about 0.1% of the records at 1.5 GHz, so it stays out of
  p99 and shows in the maximum.
