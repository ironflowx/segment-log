use super::*;
use closing::Tail;
use keys::Classification;

pub(super) struct Writer {
    pub epoch: u64,
    /// The newest group's append time. The next group never gets a lower one, even when the
    /// wall clock steps back (ADR 0064, rule 2).
    pub append_time: u64,
    pub keys: Keys,
    pub active: Active,
    pub limits: Limits,
    pub sealer: SealMode,
    /// The rolled segment whose closing run is not durable yet (ADR 0061, ADR 0067).
    pub pending: Option<Pending>,
    #[cfg(test)]
    pub memory: Arc<Mutex<KeyMemory>>,
    #[cfg(test)]
    pub rolls: Arc<Mutex<Vec<std::time::Duration>>>,
}

/// Whatever ends the writer, even poison, waits for a sealer still writing a closing run: the
/// log joins the writer before it releases the directory lock, so no write outlives the lock.
impl Drop for Writer {
    fn drop(&mut self) {
        if let Some(Pending {
            sealing: Sealing::Running(sealer),
            ..
        }) = self.pending.take()
        {
            let _ = sealer.join();
        }
    }
}

pub(super) struct Active {
    pub base: u64,
    pub length: u64,
    /// Whether the segment's last frame is a close frame, so a drop has nothing to add.
    pub ends_with_close: bool,
    tails: BTreeMap<Vec<u8>, Tail>,
    footer_body: usize,
}

impl Active {
    pub(super) fn new(base: u64) -> Self {
        Self::recovered(base, codec::HEADER_LEN as u64, false, BTreeMap::new())
    }

    pub(super) fn recovered(
        base: u64,
        length: u64,
        ends_with_close: bool,
        tails: BTreeMap<Vec<u8>, Tail>,
    ) -> Self {
        let footer_body = tails
            .keys()
            .map(|stream| footer_entry(stream))
            .sum::<usize>();
        Self {
            base,
            length,
            ends_with_close,
            tails,
            footer_body: codec::FOOTER_HEADER_LEN + footer_body,
        }
    }
}

pub(super) struct Pending {
    base: u64,
    sealing: Sealing,
}

/// Where a roll's closing run is built and written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SealMode {
    /// On a sealer thread (ADR 0061, ADR 0067).
    Thread,
    /// Built on the writer at roll and written at the next group boundary. Tests use it, so that
    /// traces stay the same from run to run.
    #[cfg_attr(not(test), allow(dead_code))]
    Inline,
    /// As a sealer step that the caller schedules (ADR 0072).
    #[cfg_attr(not(any(test, feature = "simulation")), allow(dead_code))]
    Task,
}

type SealJob = Box<dyn FnOnce() -> io::Result<Built> + Send>;

enum Sealing {
    Built(Built),
    Running(thread::JoinHandle<io::Result<Built>>),
    /// Waits for a sealer step.
    Queued(SealJob),
    /// A sealer step has run.
    Done(io::Result<Built>),
}

struct Built {
    /// `None` once the sealer has written and flushed the run itself (ADR 0067).
    run: Option<[Vec<u8>; 4]>,
    keys: keys::SealedKeys,
}

/// Writes a rolled segment's closing run. The key frames are durable before the footer and the
/// locator exist, so that a valid locator and footer prove the whole run to an open that reads
/// only them (overview.md, open step 5).
fn write_run(storage: &dyn Storage, path: &Path, run: [Vec<u8>; 4]) -> io::Result<()> {
    let [filter, index, footer, locator] = run;
    for frames in [[filter, index], [footer, locator]] {
        for frame in frames {
            call(storage, path, Purpose::Roll, Op::Append(frame))?;
        }
        call(storage, path, Purpose::Roll, Op::SyncData)?;
    }
    Ok(())
}

/// The sealer's work: every step is O(keys in the segment), so none of it runs on the writer
/// unless tests ask for that. Dropping `tails` here frees them off the writer too.
fn build(
    log_id: codec::LogId,
    hash: keys::KeyHash,
    epoch: u64,
    summary: (u64, u64, u64),
    tails: BTreeMap<Vec<u8>, Tail>,
    keys: Arc<keys::KeyMap>,
) -> Built {
    let (base, next, filter_at) = summary;
    let mut entries = hash.entries(&keys);
    let run = closing::roll_run(&log_id, base, epoch, next, filter_at, &tails, &mut entries);
    let summary = keys::SealedSummary {
        base,
        next,
        filter_at,
        footer_at: filter_at + (run[0].len() + run[1].len()) as u64,
    };
    let keys = keys::SealedKeys::written(summary, &log_id, &run[0]);
    Built {
        run: Some(run),
        keys,
    }
}

