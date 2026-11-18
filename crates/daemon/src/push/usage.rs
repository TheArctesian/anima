//! The agent usage waiting for the hub (AIS-019, K8 Section 4): the state until a batch with it
//! is accepted, and the changed session days, at most 16 to a batch, each until a batch with it
//! is accepted or refused as `MALFORMED`. A newer count of a waiting day replaces it.

use std::collections::BTreeMap;

use daemon_core::agent_usage::{MAX_SESSION_DAYS, SessionDay, UsageState};
use daemon_core::wire::BatchAgentUsage;

use crate::sampling::agent_usage::{DayKey, UsageReport, day_key};

/// What waits, and what the batch in flight carries.
#[derive(Debug, Default)]
pub struct UsageBacklog {
    state: Option<UsageState>,
    /// The state goes in the next batch: after start, and after it changed.
    announce: bool,
    waiting: BTreeMap<DayKey, SessionDay>,
    in_flight: Vec<DayKey>,
}

impl UsageBacklog {
    /// Takes what a read of the session files handed over.
    pub fn take(&mut self, report: UsageReport) {
        if self.state != Some(report.state) {
            self.state = Some(report.state);
            self.announce = true;
        }
        for day in report.session_days {
            self.waiting.insert(day_key(&day), day);
        }
    }

    /// The member of the next batch: the state with the first 16 session days waiting; `None`
    /// when nothing waits.
    pub fn outgoing(&mut self) -> Option<BatchAgentUsage> {
        let state = self.state?;
        if !self.announce && self.waiting.is_empty() {
            return None;
        }
        let days: Vec<&SessionDay> = self.waiting.values().take(MAX_SESSION_DAYS).collect();
        self.in_flight = days.iter().map(|day| day_key(day)).collect();
        let days: Vec<SessionDay> = days.into_iter().cloned().collect();
        Some(BatchAgentUsage::new(state, &days))
    }

    /// The batch of [`outgoing`](Self::outgoing) is done with: accepted, or refused as `MALFORMED`
    /// (its days are then dropped until their counts change). Nothing was taken in between,
    /// so the days in flight are the ones sent.
    pub fn done(&mut self) {
        for key in self.in_flight.drain(..) {
            self.waiting.remove(&key);
        }
        self.announce = false;
    }
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, NaiveDate};
    use daemon_core::agent_usage::Source;

    use super::*;

    fn day(id: u32) -> SessionDay {
        SessionDay {
            source: Source::ClaudeCode,
            id: format!("s{id:02}"),
            day: NaiveDate::from_ymd_opt(2026, 11, 8).expect("a day"),
            started_at: DateTime::UNIX_EPOCH,
            ended_at: DateTime::UNIX_EPOCH,
            repository: None,
            prompts: 1,
            tool_calls: 0,
            models: Vec::new(),
        }
    }

    fn reading(days: Vec<SessionDay>) -> UsageReport {
        UsageReport {
            state: UsageState::Reading,
            session_days: days,
        }
    }

    #[test]
    fn ais_019_the_state_goes_once_and_session_days_16_to_a_batch() {
        let mut backlog = UsageBacklog::default();
        assert_eq!(backlog.outgoing(), None, "nothing read yet");
        backlog.take(reading((0..20).map(day).collect()));
        let first = backlog.outgoing().expect("the first batch");
        assert_eq!(
            (first.state.as_str(), first.session_days.len()),
            ("reading", 16)
        );
        assert_eq!(backlog.outgoing(), Some(first), "kept until done");
        backlog.done();
        let rest = backlog.outgoing().expect("the rest");
        assert_eq!(rest.session_days.len(), 4);
        assert_eq!(rest.session_days[0].id, "s16");
        backlog.done();
        assert_eq!(backlog.outgoing(), None);
        backlog.take(reading(Vec::new()));
        assert_eq!(backlog.outgoing(), None, "the same state is not sent again");
    }

    #[test]
    fn ais_024_a_daemon_that_cannot_read_announces_its_state_and_no_days() {
        let mut backlog = UsageBacklog::default();
        backlog.take(UsageReport {
            state: UsageState::SystemUnit,
            session_days: Vec::new(),
        });
        let sent = backlog.outgoing().expect("the state");
        assert_eq!(
            (sent.state.as_str(), sent.session_days.len()),
            ("system_unit", 0)
        );
        backlog.done();
        assert_eq!(backlog.outgoing(), None);
    }

    #[test]
    fn ais_019_a_newer_count_of_a_waiting_day_replaces_it() {
        let mut backlog = UsageBacklog::default();
        backlog.take(reading(vec![day(1)]));
        let newer = SessionDay {
            prompts: 5,
            ..day(1)
        };
        backlog.take(reading(vec![newer]));
        let sent = backlog.outgoing().expect("one day");
        assert_eq!(sent.session_days.len(), 1);
        assert_eq!(sent.session_days[0].prompts, 5);
    }
}
