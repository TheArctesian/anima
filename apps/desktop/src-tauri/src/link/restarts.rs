//! Bound repeated short-lived child failures without preventing recovery after a stable run.
use std::time::Duration;

const STABLE_RUN: Duration = Duration::from_secs(60);
const MAX_SHORT_RUNS: u8 = 3;

#[derive(Default)]
pub(super) struct Restarts(u8);

impl Restarts {
    /// Whether another automatic launch is allowed after a child ended.
    pub(super) fn after(&mut self, runtime: Duration) -> bool {
        self.0 = if runtime >= STABLE_RUN {
            0
        } else {
            self.0.saturating_add(1)
        };
        self.0 < MAX_SHORT_RUNS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeated_early_exits_stop_but_a_stable_run_resets_the_budget() {
        let mut restarts = Restarts::default();
        assert!(restarts.after(Duration::ZERO));
        assert!(restarts.after(Duration::from_secs(3)));
        assert!(!restarts.after(Duration::from_secs(59)));
        assert!(!restarts.after(Duration::ZERO));
        assert!(restarts.after(STABLE_RUN));
        assert!(restarts.after(Duration::ZERO));
        assert!(restarts.after(Duration::ZERO));
        assert!(!restarts.after(Duration::ZERO));
    }
}