fn footer_entry(stream: &[u8]) -> usize {
    codec::FOOTER_ENTRY_LEN + stream.len()
}

impl Writer {
    fn fits(&self, draft: &Draft, submit: &Submit) -> bool {
        let mut streams: Vec<&[u8]> = submit
            .records
            .iter()
            .map(|record| record.stream.as_slice())
            .filter(|stream| !self.active.tails.contains_key(*stream))
            .filter(|stream| !draft.tails.contains_key(*stream))
            .collect();
        streams.sort_unstable();
        streams.dedup();
        let footer = self.active.footer_body
            + draft.footer_growth
            + streams.into_iter().map(footer_entry).sum::<usize>();
        let keyed = submit.records.iter().filter(|r| r.key.is_some()).count();
        footer <= self.limits.footer_body && self.keys.keyed() + keyed <= self.limits.keyed_records
    }

    fn rolls_before(&self, next: u64, fits: bool, options: &Options) -> bool {
        // A segment with no record cannot roll: its successor would take its name.
        (self.active.length >= options.roll_bytes || !fits) && next != self.active.base
    }

    /// Hands the segment's closing run to the sealer and starts the next segment. The sealer
    /// writes and flushes the run (ADR 0067); inline, the writer writes it later
    /// (`write_pending`). Nobody appends a group to the old segment again.
    fn roll(&mut self, view: &View, next: u64) -> io::Result<()> {
        #[cfg(test)]
        let started = crate::pipeline::clock();
        // At most one closing run pends, so recovery has one case to rebuild (ADR 0061, rule 4).
        self.write_pending(view, true)?;
        #[cfg(test)]
        let waited = crate::pipeline::clock();
        let base = self.active.base;
        let filter_at = self.active.length;
        let keys = self.keys.roll(base, next, filter_at);
        let tails = std::mem::take(&mut self.active.tails);
        let (log_id, hash, epoch) = (view.log_id.clone(), self.keys.hash(), self.epoch);
        let job = move || build(log_id, hash, epoch, (base, next, filter_at), tails, keys);
        let header = codec::Header {
            version: self.limits.format_version,
            log_id: *view.log_id.bytes(),
            base_offset: next,
            epoch: self.epoch,
        };
        create_segment(view.storage.as_ref(), &view.dir, Purpose::Roll, &header)?;
        let sealing = match self.sealer {
            SealMode::Inline => Sealing::Built(job()),
            mode => {
                // The next segment is durable first, so the old one is never sealed with no
                // successor. From here only the sealer writes the old segment, and only the
                // writer the new one.
                let (storage, path) = (view.storage.clone(), segment_path(&view.dir, base));
                let write = move || {
                    let mut built = job();
                    if let Some(run) = built.run.take() {
                        write_run(storage.as_ref(), &path, run)?;
                    }
                    Ok(built)
                };
                if mode == SealMode::Thread {
                    Sealing::Running(
                        thread::Builder::new()
                            .name("segment-log-sealer".into())
                            .spawn(write)?,
                    )
                } else {
                    Sealing::Queued(Box::new(write))
                }
            }
        };
        self.pending = Some(Pending { base, sealing });
        let spans = view.published.lock().unwrap().roll(self.epoch, self.epoch);
        view.tables.lock().unwrap().keep(base, spans);
        self.active = Active::new(next);
        #[cfg(test)]
        crate::pipeline::event("roll", [started, waited, crate::pipeline::clock()]);
        Ok(())
    }

