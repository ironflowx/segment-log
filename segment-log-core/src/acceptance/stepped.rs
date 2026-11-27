//! The stepped log on the simulated storage (ADR 0072).
use super::support::{bounded, fixtures::Sim, oracle, raw};
use crate::engine::{Step, SteppedLog, Stop, Task};
use crate::storage::{
    Op, Purpose,
    sim::{Crash, Event, Outcome},
};
use crate::*;
use std::{path::Path, sync::Arc, time::Duration};

const MODES: [Crash; 3] = [Crash::Kill, Crash::PowerLoss, Crash::PowerLossTorn];

/// Opens a stepped log that rolls before every group, and checks that recovery left every sealed
/// segment as it was (ORACLE-6).
fn open(sim: &Sim, queue_capacity: usize) -> SteppedLog {
    let before = raw::segments(sim);
    let options = Options {
        linger: Duration::ZERO,
        roll_bytes: super::support::fixtures::EVERY_GROUP,
        queue_capacity,
        ..Options::default()
    };
    let log = SteppedLog::open(
        Path::new("/log"),
        options,
        Arc::new(sim.clone()),
        Arc::new(sim.clone()),
    )
    .unwrap();
    oracle::sealed_unchanged(&before, &raw::segments(sim));
    log
}

/// The groups: one record, two records, one record, all keyed, on one stream.
const GROUPS: [&[(&[u8], Option<u64>)]; 3] = [
    &[(b"a", None)],
    &[(b"b", Some(0)), (b"c", Some(1))],
    &[(b"d", Some(2))],
];
/// The log offset after each whole group.
const ENDS: [usize; 4] = [0, 1, 3, 4];
const ALL: [&[u8]; 4] = [b"a", b"b", b"c", b"d"];

fn submit(group: &[(&[u8], Option<u64>)]) -> Submit {
    Submit {
        records: group
            .iter()
            .map(|&(payload, revision)| NewRecord {
                stream: b"s".to_vec(),
                expected: revision.map_or(ExpectedRevision::Empty, ExpectedRevision::At),
                key: Some(payload.to_vec()),
                payload: payload.to_vec(),
            })
            .collect(),
    }
}

/// Runs writer steps until the writer has nothing to do.
fn settle(log: &mut SteppedLog) -> Step {
    loop {
        match log.step(Task::Writer) {
            Step::Worked => {}
            other => return other,
        }
    }
}

fn answer(pending: Pending) -> Committed {
    match pending.try_wait() {
        Ok(result) => result.unwrap(),
        Err(_) => panic!("a settled writer answered every submit"),
    }
}

