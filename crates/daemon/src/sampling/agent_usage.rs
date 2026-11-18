//! The agent usage a batch announces: the daemon's state after start, then each session day
//! whose counts changed since it was last handed to the pusher (AIS-019, K8 Section 4).

use std::collections::HashMap;

use chrono::NaiveDate;
use daemon_core::agent_usage::{SessionDay, Source, UsageState};

/// What a read of the session files hands the pusher.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UsageReport {
    pub state: UsageState,
    /// The session days whose counts changed; none unless the state is `Reading`.
    pub session_days: Vec<SessionDay>,
}

/// One session day: the key the hub upserts by (K8 Section 4).
pub type DayKey = (Source, String, NaiveDate);

#[must_use]
pub fn day_key(day: &SessionDay) -> DayKey {
    (day.source, day.id.clone(), day.day)
}

/// The session days handed to the pusher last.
#[derive(Debug, Default)]
pub struct UsageWatch {
    handed: HashMap<DayKey, SessionDay>,
}

impl UsageWatch {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Of `read`, the session days that differ from the ones handed over before.
    pub fn changed(&mut self, read: Vec<SessionDay>) -> Vec<SessionDay> {
        read.into_iter()
            .filter(|day| {
                let key = day_key(day);
                if self.handed.get(&key) == Some(day) {
                    return false;
                }
                self.handed.insert(key, day.clone());
                true
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use chrono::DateTime;

    use super::*;

    fn day(prompts: u64) -> SessionDay {
        SessionDay {
            source: Source::Codex,
            id: "t".to_owned(),
            day: NaiveDate::from_ymd_opt(2026, 11, 8).expect("a day"),
            started_at: DateTime::UNIX_EPOCH,
            ended_at: DateTime::UNIX_EPOCH,
            repository: None,
            prompts,
            tool_calls: 0,
            models: Vec::new(),
        }
    }

    #[test]
    fn ais_019_only_a_session_day_whose_counts_changed_is_handed_over_again() {
        let mut watch = UsageWatch::new();
        assert_eq!(watch.changed(vec![day(1)]), [day(1)], "the first read");
        assert_eq!(watch.changed(vec![day(1)]).len(), 0, "unchanged");
        assert_eq!(watch.changed(vec![day(2)]), [day(2)], "a new prompt");
    }
}