    /// Ends the pending roll once the sealer has made its closing run durable, or, with `wait`,
    /// after waiting for it; an inline sealer's run the writer writes here. A sealer that
    /// panicked or failed to write counts as a failed roll, and nothing is retried (ADR 0004).
    fn write_pending(&mut self, view: &View, wait: bool) -> io::Result<()> {
        let Some(Pending { base, sealing }) = self.pending.take() else {
            return Ok(());
        };
        #[cfg(test)]
        let started = crate::pipeline::clock();
        let built = match sealing {
            Sealing::Built(built) => built,
            Sealing::Running(sealer) if wait || sealer.is_finished() => sealer
                .join()
                .map_err(|_| io::Error::other("the sealer panicked"))??,
            // Waiting for a sealer step is running it, because a waiting thread lets it finish.
            Sealing::Queued(job) if wait => job()?,
            Sealing::Done(result) => result?,
            sealing @ (Sealing::Running(_) | Sealing::Queued(_)) => {
                self.pending = Some(Pending { base, sealing });
                return Ok(());
            }
        };
        if let Some(run) = built.run {
            write_run(view.storage.as_ref(), &segment_path(&view.dir, base), run)?;
        }
        #[cfg(test)]
        let written = crate::pipeline::clock();
        if let Some(keys) = self.keys.sealed(built.keys) {
            // The map's few buffers are megabytes, whose pages take time to give back.
            if self.sealer == SealMode::Thread {
                let _ = thread::Builder::new()
                    .name("segment-log-sealer".into())
                    .spawn(move || drop(keys));
            }
        }
        #[cfg(test)]
        crate::pipeline::event("pending", [started, written, crate::pipeline::clock()]);
        Ok(())
    }

    /// A sealer step: builds and writes the queued closing run. Reports whether there was one.
    #[cfg_attr(not(any(test, feature = "simulation")), allow(dead_code))]
    pub(super) fn run_sealer(&mut self) -> bool {
        let Some(pending) = &mut self.pending else {
            return false;
        };
        if !matches!(pending.sealing, Sealing::Queued(_)) {
            return false;
        }
        let Sealing::Queued(job) = std::mem::replace(
            &mut pending.sealing,
            Sealing::Done(Err(io::ErrorKind::Interrupted.into())),
        ) else {
            unreachable!()
        };
        pending.sealing = Sealing::Done(job());
        true
    }

    /// Appends a close frame and syncs it, once every group before it is durable (ADR 0071).
    fn close(&mut self, view: &View) -> io::Result<()> {
        // The build's version is the active segment's: open rolls away from an older active
        // segment before the writer starts (ADR 0064, rule 12).
        if self.active.ends_with_close || self.limits.format_version < codec::CLOSE_VERSION {
            return Ok(());
        }
        let path = segment_path(&view.dir, self.active.base);
        let frame = codec::encode_close(&view.log_id, self.epoch, view.watermark());
        let length = frame.len() as u64;
        call(
            view.storage.as_ref(),
            &path,
            Purpose::Close,
            Op::Append(frame),
        )?;
        call(view.storage.as_ref(), &path, Purpose::Close, Op::SyncData)?;
        self.active.length += length;
        self.active.ends_with_close = true;
        Ok(())
    }

    fn seal(&mut self, view: &View) -> io::Result<()> {
        self.write_pending(view, true)?;
        let storage = view.storage.as_ref();
        let path = segment_path(&view.dir, self.active.base);
        let run = closing::seal_run(
            &view.log_id,
            self.epoch,
            view.watermark(),
            self.active.length,
            &self.active.tails,
        );
        for frame in run {
            call(storage, &path, Purpose::Seal, Op::Append(frame))?;
        }
        call(storage, &path, Purpose::Seal, Op::SyncData)?;
        Ok(())
    }
}

