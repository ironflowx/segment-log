use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Every time the log reads (ADR 0072). A simulator supplies its own; `SegmentLog` uses the
/// system clock.
pub trait Clock: Send + Sync {
    /// Time since an origin the clock chooses. It must never decrease. The linger runs on it.
    fn monotonic(&self) -> Duration;
    /// Milliseconds since the Unix epoch, for a group's append time. It may step back.
    fn wall_millis(&self) -> u64;
}

pub(crate) struct System {
    origin: Instant,
}

impl System {
    pub(crate) fn new() -> Self {
        Self {
            origin: Instant::now(),
        }
    }
}

impl Clock for System {
    fn monotonic(&self) -> Duration {
        self.origin.elapsed()
    }
    /// A clock before 1970 reads 0.
    fn wall_millis(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
    }
}