fn payloads(log: &SteppedLog) -> Vec<Vec<u8>> {
    log.read(0, 100)
        .unwrap()
        .into_iter()
        .map(|r| r.payload)
        .collect()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Order {
    /// A sealer step between the roll and the group after it.
    Before,
    /// A sealer step after that group.
    After,
    /// No sealer step: the writer runs the job where it would wait, at the next roll.
    Never,
}

/// Writes the three groups, rolling before each, then closes. Returns where the run started.
fn script(sim: &Sim, order: Order) -> usize {
    let mut log = open(sim, 16);
    let start = sim.trace().1.len();
    let first = log.append(submit(GROUPS[0])).unwrap();
    assert_eq!(settle(&mut log), Step::Idle { until: None });
    answer(first);
    let second = log.append(submit(GROUPS[1])).unwrap();
    // A roll ends a step, before the group that caused it is written.
    assert_eq!(log.step(Task::Writer), Step::Worked);
    assert_eq!(log.watermark(), 1);
    if order == Order::Before {
        assert_eq!(log.step(Task::Sealer), Step::Worked);
    }
    settle(&mut log);
    if order == Order::After {
        assert_eq!(log.step(Task::Sealer), Step::Worked);
    }
    if order != Order::Never {
        assert_eq!(log.step(Task::Sealer), Step::Idle { until: None });
    }
    answer(second);
    let third = log.append(submit(GROUPS[2])).unwrap();
    settle(&mut log);
    answer(third);
    log.close();
    assert_eq!(settle(&mut log), Step::Stopped(Stop::Closed));
    assert_eq!(log.step(Task::Sealer), Step::Stopped(Stop::Closed));
    start
}

/// The position in the trace of the first call that matches.
fn first(trace: &[Event], wanted: impl Fn(&crate::storage::Call) -> bool) -> usize {
    trace
        .iter()
        .position(|e| matches!(e, Event::Call(call, _, Outcome::Ok(_)) if wanted(call)))
        .unwrap()
}

/// Given a stepped log that rolls before every group
/// When the caller runs the sealer step before the group after a roll, after it, or never
/// Then the rolled segment's closing run is written before that group, after it, or at the next
/// roll, where the writer would wait for the sealer
/// And a crash at every storage call of each order, in every mode, reopens on the same storage to a
/// prefix of whole groups that keeps every flushed group, leaves sealed segments unchanged, answers
/// keyed retries with their original positions, and gives the same log after a power loss
///
/// Cases: STEPPED-ORDERS, ORACLE-1, ORACLE-2, ORACLE-3, ORACLE-4, ORACLE-5, ORACLE-6
#[test]
fn every_sealer_order_recovers_at_every_storage_call() {
    bounded(|| {
        for order in [Order::Before, Order::After, Order::Never] {
            let sim = Sim::new(7);
            let start = script(&sim, order);
            let (seed, trace) = sim.trace();
            let run = first(&trace, |c| {
                c.purpose == Purpose::Roll
                    && c.path == raw::segment(0)
                    && matches!(c.op, Op::Append(_))
            });
            let second_group = |op: fn(&Op) -> bool| {
                trace
                    .iter()
                    .enumerate()
                    .filter(|(_, e)| matches!(e, Event::Call(c, _, Outcome::Ok(_)) if c.purpose == Purpose::Group && op(&c.op)))
                    .nth(1)
                    .unwrap()
                    .0
            };
            let written = second_group(|op| matches!(op, Op::Append(_)));
            let flushed = second_group(|op| matches!(op, Op::SyncData));
            let third = first(&trace, |c| c.path == raw::segment(3) && c.op == Op::Create);
            match order {
                Order::Before => assert!(run < written, "{order:?}"),
                Order::After => assert!(flushed < run && run < third, "{order:?}"),
                Order::Never => assert!(flushed < run && run < third, "{order:?}"),
            }
            for cut in start..=trace.len() {
                let count = |op: fn(&Op) -> bool| {
                    trace[..cut]
                        .iter()
                        .filter(|e| matches!(e, Event::Call(c, _, Outcome::Ok(_)) if c.purpose == Purpose::Group && op(&c.op)))
                        .count()
                };
                let flushed = ENDS[count(|op| matches!(op, Op::SyncData))];
                let written = ENDS[count(|op| matches!(op, Op::Append(_)))];
                for mode in MODES {
                    let label = format!("{order:?} cut {cut} {mode:?}");
                    let replay = Sim::replay(seed, &trace[..cut]);
                    replay.crash(mode, cut as u64);
                    let mut log = open(&replay, 16);
                    let kept = payloads(&log);
                    assert_eq!(kept, ALL[..kept.len()], "{label}");
                    assert!(ENDS.contains(&kept.len()), "{label}: {kept:?}");
                    assert!(flushed <= kept.len() && kept.len() <= written, "{label}");
                    if mode == Crash::Kill {
                        assert_eq!(kept.len(), written, "{label}");
                    }
                    let mut offset = 0;
                    for group in GROUPS {
                        if offset == kept.len() as u64 {
                            break;
                        }
                        let retry = log.append(submit(group)).unwrap();
                        settle(&mut log);
                        let retry = answer(retry);
                        assert!(retry.deduplicated, "{label}");
                        let positions: Vec<_> = retry
                            .records
                            .iter()
                            .map(|p| (p.offset, p.revision))
                            .collect();
                        let original: Vec<_> = (offset..offset + group.len() as u64)
                            .map(|n| (n, n))
                            .collect();
                        assert_eq!(positions, original, "{label}");
                        offset += group.len() as u64;
                    }
                    assert_eq!(log.watermark(), kept.len() as u64, "{label}");
                    drop(log);
                    replay.crash(Crash::PowerLoss, cut as u64);
                    assert_eq!(payloads(&open(&replay, 16)), kept, "{label}");
                }
            }
        }
    });
}

/// Given a stepped log whose accept queue is full
/// When the caller seals it
/// Then `seal` returns at once and admission refuses with `Sealed`
/// And writer steps answer every queued submit before the seal, and end with `Stopped(Sealed)`
/// And a reopen finds the sealed log with every record
///
/// Cases: STEPPED-ORDERS
#[test]
fn a_seal_with_a_full_queue_returns_at_once_and_follows_every_queued_submit() {
    bounded(|| {
        let sim = Sim::new(8);
        let mut log = open(&sim, 2);
        let first = log.append(submit(GROUPS[0])).unwrap();
        let second = log.append(submit(GROUPS[1])).unwrap();
        assert!(matches!(
            log.append(submit(GROUPS[2])),
            Err(AppendError::Full)
        ));
        log.seal();
        assert!(matches!(
            log.append(submit(GROUPS[2])),
            Err(AppendError::Sealed)
        ));
        assert_eq!(settle(&mut log), Step::Stopped(Stop::Sealed));
        answer(first);
        answer(second);
        assert_eq!(log.watermark(), 3);
        assert_eq!(log.stream_head(b"s").map(|h| h.revision), Some(2));
        assert_eq!(log.step(Task::KeyFilter), Step::Stopped(Stop::Sealed));
        drop(log);
        let mut reopened = open(&sim, 2);
        assert_eq!(payloads(&reopened), ALL[..3]);
        assert_eq!(reopened.step(Task::Writer), Step::Stopped(Stop::Sealed));
    });
}

/// Given a stepped log with a submit gathering under a linger
/// When the caller steps the writer before the linger ends
/// Then the step is idle until the linger's end on the log's clock, and writes nothing
/// And once the clock passes that time, the next step writes the group
///
/// Cases: STEPPED-ORDERS
#[test]
fn a_gathering_group_waits_for_the_clock_not_for_a_timer() {
    struct Manual(std::sync::Mutex<Duration>);
    impl crate::clock::Clock for Manual {
        fn monotonic(&self) -> Duration {
            *self.0.lock().unwrap()
        }
        fn wall_millis(&self) -> u64 {
            0
        }
    }
    bounded(|| {
        let sim = Sim::new(9);
        let clock = Arc::new(Manual(std::sync::Mutex::new(Duration::from_secs(1))));
        let options = Options {
            linger: Duration::from_millis(5),
            ..Options::default()
        };
        let mut log = SteppedLog::open(
            Path::new("/log"),
            options,
            Arc::new(sim.clone()),
            clock.clone(),
        )
        .unwrap();
        let pending = log.append(submit(GROUPS[0])).unwrap();
        let end = Duration::from_secs(1) + Duration::from_millis(5);
        let until = Some(end);
        assert_eq!(log.step(Task::Writer), Step::Idle { until });
        assert_eq!(log.step(Task::Writer), Step::Idle { until });
        assert_eq!(log.watermark(), 0);
        *clock.0.lock().unwrap() = end;
        assert_eq!(log.step(Task::Writer), Step::Worked);
        answer(pending);
        assert_eq!(log.watermark(), 1);
    });
}