pub(super) struct RevisionSlot {
    pub index: usize,
    pub revision: Revision,
}
// Classification reuses the result-position allowance before any result is allocated.
const _: () = assert!(std::mem::size_of::<RevisionSlot>() <= std::mem::size_of::<Position>());
/// `pending` holds the revisions that the current group has assigned. It overrides the
/// durable heads, which the group has not changed yet.
fn revisions(
    submit: &Submit,
    pending: &BTreeMap<Vec<u8>, Tail>,
    published: &Mutex<Published>,
) -> Result<Box<[RevisionSlot]>, CommitError> {
    let mut slots: Box<[_]> = (0..submit.records.len())
        .map(|index| RevisionSlot { index, revision: 0 })
        .collect();
    slots.sort_unstable_by(|a, b| {
        submit.records[a.index]
            .stream
            .cmp(&submit.records[b.index].stream)
            .then(a.index.cmp(&b.index))
    });
    let mut cursor = 0;
    let mut failure: Option<(usize, CommitError)> = None;
    // Only the writer thread changes the durable heads, and it does so between groups.
    let durable = published.lock().unwrap();
    while cursor < slots.len() {
        let stream = &submit.records[slots[cursor].index].stream;
        let mut actual = pending
            .get(stream)
            .map(|tail| tail.revision)
            .or_else(|| durable.heads.get(stream).map(|head| head.revision));
        while cursor < slots.len() && submit.records[slots[cursor].index].stream == *stream {
            let index = slots[cursor].index;
            let matches = match submit.records[index].expected {
                ExpectedRevision::Empty => actual.is_none(),
                ExpectedRevision::At(n) => actual == Some(n),
            };
            let result = if !matches {
                Err(CommitError::WrongRevision { index, actual })
            } else {
                match actual {
                    None => Ok(0),
                    Some(n) => n.checked_add(1).ok_or(CommitError::Exhausted),
                }
            };
            let revision = match result {
                Ok(revision) => revision,
                Err(error) => {
                    if failure.as_ref().is_none_or(|(earlier, _)| index < *earlier) {
                        failure = Some((index, error));
                    }
                    while cursor < slots.len()
                        && submit.records[slots[cursor].index].stream == *stream
                    {
                        cursor += 1;
                    }
                    break;
                }
            };
            slots[cursor].revision = revision;
            actual = Some(revision);
            cursor += 1;
        }
    }
    if let Some((_, error)) = failure {
        return Err(error);
    }
    slots.sort_unstable_by_key(|slot| slot.index);
    Ok(slots)
}

struct Draft {
    base: u64,
    requests: Vec<Request>,
    /// Request and record index of each staged record, by offset past `base`.
    origins: Vec<(usize, usize)>,
    assigned: Vec<Revision>,
    tails: BTreeMap<Vec<u8>, Tail>,
    footer_growth: usize,
    waiters: Vec<RetryWaiter>,
    size: usize,
}

impl Draft {
    fn new(base: u64) -> Self {
        Self {
            base,
            requests: Vec::new(),
            origins: Vec::new(),
            assigned: Vec::new(),
            // Copying every durable head here would make each group cost grow with the stream count.
            tails: BTreeMap::new(),
            footer_growth: 0,
            waiters: Vec::new(),
            size: codec::GROUP_HEADER_LEN,
        }
    }

    fn staged(&self, position: u64) -> keys::Staged<'_> {
        keys::Staged {
            position,
            base: self.base,
            requests: &self.requests,
            origins: &self.origins,
            assigned: &self.assigned,
        }
    }
}

impl Writer {
    fn stage(&mut self, draft: &mut Draft, request: Request, revisions: &[RevisionSlot]) {
        let submit = &request.submit;
        let first = draft.base + draft.assigned.len() as u64;
        self.keys
            .stage(submit, self.active.length, draft.assigned.len());
        draft
            .origins
            .extend((0..submit.records.len()).map(|record| (draft.requests.len(), record)));
        draft.size += body_size(submit).unwrap() - codec::GROUP_HEADER_LEN;
        for (i, (record, revision)) in submit.records.iter().zip(revisions).enumerate() {
            if !self.active.tails.contains_key(&record.stream)
                && !draft.tails.contains_key(&record.stream)
            {
                draft.footer_growth += footer_entry(&record.stream);
            }
            draft.tails.insert(
                record.stream.clone(),
                Tail {
                    revision: revision.revision,
                    offset: first + i as u64,
                    position: self.active.length,
                },
            );
        }
        draft
            .assigned
            .extend(revisions.iter().map(|slot| slot.revision));
        #[cfg(test)]
        crate::pipeline::request(submit, crate::pipeline::READY);
        draft.requests.push(request);
    }
}

/// A dequeued seal marker ends the gather, because nothing is queued behind it.
fn dequeued(message: Option<Message>, seal: &mut Option<mpsc::Sender<()>>) -> Option<Request> {
    match message? {
        Message::Append(request) => Some(request),
        Message::Seal(reply) => {
            *seal = Some(reply);
            None
        }
    }
}

/// Marks the writer poisoned when a step panics (api.md, admission step 3).
pub(super) struct PoisonOnPanic<'a>(pub &'a std::sync::atomic::AtomicBool);

