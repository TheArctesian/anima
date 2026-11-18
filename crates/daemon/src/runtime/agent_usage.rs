//! The task that reads the session files of the AI agent programs at start and every 15
//! minutes and hands the changed session days to the pusher (AIS-018, AIS-019, AIS-024).

use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use daemon_core::agent_usage::UsageState;
use tokio::sync::mpsc;
use tokio::time::{MissedTickBehavior, interval};

use crate::config::agent_usage::UsageSettings;
use crate::config::home::Home;
use crate::push::Event;
use crate::readers::Platform;
use crate::readers::agent_sessions::{AgentSessions, user_home};
use crate::report;
use crate::sampling::agent_usage::{UsageReport, UsageWatch};

/// The time between two reads of the session files (K8 Section 4).
const COUNT_EVERY: Duration = Duration::from_mins(15);

/// What the daemon does with the session files: read them, or say why it cannot (AIS-024).
pub(super) enum Counting {
    Reading(AgentSessions),
    Silent(UsageState),
}

/// Reads the session files only when the daemon runs as a user, and the operator's settings
/// allow it; settings that cannot be read turn the function off (AIS-008, AIS-024).
pub(super) fn counting(home: &Home, env: impl Fn(&str) -> Option<String>) -> Counting {
    let Some(user) = user_home(env) else {
        return Counting::Silent(UsageState::SystemUnit);
    };
    match UsageSettings::read(home) {
        Ok(settings) if !settings.off => {
            let macos = Platform::current() == Platform::Macos;
            Counting::Reading(AgentSessions::new(user, macos, settings))
        }
        Ok(_) => Counting::Silent(UsageState::TurnedOff),
        Err(error) => {
            report::agent_usage_settings_refused(&error);
            Counting::Silent(UsageState::TurnedOff)
        }
    }
}

/// At start and every 15 minutes: reads the session files on a blocking thread and hands the
/// changed session days over (AIS-019). A daemon that does not read hands its state over once.
/// A read stops between two lines when `stopping` is set.
pub(super) async fn count(
    counting: Counting,
    events: mpsc::Sender<Event>,
    stopping: Arc<AtomicBool>,
) {
    let mut sessions = match counting {
        Counting::Reading(sessions) => sessions,
        Counting::Silent(state) => {
            report::agent_usage(state);
            let silent = UsageReport {
                state,
                session_days: Vec::new(),
            };
            events.send(Event::AgentUsage(silent)).await.ok();
            return;
        }
    };
    report::agent_usage(UsageState::Reading);
    let mut watch = UsageWatch::new();
    let mut ticks = interval(COUNT_EVERY);
    ticks.set_missed_tick_behavior(MissedTickBehavior::Delay);
    loop {
        ticks.tick().await;
        let stop = Arc::clone(&stopping);
        let read = tokio::task::spawn_blocking(move || {
            let read = sessions.scan(&stop);
            (sessions, read)
        });
        let Ok((back, read)) = read.await else {
            report::agent_usage_read_failed();
            return;
        };
        sessions = back;
        let report = UsageReport {
            state: UsageState::Reading,
            session_days: watch.changed(read),
        };
        if events.send(Event::AgentUsage(report)).await.is_err() {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::agent_usage::SETTINGS_FILE;

    fn decided(settings: Option<&str>, daemon_home: Option<&str>) -> Counting {
        let dir = tempfile::tempdir().expect("a folder");
        let home = Home::at(dir.path().to_owned());
        if let Some(text) = settings {
            std::fs::write(dir.path().join(SETTINGS_FILE), text).expect("written");
        }
        let env = |name: &str| match name {
            "HOME" => Some("/home/me".to_owned()),
            "ANIMA_DAEMON_HOME" => daemon_home.map(str::to_owned),
            _ => None,
        };
        counting(&home, env)
    }

    #[test]
    fn ais_024_the_system_unit_does_not_read_sessions() {
        let system = decided(None, Some("/var/lib/anima-daemon"));
        assert!(matches!(system, Counting::Silent(UsageState::SystemUnit)));
    }

    #[test]
    fn ais_008_settings_that_are_off_or_unreadable_turn_the_counts_off() {
        let off = decided(Some(r#"{"off":true}"#), None);
        assert!(matches!(off, Counting::Silent(UsageState::TurnedOff)));
        let misspelt = decided(Some(r#"{"excludeFolders":["/x"]}"#), None);
        assert!(matches!(misspelt, Counting::Silent(UsageState::TurnedOff)));
        assert!(matches!(decided(None, None), Counting::Reading(_)));
        let excluded = decided(Some(r#"{"excludedFolders":["/x"]}"#), None);
        assert!(matches!(excluded, Counting::Reading(_)));
    }
}
