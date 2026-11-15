//! When the next frame may go to the client.
//!
//! The client sends back the timestamp of each `sync` when it has drawn that frame. While
//! it has not answered the latest frames, it is behind: the link is slow or the browser is
//! busy. A frame sent then would only wait in a queue and arrive old. So the gateway holds
//! the frame back; the screen copy goes on collecting changes, and the frame that goes out
//! next holds only the newest pixels.
//!
//! Pure: the caller gives the time, counted from the start of the session.

use std::collections::VecDeque;
use std::time::Duration;

/// The limits of one session's frames.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pace {
    /// The shortest time from one frame to the next: the frame rate limit.
    pub least_interval: Duration,
    /// How many frames may be on their way to the client and not yet answered.
    pub most_unanswered: usize,
}

impl Default for Pace {
    /// At most 30 frames each second, and two frames on their way: one that the client
    /// draws and one behind it, so a fast client never waits for the gateway.
    fn default() -> Self {
        Self {
            least_interval: Duration::from_millis(33),
            most_unanswered: 2,
        }
    }
}

/// When a frame may be sent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Next {
    /// Now.
    Now,
    /// Not before this time.
    At(Duration),
    /// When the client answers a frame it has.
    AfterAnswer,
}

/// The frames of one session that the client has not answered yet.
#[derive(Debug)]
pub struct Pacing {
    pace: Pace,
    /// The timestamps of the unanswered frames, oldest first.
    unanswered: VecDeque<u64>,
    /// When the latest frame was sent, and its timestamp.
    latest: Option<(Duration, u64)>,
}

impl Pacing {
    #[must_use]
    pub const fn new(pace: Pace) -> Self {
        Self {
            pace,
            unanswered: VecDeque::new(),
            latest: None,
        }
    }

    /// When the next frame may be sent, at time `now`.
    #[must_use]
    pub fn next(&self, now: Duration) -> Next {
        if self.unanswered.len() >= self.pace.most_unanswered.max(1) {
            return Next::AfterAnswer;
        }
        match self.latest {
            Some((sent, _)) if now < sent + self.pace.least_interval => {
                Next::At(sent + self.pace.least_interval)
            }
            _ => Next::Now,
        }
    }

    /// Notes a frame sent at `now` and gives the timestamp for its `sync`: the time in
    /// milliseconds, and always more than the timestamp before it.
    pub fn sent(&mut self, now: Duration) -> u64 {
        let millis = u64::try_from(now.as_millis()).unwrap_or(u64::MAX);
        let timestamp = match self.latest {
            Some((_, before)) => millis.max(before.saturating_add(1)),
            None => millis,
        };
        self.latest = Some((now, timestamp));
        self.unanswered.push_back(timestamp);
        timestamp
    }

    /// The client's `sync` with `timestamp`: it has drawn that frame and every frame
    /// before it. Text that is not a timestamp answers nothing.
    pub fn answered(&mut self, timestamp: &str) {
        let Ok(drawn) = timestamp.parse::<u64>() else {
            return;
        };
        while self.unanswered.front().is_some_and(|sent| *sent <= drawn) {
            self.unanswered.pop_front();
        }
    }
}