impl Drop for PoisonOnPanic<'_> {
    fn drop(&mut self) {
        if std::thread::panicking() {
            self.0.store(true, std::sync::atomic::Ordering::Release);
        }
    }
}

/// How the writer is ended (ADR 0072).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stop {
    /// A clean drop, or `SteppedLog::close`, finished: the queue is drained and the close frame
    /// written (ADR 0071).
    Closed,
    Sealed,
    Poisoned,
}

pub(super) enum Progress {
    Worked,
    /// Nothing to do until a request arrives or admission closes, or until the monotonic clock
    /// reaches the deadline, the end of the gathering group's linger.
    Idle(#[cfg_attr(not(any(test, feature = "simulation")), allow(dead_code))] Option<Duration>),
    Stopped(Stop),
}

/// Where the writer thread would block. A stepped writer returns there instead (ADR 0072).
enum Wait {
    No,
    Forever,
    Until {
        deadline: Duration,
        remaining: Duration,
    },
}

enum Received {
    Message(Message),
    /// Nothing is queued now, or the wait timed out.
    Nothing,
    /// The queue is empty and admission is closed.
    Closed,
    /// A stepped writer would wait here.
    Blocked(Option<Duration>),
}

/// The group the writer is gathering between steps.
struct Gathering {
    draft: Draft,
    /// When the writer first saw this group's linger over.
    gathered: Option<Duration>,
    next: Option<Request>,
}

/// The writer loop as steps, so that a caller can run it without a thread (ADR 0072). Each step
/// ends where the thread would block, or after one group, a roll, a seal, a close or poison.
pub(super) struct Machine {
    pub view: Arc<View>,
    pub poisoned: Arc<std::sync::atomic::AtomicBool>,
    pub writer: Writer,
    receiver: mpsc::Receiver<Message>,
    options: Arc<Options>,
    clock: Arc<dyn Clock>,
    /// Without a thread, a step never blocks, and a seal is a mark instead of a message, because
    /// the queue's bound could leave no room for it.
    stepped: bool,
    pub seal_requested: bool,
    seal_delivered: bool,
    deferred: Option<Request>,
    seal: Option<mpsc::Sender<()>>,
    gathering: Option<Gathering>,
    pub stopped: Option<Stop>,
}

pub(super) fn run(mut machine: Machine) {
    // A local, so that unwinding drops it before the machine's receiver: admission sees the flag
    // before it can see the disconnected queue.
    let poisoned = machine.poisoned.clone();
    let _panic = PoisonOnPanic(&poisoned);
    while !matches!(machine.step(), Progress::Stopped(_)) {}
}

impl Machine {
    pub(super) fn new(
        view: Arc<View>,
        poisoned: Arc<std::sync::atomic::AtomicBool>,
        writer: Writer,
        receiver: mpsc::Receiver<Message>,
        options: Arc<Options>,
        clock: Arc<dyn Clock>,
        stepped: bool,
    ) -> Self {
        Self {
            view,
            poisoned,
            writer,
            receiver,
            options,
            clock,
            stepped,
            seal_requested: false,
            seal_delivered: false,
            deferred: None,
            seal: None,
            gathering: None,
            stopped: None,
        }
    }

    pub(super) fn step(&mut self) -> Progress {
        if let Some(stop) = self.stopped {
            return Progress::Stopped(stop);
        }
        let progress = self.advance();
        if let Progress::Stopped(stop) = progress {
            self.stopped = Some(stop);
        }
        progress
    }

