//! The waits and the counts of the supervision of a server (Section 9, CAG-005, CAG-007). Pure:
//! the time is a parameter in whole seconds of a clock that only goes forward.

/// The waits before each restart, in seconds; after the last, each wait is [`LAST_WAIT`].
pub const WAITS: [u64; 9] = [1, 2, 4, 8, 16, 32, 64, 128, 256];
pub const LAST_WAIT: u64 = 300;

const HOUR: u64 = 3_600;
const DAY: u64 = 86_400;
/// The ends in one hour and in 24 hours after which the server stays `failed`.
pub const LIMIT_HOUR: usize = 5;
pub const LIMIT_DAY: usize = 20;
/// How long a server of a new `server_start` must stay `ready` to set the counts to 0.
pub const CLEAR_AFTER: u64 = 600;

/// What the daemon does after an end that no item asked for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Next {
    /// Start the server again at this time.
    RestartAt(u64),
    /// The server stays `failed` until a new `server_start`.
    Failed,
}

/// The record of the ends of one server.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Supervision {
    /// The times of the ends that no item asked for, in the last 24 hours.
    ends: Vec<u64>,
    failed: bool,
    /// A `server_start` came after the last clear; the counts go to 0 after 10 minutes ready.
    clear_pending: bool,
    ready_since: Option<u64>,
}

impl Supervision {
    /// The wait before restart number `index` (from 0).
    #[must_use]
    pub fn wait(index: usize) -> u64 {
        WAITS.get(index).copied().unwrap_or(LAST_WAIT)
    }

    /// A `server_start` for this server: it may run again, and the counts go to 0 when it then
    /// stays ready for 10 minutes.
    pub fn started(&mut self) {
        self.failed = false;
        self.clear_pending = true;
        self.ready_since = None;
    }

    /// The server is ready at `now`.
    pub fn ready(&mut self, now: u64) {
        self.ready_since.get_or_insert(now);
        self.tick(now);
    }

    /// Clears the counts when the server of a new `server_start` has been ready long enough.
    pub fn tick(&mut self, now: u64) {
        let ready_long = self
            .ready_since
            .is_some_and(|since| now.saturating_sub(since) >= CLEAR_AFTER);
        if self.clear_pending && ready_long {
            self.ends.clear();
            self.clear_pending = false;
        }
    }

    /// The server ended at `now` and no item asked for it.
    pub fn ended(&mut self, now: u64) -> Next {
        self.ready_since = None;
        self.ends.retain(|at| now.saturating_sub(*at) < DAY);
        self.ends.push(now);
        let in_hour = self
            .ends
            .iter()
            .filter(|at| now.saturating_sub(**at) < HOUR)
            .count();
        if self.failed || in_hour >= LIMIT_HOUR || self.ends.len() >= LIMIT_DAY {
            self.failed = true;
            return Next::Failed;
        }
        Next::RestartAt(now + Self::wait(self.ends.len() - 1))
    }

    /// The number of ends in the record: the `restarts` of the runtime report.
    #[must_use]
    pub fn restarts(&self) -> usize {
        self.ends.len()
    }

    #[must_use]
    pub const fn is_failed(&self) -> bool {
        self.failed
    }
}
