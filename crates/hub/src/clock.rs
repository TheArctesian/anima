//! The hub's only source of the current time (CQ-021).

use chrono::{DateTime, Utc};

pub trait Clock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
}

/// The wall clock. The only place in the hub that reads the system time.
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

/// A clock that always reports the same instant, for tests.
pub struct FixedClock(pub DateTime<Utc>);

impl Clock for FixedClock {
    fn now(&self) -> DateTime<Utc> {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cq_021_fixed_clock_reports_its_instant() {
        let at = DateTime::from_timestamp(1_700_000_000, 0).unwrap();
        assert_eq!(FixedClock(at).now(), at);
    }
}