    fn receive(&mut self, wait: Wait) -> Received {
        use mpsc::{RecvTimeoutError, TryRecvError};
        if !self.stepped {
            return match wait {
                Wait::No => match self.receiver.try_recv() {
                    Ok(message) => Received::Message(message),
                    Err(TryRecvError::Empty) => Received::Nothing,
                    Err(TryRecvError::Disconnected) => Received::Closed,
                },
                Wait::Forever => self
                    .receiver
                    .recv()
                    .map_or(Received::Closed, Received::Message),
                Wait::Until { remaining, .. } => match self.receiver.recv_timeout(remaining) {
                    Ok(message) => Received::Message(message),
                    Err(RecvTimeoutError::Timeout) => Received::Nothing,
                    Err(RecvTimeoutError::Disconnected) => Received::Closed,
                },
            };
        }
        match self.receiver.try_recv() {
            Ok(message) => Received::Message(message),
            Err(TryRecvError::Empty) => match wait {
                Wait::No => Received::Nothing,
                Wait::Forever => Received::Blocked(None),
                Wait::Until { deadline, .. } => Received::Blocked(Some(deadline)),
            },
            // The mark stands behind every admitted request, as the seal message does.
            Err(TryRecvError::Disconnected) if self.seal_requested && !self.seal_delivered => {
                self.seal_delivered = true;
                Received::Message(Message::Seal(mpsc::channel().0))
            }
            Err(TryRecvError::Disconnected) => Received::Closed,
        }
    }

    fn advance(&mut self) -> Progress {
        let mut gathering = match self.gathering.take() {
            Some(gathering) => gathering,
            None => {
                self.writer.keys.refresh();
                let first = match self.deferred.take() {
                    Some(request) => request,
                    None => {
                        if let Some(reply) = self.seal.take() {
                            if self.writer.seal(&self.view).is_ok() {
                                let _ = reply.send(());
                                return Progress::Stopped(Stop::Sealed);
                            }
                            self.poisoned
                                .store(true, std::sync::atomic::Ordering::Release);
                            return Progress::Stopped(Stop::Poisoned);
                        }
                        let message = match self.receive(Wait::Forever) {
                            Received::Message(message) => message,
                            Received::Closed => return self.shut_down(),
                            Received::Blocked(until) => return Progress::Idle(until),
                            Received::Nothing => return Progress::Idle(None),
                        };
                        match dequeued(Some(message), &mut self.seal) {
                            Some(request) => request,
                            None => return Progress::Worked,
                        }
                    }
                };
                #[cfg(test)]
                crate::pipeline::request(&first.submit, crate::pipeline::DEQUEUE);
                Gathering {
                    draft: Draft::new(self.view.watermark()),
                    gathered: None,
                    next: Some(first),
                }
            }
        };
        loop {
            if let Some(request) = gathering.next.take() {
                #[cfg(test)]
                crate::pipeline::request(&request.submit, crate::pipeline::CLASSIFY);
                match self.take(&mut gathering.draft, request) {
                    Taken::Staged => {}
                    Taken::Deferred => break,
                    Taken::Rolled => {
                        // A roll ends a step, so that a sealer step can come before the group.
                        self.gathering = Some(gathering);
                        return Progress::Worked;
                    }
                    Taken::Stopped(progress) => return progress,
                }
            }
            let first = gathering.draft.requests.first().map(|r| r.admitted);
            gathering.next = match first {
                _ if self.seal.is_some() => None,
                // No new submit has joined yet, so no linger runs.
                None => match self.receive(Wait::No) {
                    Received::Message(message) => dequeued(Some(message), &mut self.seal),
                    _ => None,
                },
                Some(admitted) => {
                    // The linger runs from the admission of the group's first submit, so the time
                    // that submit spent queued behind the previous flush counts (ADR 0046).
                    let now = self.clock.monotonic();
                    let remaining = self
                        .options
                        .linger
                        .saturating_sub(now.saturating_sub(admitted));
                    if remaining.is_zero() {
                        // The group takes the submits admitted before the writer first saw the
                        // linger over. Without that limit, steady arrivals keep one group
                        // gathering until the byte cap.
                        let limit = *gathering.gathered.get_or_insert(now);
                        match self.receive(Wait::No) {
                            Received::Message(message) => {
                                match dequeued(Some(message), &mut self.seal) {
                                    Some(late) if late.admitted > limit => {
                                        self.deferred = Some(late);
                                        None
                                    }
                                    queued => queued,
                                }
                            }
                            _ => None,
                        }
                    } else {
                        let deadline = admitted.saturating_add(self.options.linger);
                        match self.receive(Wait::Until {
                            deadline,
                            remaining,
                        }) {
                            Received::Message(message) => dequeued(Some(message), &mut self.seal),
                            Received::Blocked(until) => {
                                self.gathering = Some(gathering);
                                return Progress::Idle(until);
                            }
                            Received::Nothing | Received::Closed => None,
                        }
                    }
                }
            };
            #[cfg(test)]
            if let Some(request) = &gathering.next {
                crate::pipeline::request(&request.submit, crate::pipeline::DEQUEUE);
            }
            if gathering.next.is_none() {
                break;
            }
        }
        let mut draft = gathering.draft;
        if draft.requests.is_empty() {
            return Progress::Worked;
        }
        // The group's frame waits for no sealer: an unfinished closing run waits for a later group.
        if self.writer.write_pending(&self.view, false).is_err() {
            let deferred = self.deferred.take();
            return self.poison(draft, deferred, true);
        }
        if self
            .writer
            .commit(&self.view, self.clock.as_ref(), &mut draft)
            .is_err()
        {
            let deferred = self.deferred.take();
            return self.poison(draft, deferred, false);
        }
        Progress::Worked
    }

