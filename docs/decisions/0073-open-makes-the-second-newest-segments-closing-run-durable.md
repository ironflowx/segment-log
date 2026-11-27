---
id: "0073"
title: "Open rewrites the second-newest segment's footer and locator, so a closing run found only in the page cache becomes durable"
status: accepted
date: 2026-11-27
deciders: [Altug Sahin]
supersedes: null
superseded_by: null
tags: [recovery, durability, release-0.3]
touches: [segment-log-core/src/engine/recovery.rs, segment-log-core/src/acceptance/roll/crashes.rs, segment-log-core/src/acceptance/summary_open.rs, docs/architecture/overview.md, docs/architecture/testing.md, docs/decisions/0015-open-makes-the-recovered-tail-durable.md, docs/decisions/0061-a-sealer-thread-builds-the-key-frames-off-the-writer.md, CHANGELOG.md]
enforced_by: test
---

# 0073. Open rewrites the second-newest segment's footer and locator, so a closing run found only in the page cache becomes durable

## Context

The seeded crash test that [ADR 0072](0072-a-simulator-drives-the-log-through-public-storage-a-clock-and-steps.md) specifies (`STEPPED-CRASH`, in [pull request 208](https://github.com/ironflowx/segment-log/pull/208)) found a log that no longer opens. Its first failing seed ran in this order:

1. The sealer appends a rolled segment's key filter and key index and flushes them, then appends its footer and locator ([ADR 0067](0067-the-sealer-writes-and-flushes-the-closing-run.md)).
2. The process dies before the second flush. The footer and locator are in the page cache only.
3. Open reads them, finds them valid, and counts the segment as sealed (overview.md, open step 5). It writes nothing to that segment. [ADR 0015](0015-open-makes-the-recovered-tail-durable.md) rewrites only the newest segment's tail.
4. The writer rolls again, so the segment is no longer the second-newest. ADR 0061 rule 8 rebuilds a missing run only in the second-newest.
5. A power loss drops the page that holds the locator. Every later open fails with `Corrupt` at that segment, and the log cannot open until the repair tool runs.

A failed second flush leads to the same state. After a failed `sync_data` the bytes stay readable, but a later successful flush need not make them durable unless they are written again ([ADR 0015](0015-open-makes-the-recovered-tail-durable.md), Consequences). The writer is poisoned, the next open finds the run whole, and the log rolls on.

These facts are fixed:

- No group is lost: the segment's groups were flushed before the roll. But the log is out of service, which ADR 0015 exists to prevent: whatever open keeps must be durable.
- The key filter and key index are always durable when a footer exists, because the sealer flushes them before it writes the footer (ADR 0061 rule 3, ADR 0067 rule 1). Only the footer and locator can be in the page cache alone.
- The gap exists since ADR 0061 (0.1): the writer then wrote the run itself, with the same two flushes.
- The in-crate roll crash tests did not catch it, because they lose power while the segment is still the second-newest, where rule 8 rebuilds the run.

## Options & trade-offs

### Option A - rewrite the second-newest segment's footer and locator at open

When the newest segment is not sealed and the second-newest's run was proved, not rebuilt, open reads the bytes from the footer to the end of the file, writes them back in place, and calls `sync_all`, as ADR 0015 does for the tail.

- **For:** the same identical rewrite that ADR 0015 already relies on, with the same sector-atomicity assumption. It reads only the footer and the locator, so `ORACLE-13` holds.
- **Against:** one write of the footer and locator (their widths are in [format.md](../architecture/format.md)), and one more `sync_all` at each writable open of a log with more than one segment, even when the run was durable all along.

### Option B - only `sync_all` the second-newest segment

- **For:** no write.
- **Against:** after a failed flush the page cache may hold the bytes as clean, so a flush writes nothing. Only a new write makes them durable, which is why ADR 0015 rewrites rather than syncs.

### Option C - rebuild the run whenever the newest segment has no group yet

- **Against:** the newest segment can already hold groups when the process dies, because the sealer runs beside them. The failing seed had groups there.

### Option D - make step 5 trust a run only when its flush is proven

- **Against:** one barrier cannot record its own completion (ADR 0003). Nothing on disk tells a flushed run from one that is only in the page cache.

### Option E - leave it to the repair tool

- **Against:** a crash and a later power loss, both within the storage contract, would take the log out of service. That contradicts R14 and ADR 0015.

## Decision

We will build option A. During open, after the reads of step 11 and before any other change, if the newest segment is not sealed (it ends in groups, or the scan stopped), and the second-newest segment's closing run was proved by its locator and footer rather than rebuilt under ADR 0061 rule 8, open:

1. reads that segment's bytes from the footer's position to the end of the file;
2. writes them back at the same position;
3. calls `sync_all` on the segment.

A failure fails open with `Io` and is not retried ([ADR 0004](0004-io-errors-poison-the-writer.md)). The bytes are identical, so I7 holds and the segment's content does not change. A rebuilt run is already written and flushed by rule 8. A sealed log, or a newest segment that ends in its own closing run, needs nothing: a roll and a seal wait for a pending run to be durable (ADR 0061 rules 4 and 7).

We accept one more small write and flush at open, which is cheap beside open's existing flushes, to keep every log that open has kept openable after a power loss.

## Consequences

- Accepted by the owner on 2026-11-27, split out of pull request 208 so that it ships before the simulation API ([issue 206](https://github.com/ironflowx/segment-log/issues/206)).
- Amends ADR 0015: open rewrites the second-newest segment's footer and locator as well as the newest segment's tail. overview.md's open step 5 says so.
- Amends ADR 0061 rule 8 and overview.md's open step 5: a run proved by its locator and footer is made durable before the log rolls past it.
- Each writable open of a log with more than one segment makes one more write and one more `sync_all`. `ORACLE-13` is unchanged, because the extra read covers only the footer and the locator.
- The in-crate open read-count scenarios list the rewrite's calls for the second-newest segment.
- A log that an older build reopened and then rolled past such a run still holds it unflushed: this open looks only at the second-newest segment. A power loss there still needs the repair tool.

## Enforcement

- `KEY-FRAMES-CRASH` gains three scenarios in `segment-log-core/src/acceptance/roll/crashes.rs`. Each failed before this change.
  - `a_whole_closing_run_found_unflushed_is_made_durable_by_open`: a kill after the footer and locator writes and before their flush, a reopen, more rolls, then a power loss. The log still opens with every record.
  - `a_closing_run_whose_flush_failed_is_made_durable_by_open`: the same after a failed second flush.
  - `a_crash_or_fault_inside_opens_rewrite_of_the_run_recovers`: a crash before and after each of the rewrite's calls in every mode, and a failed write or flush, which open does not retry. The next open, more rolls and a power loss keep every record.
- `STEPPED-CRASH` (pull request 208) reaches the same order from its seeds once it lands.

## Revisit if

- Open time matters more than one flush and a later design can prove a run's flush from disk.
- The sealer gains a third write after the footer and locator.
