//! A log that starts no thread: the caller runs its background work as steps (ADR 0072).
use super::writer::{Progress, Stop};
use super::*;

/// Background work that a [`SteppedLog`] runs only when the caller asks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Task {
    /// Takes queued submits, writes and flushes groups, rolls, seals and closes.
    Writer,
    /// Builds and writes a rolled segment's closing run (ADR 0067).
    Sealer,
    /// Catches the log key filter up over the sealed segments (ADR 0051).
    KeyFilter,
}

/// What one step did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// The task did work and may have more.
    Worked,
    /// The task has nothing to do until the caller appends, closes or seals, or, for the writer,
    /// until the clock's monotonic reading reaches `until`, the end of a group's linger.
    Idle { until: Option<Duration> },
    /// The writer has stopped. Every later step of every task returns the same value.
    Stopped(Stop),
}

/// A segment log whose writer, sealer and key filter builder run only as steps that the caller
/// schedules, on the caller's thread. Every storage call and clock reading happens inside a call
/// to it. Dropping it makes no storage call: it is a crash. See api.md.
pub struct SteppedLog {
    view: Arc<View>,
    admission: Admission,
    /// `None` for a log that open found sealed.
    machine: Option<Machine>,
    lock: Option<Box<dyn Lock>>,
    /// Set across a step, so that a step that panicked leaves the log stopped.
    stepping: bool,
}

impl SteppedLog {
    /// Opens or creates the log as `SegmentLog::open` does, on the caller's thread.
    pub fn open(
        dir: impl AsRef<Path>,
        options: Options,
        storage: Arc<dyn Storage>,
        clock: Arc<dyn Clock>,
    ) -> Result<Self, OpenError> {
        let modes = Modes {
            log_filter: LogFilterMode::Task,
            sealer: SealMode::Task,
            stepped: true,
        };
        let prepared = prepare(
            dir.as_ref(),
            options,
            storage,
            clock,
            KeyHash::new,
            Limits::default(),
            modes,
        )?;
        let (machine, lock) = prepared.writing.unzip();
        Ok(Self {
            view: prepared.view,
            admission: prepared.admission,
            machine,
            lock,
            stepping: false,
        })
    }

    pub fn append(&self, submit: Submit) -> Result<Pending, AppendError> {
        self.admission.append(submit)
    }

    pub fn watermark(&self) -> LogOffset {
        self.view.watermark()
    }

    pub fn stream_head(&self, stream: &[u8]) -> Option<StreamHead> {
        self.view.stream_head(stream)
    }

    pub fn read(&self, from: LogOffset, max: usize) -> Result<Vec<Record>, ReadError> {
        self.view.read(from, max)
    }

    /// Runs one step of `task`.
    pub fn step(&mut self, task: Task) -> Step {
        let Some(machine) = &mut self.machine else {
            return Step::Stopped(Stop::Sealed);
        };
        if self.stepping {
            return Step::Stopped(Stop::Poisoned);
        }
        if let Some(stop) = machine.stopped {
            return Step::Stopped(stop);
        }
        self.stepping = true;
        let _panic = writer::PoisonOnPanic(&self.admission.poisoned);
        let idle = Step::Idle { until: None };
        let step = match task {
            Task::Writer => match machine.step() {
                Progress::Worked => Step::Worked,
                Progress::Idle(until) => Step::Idle { until },
                Progress::Stopped(stop) => Step::Stopped(stop),
            },
            Task::Sealer if machine.writer.run_sealer() => Step::Worked,
            Task::KeyFilter if machine.writer.keys.step_builder() => Step::Worked,
            Task::Sealer | Task::KeyFilter => idle,
        };
        self.stepping = false;
        // A sealed log writes nothing more, so it holds no lock (I8).
        if step == Step::Stopped(Stop::Sealed) {
            self.lock = None;
        }
        step
    }

    /// Closes admission and marks the log for sealing. It returns at once. The writer seals once
    /// every admitted submit is taken, so later writer steps end with `Stopped(Sealed)` or
    /// `Stopped(Poisoned)`. It does nothing after `close`.
    pub fn seal(&mut self) {
        let gate = std::mem::replace(&mut *self.admission.gate.write().unwrap(), Gate::Closed);
        if let (Gate::Open(_), Some(machine)) = (gate, &mut self.machine) {
            machine.seal_requested = true;
        }
    }

    /// Closes admission as a drop of `SegmentLog` does. Later writer steps drain the queue, write
    /// a pending closing run and the close frame, and end with `Stopped(Closed)`. It does nothing
    /// after `seal`.
    pub fn close(&mut self) {
        *self.admission.gate.write().unwrap() = Gate::Closed;
    }
}