    /// Classifies one request into the draft.
    fn take(&mut self, draft: &mut Draft, request: Request) -> Taken {
        let options = self.options.clone();
        match self.writer.keys.classify(
            &self.view,
            &request.submit,
            &draft.staged(self.writer.active.length),
        ) {
            Classification::New => {
                match revisions(&request.submit, &draft.tails, &self.view.published) {
                    Err(error) => {
                        request.answer(Err(error));
                    }
                    Ok(revisions) => {
                        let added = body_size(&request.submit).unwrap() - codec::GROUP_HEADER_LEN;
                        let fits = self.writer.fits(draft, &request.submit);
                        if draft.size + added > options.group_byte_cap
                            || (!fits && !draft.requests.is_empty())
                        {
                            self.deferred = Some(request);
                            return Taken::Deferred;
                        }
                        if draft
                            .base
                            .checked_add(draft.assigned.len() as u64)
                            .and_then(|next| next.checked_add(request.submit.records.len() as u64))
                            .is_none()
                        {
                            request.answer(Err(CommitError::Exhausted));
                        } else {
                            // The roll waits for the group's first new submit, so that a group
                            // that appends nothing writes and flushes nothing.
                            let rolls = draft.requests.is_empty()
                                && self.writer.rolls_before(draft.base, fits, &options);
                            if rolls {
                                #[cfg(test)]
                                let started = std::time::Instant::now();
                                let rolled = self.writer.roll(&self.view, draft.base);
                                #[cfg(test)]
                                self.writer.rolls.lock().unwrap().push(started.elapsed());
                                if rolled.is_err() {
                                    let draft = std::mem::replace(draft, Draft::new(draft.base));
                                    return Taken::Stopped(self.poison(
                                        draft,
                                        Some(request),
                                        false,
                                    ));
                                }
                                debug_assert!(
                                    self.writer.fits(draft, &request.submit),
                                    "a submit under the group cap fits an empty segment's closing frames"
                                );
                            }
                            self.writer.stage(draft, request, &revisions);
                            if rolls {
                                return Taken::Rolled;
                            }
                        }
                    }
                }
            }
            Classification::DurableRetry(records) => {
                request.answer(Ok(Committed {
                    records,
                    deduplicated: true,
                }));
            }
            Classification::PendingRetry(positions) => {
                draft.waiters.push(RetryWaiter { request, positions });
            }
            Classification::Rejected(error) => {
                request.answer(Err(error));
            }
            Classification::LookupFailure => {
                draft.requests.push(request);
                let draft = std::mem::replace(draft, Draft::new(draft.base));
                let deferred = self.deferred.take();
                return Taken::Stopped(self.poison(draft, deferred, false));
            }
        }
        Taken::Staged
    }

    /// A dropped log leaves no closing run pending (ADR 0061, rule 7). Only then is every group
    /// synced, so only then may a close frame say so (ADR 0071). Nothing is retried after a
    /// failure (ADR 0004).
    fn shut_down(&mut self) -> Progress {
        let closed = self.writer.write_pending(&self.view, true).is_ok()
            && !self.poisoned.load(std::sync::atomic::Ordering::Acquire)
            && self.writer.close(&self.view).is_ok();
        if closed {
            return Progress::Stopped(Stop::Closed);
        }
        self.poisoned
            .store(true, std::sync::atomic::Ordering::Release);
        Progress::Stopped(Stop::Poisoned)
    }

