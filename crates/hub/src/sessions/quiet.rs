//! One side's silence on a tunnel (K4 Sections 4, 8.4 and 9): a deadline that starts again
//! after each turn that side takes. Pure: the caller passes the time in (CQ-021), as a
//! `tokio::time::Instant` so tests with paused time drive it.

use std::time::Duration;

use tokio::time::Instant;

/// When one side's silence comes due: `wait` after it last started.
#[derive(Clone, Copy)]
pub(in crate::sessions) struct Quiet {
    wait: Duration,
    due: Instant,
}

impl Quiet {
    /// A silence of `wait` that starts at `now`.
    pub(in crate::sessions) fn new(wait: Duration, now: Instant) -> Self {
        Self {
            wait,
            due: now + wait,
        }
    }

    /// Starts the silence again at `now`: it comes due `wait` later.
    pub(in crate::sessions) fn restart(&mut self, now: Instant) {
        self.due = now + self.wait;
    }

    /// When the silence comes due.
    pub(in crate::sessions) const fn due(&self) -> Instant {
        self.due
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tokio::time::Instant;

    use super::Quiet;

    const WAIT: Duration = Duration::from_secs(5);

    #[test]
    fn k4_4_a_silence_comes_due_its_wait_after_it_starts() {
        let start = Instant::now();

        assert_eq!(Quiet::new(WAIT, start).due(), start + WAIT);
    }

    #[test]
    fn k4_4_a_turn_restarts_the_silence_from_the_turn_not_from_the_start() {
        let start = Instant::now();
        let mut quiet = Quiet::new(WAIT, start);
        let turn = start + Duration::from_secs(3);

        quiet.restart(turn);

        assert_eq!(quiet.due(), turn + WAIT);
    }
}
