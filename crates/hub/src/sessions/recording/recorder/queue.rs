//! The parts between the relay and the disk (K4 §11), pure: time is a parameter (CQ-021).
//!
//! The relay pushes bytes into the open part; a part is ready when it holds 64 KiB, or 5
//! seconds after its first byte. At most 64 ready parts wait for the writer; a push that would
//! need a 65th stops the recording with `write_failed`, so the relay never waits on the disk.
//! A push is kept whole or dropped whole, so a recording never holds half of what the relay
//! handed over.

use std::{collections::VecDeque, time::Duration};

use tokio::time::Instant;
use zeroize::Zeroizing;

use crate::sessions::recording::{format::MAX_PART, model::StopReason};

/// How many ready parts may wait for the writer (K4 §11).
pub(in crate::sessions::recording) const QUEUE_PARTS: usize = 64;

/// How long the open part waits after its first byte before it is written (K4 §11).
pub(in crate::sessions::recording) const FLUSH_AFTER: Duration = Duration::from_secs(5);

/// Plaintext held in memory; wiped when dropped.
pub(in crate::sessions::recording) type Plain = Zeroizing<Vec<u8>>;

/// What [`Queue::push`] did with the bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::sessions::recording) enum Pushed {
    /// Kept; `wake` when the writer has something new to wait for or write.
    Kept { wake: bool },
    /// This push stopped the recording; the bytes were dropped.
    Stopped(StopReason),
    /// The recording had already stopped or is closing; the bytes were dropped.
    Dropped,
}

/// What the writer writes next.
pub(in crate::sessions::recording) enum Job {
    /// A part before the last.
    Part(Plain),
    /// The last part, with `tail` (possibly empty); then the recording ends.
    Last {
        tail: Plain,
        stopped: Option<StopReason>,
    },
}

/// The writer's next step.
pub(in crate::sessions::recording) enum Next {
    Write(Job),
    /// Nothing to write: wait for a push, or until the open part is due.
    Wait(Option<Instant>),
}

pub(in crate::sessions::recording) struct Queue {
    ready: VecDeque<Plain>,
    open: Plain,
    /// When the open part took its first byte.
    opened_at: Option<Instant>,
    /// Plaintext kept so far, against `max_bytes`.
    kept: u64,
    max_bytes: u64,
    stopped: Option<StopReason>,
    closing: bool,
}

impl Queue {
    pub(in crate::sessions::recording) fn new(max_bytes: u64) -> Self {
        Self {
            ready: VecDeque::with_capacity(QUEUE_PARTS),
            open: empty_part(),
            opened_at: None,
            kept: 0,
            max_bytes,
            stopped: None,
            closing: false,
        }
    }

    /// Keeps `bytes`, received at `now`, or stops the recording when they would pass the size
    /// limit or need more than [`QUEUE_PARTS`] ready parts.
    pub(in crate::sessions::recording) fn push(&mut self, bytes: &[u8], now: Instant) -> Pushed {
        if self.stopped.is_some() || self.closing {
            return Pushed::Dropped;
        }
        let length = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
        if self.kept.saturating_add(length) > self.max_bytes {
            return self.stop_at(StopReason::SizeLimit);
        }
        let completes = (self.open.len() + bytes.len()) / MAX_PART;
        if self.ready.len() + completes > QUEUE_PARTS {
            return self.stop_at(StopReason::WriteFailed);
        }
        let was_empty = self.open.is_empty();
        let mut rest = bytes;
        while !rest.is_empty() {
            let take = rest.len().min(MAX_PART - self.open.len());
            let (head, tail) = rest.split_at(take);
            if self.open.is_empty() {
                self.opened_at = Some(now);
            }
            self.open.extend_from_slice(head);
            rest = tail;
            if self.open.len() == MAX_PART {
                self.ready
                    .push_back(std::mem::replace(&mut self.open, empty_part()));
                self.opened_at = None;
            }
        }
        self.kept += length;
        Pushed::Kept {
            wake: completes > 0 || (was_empty && !self.open.is_empty()),
        }
    }

    /// The writer's next job at `now`: ready parts in order, then the last part once the
    /// recording stopped or is closing, else the open part once it is due.
    pub(in crate::sessions::recording) fn next(&mut self, now: Instant) -> Next {
        if let Some(part) = self.ready.pop_front() {
            return Next::Write(Job::Part(part));
        }
        if self.stopped.is_some() || self.closing {
            self.closing = true;
            return Next::Write(Job::Last {
                tail: std::mem::replace(&mut self.open, empty_part()),
                stopped: self.stopped,
            });
        }
        match self.opened_at {
            Some(opened) if now >= opened + FLUSH_AFTER => {
                self.opened_at = None;
                Next::Write(Job::Part(std::mem::replace(&mut self.open, empty_part())))
            }
            due => Next::Wait(due.map(|opened| opened + FLUSH_AFTER)),
        }
    }

    /// The session ends: nothing more is kept, and the writer ends the recording.
    pub(in crate::sessions::recording) const fn close(&mut self) {
        self.closing = true;
    }

    /// The disk failed: the bytes not yet written are dropped, and the writer writes only the
    /// last part. A recording stops once; the first reason stays.
    pub(in crate::sessions::recording) fn fail(&mut self, reason: StopReason) {
        if self.stopped.is_none() {
            self.stopped = Some(reason);
        }
        self.ready.clear();
        self.open = empty_part();
        self.opened_at = None;
    }

    const fn stop_at(&mut self, reason: StopReason) -> Pushed {
        self.stopped = Some(reason);
        Pushed::Stopped(reason)
    }
}

/// An empty part with room for 64 KiB, so filling it never moves the plaintext.
fn empty_part() -> Plain {
    Zeroizing::new(Vec::with_capacity(MAX_PART))
}