    fn poison(&mut self, draft: Draft, current: Option<Request>, unwritten: bool) -> Progress {
        self.writer.keys.stop_building();
        poison(&self.poisoned, draft, current, unwritten);
        let refuse = |message| {
            if let Message::Append(request) = message {
                request.answer(Err(CommitError::Poisoned));
            }
        };
        // A thread answers until every sender is gone. Without one, admission already refuses,
        // so only what is queued is left.
        if self.stepped {
            self.receiver.try_iter().for_each(refuse);
        } else {
            self.receiver.iter().for_each(refuse);
        }
        Progress::Stopped(Stop::Poisoned)
    }
}

enum Taken {
    Staged,
    Deferred,
    Rolled,
    Stopped(Progress),
}

impl Writer {
    /// A failed write or flush leaves every request of the draft unanswered.
    fn commit(&mut self, view: &View, clock: &dyn Clock, draft: &mut Draft) -> io::Result<()> {
        let base = draft.base;
        #[cfg(test)]
        let mut timeline = crate::pipeline::Group::new(base, draft.requests.len());
        let entries = draft
            .requests
            .iter()
            .flat_map(|r| &r.submit.records)
            .zip(&draft.assigned)
            .map(|(r, revision)| codec::Entry {
                stream: &r.stream,
                key: r.key.as_deref(),
                payload: &r.payload,
                revision: *revision,
            })
            .collect();
        let group = codec::Group {
            epoch: self.epoch,
            base_offset: base,
            append_time: clock.wall_millis().max(self.append_time),
            entries,
        };
        let bytes = codec::encode_group(&view.log_id, &group).expect("validated group");
        let length = bytes.len();
        let segment = segment_path(&view.dir, self.active.base);
        #[cfg(test)]
        timeline.mark(1);
        let result = call(
            view.storage.as_ref(),
            &segment,
            Purpose::Group,
            Op::Append(bytes),
        )
        .and_then(|_| {
            #[cfg(test)]
            timeline.mark(2);
            call(
                view.storage.as_ref(),
                &segment,
                Purpose::Group,
                Op::SyncData,
            )
        });
        #[cfg(test)]
        timeline.mark(3);
        result?;
        self.append_time = group.append_time;
        let span = GroupSpan::of(self.active.length, length, &group);
        self.active.length += length as u64;
        self.active.ends_with_close = false;
        self.active.tails.append(&mut draft.tails);
        self.active.footer_body += std::mem::take(&mut draft.footer_growth);
        view.published.lock().unwrap().publish(span, &group);
        #[cfg(test)]
        timeline.mark(4);
        let mut index = 0;
        let mut slots = Vec::with_capacity(draft.requests.len() + draft.waiters.len());
        for request in std::mem::take(&mut draft.requests) {
            #[cfg(test)]
            crate::pipeline::group_id(&request.submit, base);
            let records = (0..request.submit.records.len())
                .map(|_| {
                    let p = Position {
                        offset: base + index as u64,
                        revision: draft.assigned[index],
                    };
                    index += 1;
                    p
                })
                .collect();
            slots.push(request.store(Ok(Committed {
                records,
                deduplicated: false,
            })));
        }
        for waiter in std::mem::take(&mut draft.waiters) {
            slots.push(waiter.request.store(Ok(Committed {
                records: waiter.positions,
                deduplicated: true,
            })));
        }
        #[cfg(test)]
        {
            *self.memory.lock().unwrap() = self.keys.memory();
        }
        reply::Relay::start(slots);
        #[cfg(test)]
        timeline.finish();
        Ok(())
    }
}

/// `unwritten`: the writer failed before it wrote any byte of the draft's group, so its
/// requests and waiters get `Poisoned` rather than `Indeterminate`.
fn poison(
    poisoned: &std::sync::atomic::AtomicBool,
    draft: Draft,
    current: Option<Request>,
    unwritten: bool,
) {
    poisoned.store(true, std::sync::atomic::Ordering::Release);
    let outcome = || {
        if unwritten {
            CommitError::Poisoned
        } else {
            CommitError::Indeterminate
        }
    };
    for request in draft.requests {
        request.answer(Err(outcome()));
    }
    for waiter in draft.waiters {
        waiter.request.answer(Err(outcome()));
    }
    if let Some(request) = current {
        request.answer(Err(CommitError::Poisoned));
    }
}
