// The README is the crate's front page on docs.rs; its links are absolute so they resolve there.
#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]

mod clock;
mod codec;
mod storage;

#[cfg(test)]
mod acceptance;
mod api;
pub use api::*;
mod engine;
#[cfg(fuzzing)]
#[doc(hidden)]
pub use engine::fuzzing;
/// The operator repair tool's core (ADR 0065). Not part of the API promise: use
/// `segment-log-server repair`.
#[doc(hidden)]
pub use engine::repair;
pub use engine::{LogReader, Pending, SegmentLog};

/// A caller-supplied storage and clock, and a log whose background work runs as steps the caller
/// schedules, for a deterministic simulator ([ADR 0072]). Every storage call and clock reading of
/// a [`SteppedLog`](simulation::SteppedLog) happens inside one of its methods, so a seed that
/// fixes the calls, the storage's results and the clock's readings fixes the whole run.
///
/// [ADR 0072]: https://github.com/ironflowx/segment-log/blob/main/docs/decisions/0072-a-simulator-drives-the-log-through-public-storage-a-clock-and-steps.md
#[cfg(feature = "simulation")]
pub mod simulation {
    pub use crate::clock::Clock;
    pub use crate::engine::{Step, SteppedLog, Stop, Task};
    pub use crate::storage::{Call, Lock, Op, Purpose, Storage, Value};
}

#[cfg(test)]
mod pipeline;
