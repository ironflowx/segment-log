# Issue 200: growth of the key index on the writer

Question: does splitting the active key map's index into 64 small maps (#202) take its growth off
the writer's latency? **Yes. Both rules set before the run pass.** The paced maximum does not clearly
change.

## Host and commits

Hetzner auction #3119419, 2026-11-23: EPYC 7502P, 128 GB, Debian 12 (6.1.0-56, glibc 2.36), /tmp as
ext4 on a Samsung PM983 MZQLB960HAJR-00007 EDA5502Q. [grow.sh](grow.sh) slowed it as in the
[issue 154 record](../issue154/README.md): 24 cores (48 threads), boost off, at most 1.5 GHz. It waited
until `ext4lazyinit` was done. 5aa9ddb (base) and feaa2de (#202), interleaved. [grow.py](grow.py) gives
the numbers.

- **plain:** 1,000,000 records with 256-byte payloads at 50,000 records/s. A segment holds about 226,150
  keys.
- **shrink:** 232 bytes from record 226,001 on. The second segment holds about 246,000 keys, and its
  index was sized for 226,150, which gives room for 229,376.

## Rules set before the run

1. No gap over 2 ms between group writes at 114,688 keys in the traced runs. **Pass:** 1.0–1.2 ms.
2. Paced p99 within the bound in 5 of 5 runs. **Pass:** 5 of 5, and 5 of 5 for shrink.

## Longest gap between group writes near each doubling, ms

Three traced runs each, within 1,500 records of the doubling.

| Doubling | Base | #202 |
|---|---|---|
| First segment, 14,336 keys | 2.0–2.5 | 1.1 |
| First segment, 28,672 keys | 3.5–3.6 | 1.1 |
| First segment, 57,344 keys | 6.4–7.1 | 1.1–1.2 |
| First segment, 114,688 keys | 10.4–11.0 | 1.0–1.2 |
| Second segment outgrows its index (shrink), 229,376 keys | 17.3–18.3 | 1.0–1.3 |

The last row measures what issue 200 had only estimated (about 20 ms).

## Paced and closed-loop runs

Medians of 5 runs, with the range.

| | Base, plain | #202, plain | Base, shrink | #202, shrink |
|---|---|---|---|---|
| Paced p99 ms | 1.52 (1.51–1.53) | 1.51 (1.50–1.52) | 1.51 (1.49–1.54) | 1.51 (1.49–1.52) |
| Paced p999 ms | 3.50 (3.31–3.65) | 3.01 (2.47–3.38) | 7.21 (6.52–9.16) | 3.71 (2.99–3.88) |
| Paced max ms | 12.2 (11.5–18.6) | 10.4 (4.8–14.8) | 19.4 (19.0–19.7) | 11.9 (11.2–15.6) |
| Bound ms | 2.13 | 2.14 | 2.14 | 2.13 |
| Runs within the bound | 5 of 5 | 5 of 5 | 5 of 5 | 5 of 5 |
| Closed-loop records/s | 145,280 (144,027–145,722) | 148,237 (146,770–149,218) | | |
| Peak resident MiB, closed loop | 504 (501–509) | 500 (499–506) | | |

- **The maximum:** it falls for shrink. For plain the ranges overlap. In the traced runs of #202 the
  longest gaps that are left are mostly among the last 1,000 records, as the callers finish, and that is
  not the index. The paced runs do not show where their maxima are.
- The index's larger memory in some full segments does not show in the peak resident set.
