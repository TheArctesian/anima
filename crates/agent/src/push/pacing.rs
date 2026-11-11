//! The push rhythm and what each answer makes the agent do (K8 Sections 6 and 7): pure.

use std::time::Duration;

use agent_core::points::BATCH_POINTS;
use agent_core::wire::RefusalCode;

use crate::hub::answer::Answer;

/// The time between pushes while the hub answers well (MET-014).
pub const RHYTHM: Duration = Duration::from_secs(10);

/// The first wait after a failure; each failure doubles it up to [`BACKOFF_MAX`].
pub const BACKOFF_START: Duration = Duration::from_secs(10);

pub const BACKOFF_MAX: Duration = Duration::from_secs(60);

/// How far a wait after a failure may move from its step, either way: 20 % (K8 Section 7).
pub const JITTER: f64 = 0.2;

/// How long a refused token waits before the agent tries it again (K8 Section 6).
pub const REFUSED_RETRY: Duration = Duration::from_mins(10);

/// What one push answer makes the agent do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    /// 204: remove the points through the batch's receipt.
    Remove,
    /// `MALFORMED` or `OUT_OF_RANGE`: the batch can never be accepted; drop it.
    Drop(RefusalCode),
    /// `TOO_LARGE`: send the same data again as a smaller batch.
    Shrink,
    /// `TOKEN_REFUSED` or `WRONG_DEVICE`: stop pushing, keep sampling and buffering.
    Stop(RefusalCode),
    /// Network error, timeout, 5xx or `UNAVAILABLE`: keep the points and wait.
    Wait,
}

/// The action for the answer to a push.
#[must_use]
pub const fn action(answer: &Answer<()>) -> Action {
    match answer {
        Answer::Accepted(()) => Action::Remove,
        Answer::Refused(code) => match code {
            RefusalCode::Malformed | RefusalCode::OutOfRange => Action::Drop(*code),
            RefusalCode::TooLarge => Action::Shrink,
            RefusalCode::TokenRefused | RefusalCode::WrongDevice => Action::Stop(*code),
            // `ENROLMENT_REFUSED` is not a push answer; treat any such answer as a wait.
            RefusalCode::EnrolmentRefused | RefusalCode::Unavailable => Action::Wait,
        },
        Answer::Failed(_) => Action::Wait,
    }
}

/// The size of the batch to retry after `TOO_LARGE` for one of `sent` points: half of it, or
/// `None` when one point alone was too large and can only be dropped.
#[must_use]
pub const fn smaller(sent: usize) -> Option<usize> {
    if sent > 1 { Some(sent / 2) } else { None }
}

/// The largest batch to try next: grows back toward [`BATCH_POINTS`] after each accepted batch.
#[must_use]
pub fn grown(limit: usize) -> usize {
    limit.saturating_mul(2).min(BATCH_POINTS)
}

/// The wait after consecutive failures.
#[derive(Clone, Copy, Debug, Default)]
pub struct Backoff {
    last: Option<Duration>,
}

impl Backoff {
    /// Records a failure and returns how long to wait: the step, 10 s, then 20 s, 40 s, 60 s,
    /// 60 s, moved by up to [`JITTER`] either way by `spread` (0 to 1, from a random source; 0.5
    /// keeps the step), so agents that failed together do not all try again together.
    pub fn failed(&mut self, spread: f64) -> Duration {
        let step = self
            .last
            .map_or(BACKOFF_START, |last| (last * 2).min(BACKOFF_MAX));
        self.last = Some(step);
        step.mul_f64(2.0f64.mul_add(JITTER * spread.clamp(0.0, 1.0), 1.0 - JITTER))
    }

    /// Records a success: the next failure starts again at 10 s.
    pub const fn recovered(&mut self) {
        self.last = None;
    }

    /// Whether the last push failed.
    #[must_use]
    pub const fn is_failing(&self) -> bool {
        self.last.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hub::answer::Cause;

    #[test]
    fn k8_6_each_answer_maps_to_its_action() {
        let refused = |code| action(&Answer::Refused(code));
        assert_eq!(action(&Answer::Accepted(())), Action::Remove);
        assert_eq!(
            refused(RefusalCode::Malformed),
            Action::Drop(RefusalCode::Malformed)
        );
        assert_eq!(
            refused(RefusalCode::OutOfRange),
            Action::Drop(RefusalCode::OutOfRange)
        );
        assert_eq!(refused(RefusalCode::TooLarge), Action::Shrink);
        assert_eq!(
            refused(RefusalCode::TokenRefused),
            Action::Stop(RefusalCode::TokenRefused)
        );
        assert_eq!(
            refused(RefusalCode::WrongDevice),
            Action::Stop(RefusalCode::WrongDevice)
        );
        assert_eq!(refused(RefusalCode::Unavailable), Action::Wait);
        for cause in [Cause::Network, Cause::Timeout, Cause::Status(503)] {
            assert_eq!(action(&Answer::Failed(cause)), Action::Wait);
        }
    }

    #[test]
    fn k8_7_the_backoff_doubles_from_10_to_60_seconds_and_resets() {
        let mut backoff = Backoff::default();
        let waits: Vec<u64> = (0..5).map(|_| backoff.failed(0.5).as_secs()).collect();
        assert_eq!(waits, [10, 20, 40, 60, 60]);
        assert!(backoff.is_failing());
        backoff.recovered();
        assert!(!backoff.is_failing());
        assert_eq!(backoff.failed(0.5).as_secs(), 10);
    }

    #[test]
    fn k8_7_each_wait_is_spread_20_percent_either_way_around_its_step() {
        let waits = |spread: f64| -> Vec<u64> {
            let mut backoff = Backoff::default();
            (0..5).map(|_| backoff.failed(spread).as_secs()).collect()
        };
        assert_eq!(waits(0.0), [8, 16, 32, 48, 48]);
        assert_eq!(waits(1.0), [12, 24, 48, 72, 72]);
        assert_eq!(
            waits(-3.0),
            waits(0.0),
            "a spread past its range is held to it"
        );
        assert_eq!(waits(7.0), waits(1.0));
    }

    #[test]
    fn k8_6_too_large_halves_until_one_point_which_is_dropped() {
        assert_eq!(smaller(60), Some(30));
        assert_eq!(smaller(3), Some(1));
        assert_eq!(smaller(1), None);
        assert_eq!(grown(1), 2);
        assert_eq!(grown(40), 60);
    }
}
