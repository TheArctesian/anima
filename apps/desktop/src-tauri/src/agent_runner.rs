//! The agent threads that the daemon of this computer runs (TH14, THR-128): the page Threads is a
//! client of the local socket of the runner, as `anima agents` is. This module adds the caller
//! check, the error codes, the watch that sends each update to the main window, and the system
//! notification of a thread that needs the person (THR-059). It starts no program itself, sends
//! nothing to the hub and logs no text of a thread (LOG-007).

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use agent_threads::control::{Action, Outcome, Reply};
use anima_cli::agents::runner::RunnerLink;
use anima_cli::failure::Failure;
use serde::{Serialize, Serializer, ser::SerializeStruct};
use tauri::{AppHandle, Emitter, EventTarget, Manager as _, Runtime, State, Webview};
use tauri_plugin_log::log;
use tauri_plugin_notification::NotificationExt as _;

use crate::web_view::MAIN;

/// The event of one update of a thread, to the main window only.
pub const UPDATE_EVENT: &str = "agent-runner:update";
/// The event that says whether the watch reaches the runner: `true` or `false`.
pub const LINK_EVENT: &str = "agent-runner:link";
/// The time between two tries to reach a runner that is not there.
const RETRY: Duration = Duration::from_secs(2);
/// The most characters of the title and of the words of a notification.
const TITLE_CHARS: usize = 120;
const WORDS_CHARS: usize = 80;

/// Why a call did not reach the runner. Carries no path and no text of a thread (LOG-007).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum RunnerError {
    #[error("only the main window may ask")]
    Caller,
    #[error("the thread runner of this computer is not running")]
    NotRunning,
    #[error("the socket of the runner is not a private socket of this user")]
    Foreign,
    #[error("the call did not read")]
    BadCall,
    #[error("the notification was not shown")]
    Notification,
}

impl RunnerError {
    /// Stable code the client switches on.
    const fn code(self) -> &'static str {
        match self {
            Self::Caller => "caller",
            Self::NotRunning => "not_running",
            Self::Foreign => "foreign",
            Self::BadCall => "bad_call",
            Self::Notification => "notification",
        }
    }

    fn of(failure: &Failure) -> Self {
        match failure {
            Failure::Forbidden { .. } => Self::Foreign,
            _ => Self::NotRunning,
        }
    }
}

/// Serialises as `{ "code": ..., "message": ... }` with the fixed `Display` text.
impl Serialize for RunnerError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut out = serializer.serialize_struct("RunnerError", 2)?;
        out.serialize_field("code", self.code())?;
        out.serialize_field("message", &self.to_string())?;
        out.end()
    }
}

/// The number of the newest watch of the runner: an older watch ends at its next update, so that
/// one watch runs, and a page that loads again gets the kept updates again.
#[derive(Default)]
pub struct Watching(AtomicU64);

fn checked<R: Runtime>(webview: &Webview<R>) -> Result<(), RunnerError> {
    crate::web_view::caller(webview.label()).map_err(|_| RunnerError::Caller)
}

/// One call to the runner and its result. A refusal of the runner is a result
/// (`Outcome::Refused`), not an error: the page says it in words.
#[tauri::command]
pub async fn agent_runner_call<R: Runtime>(
    webview: Webview<R>,
    action: serde_json::Value,
) -> Result<Outcome, RunnerError> {
    checked(&webview)?;
    let action: Action = serde_json::from_value(action).map_err(|_| RunnerError::BadCall)?;
    if matches!(action, Action::Watch) {
        return Err(RunnerError::BadCall);
    }
    let mut link = RunnerLink::connect()
        .await
        .map_err(|failure| RunnerError::of(&failure))?;
    let outcome = link.call(action).await;
    outcome.map_err(|failure| {
        let error = RunnerError::of(&failure);
        log::warn!("a call to the thread runner failed: {}", error.code());
        error
    })
}

/// Starts a new watch of the runner in place of the one before: each update goes to the main
/// window as [`UPDATE_EVENT`]; when the runner is not there, the watch tries again each [`RETRY`].
#[expect(
    clippy::needless_pass_by_value,
    reason = "a Tauri command takes its arguments by value"
)]
#[tauri::command]
pub fn agent_runner_watch<R: Runtime>(
    webview: Webview<R>,
    app: AppHandle<R>,
    watching: State<'_, Watching>,
) -> Result<(), RunnerError> {
    checked(&webview)?;
    let generation = watching.0.fetch_add(1, Ordering::SeqCst) + 1;
    tauri::async_runtime::spawn(watch(app, generation));
    Ok(())
}

fn newest<R: Runtime>(app: &AppHandle<R>, generation: u64) -> bool {
    app.state::<Watching>().0.load(Ordering::SeqCst) == generation
}

async fn watch<R: Runtime>(app: AppHandle<R>, generation: u64) {
    let to = EventTarget::webview_window(MAIN);
    while newest(&app, generation) {
        if let Ok(mut link) = RunnerLink::connect().await
            && link.send(Action::Watch).await.is_ok()
        {
            let _ = app.emit_to(to.clone(), LINK_EVENT, true);
            while let Ok(Some(reply)) = link.next().await {
                if !newest(&app, generation) {
                    return;
                }
                if let Reply::Update(update) = reply {
                    let _ = app.emit_to(to.clone(), UPDATE_EVENT, update);
                }
            }
        }
        let _ = app.emit_to(to.clone(), LINK_EVENT, false);
        tokio::time::sleep(RETRY).await;
    }
}

/// `text` on one line with no control character, held to `most` characters.
fn shown(text: &str, most: usize) -> String {
    let one: String = text
        .chars()
        .map(|each| if each.is_control() { ' ' } else { each })
        .collect();
    let one = one.split_whitespace().collect::<Vec<_>>().join(" ");
    if one.chars().count() > most {
        format!("{}…", one.chars().take(most).collect::<String>())
    } else {
        one
    }
}

/// A system notification of this computer for a thread that needs the person (THR-059). Its
/// text goes to the operating system only (THR-060); the page decides when one is due.
#[expect(
    clippy::needless_pass_by_value,
    reason = "a Tauri command takes its arguments by value"
)]
#[tauri::command]
pub fn agent_thread_notify<R: Runtime>(
    webview: Webview<R>,
    app: AppHandle<R>,
    title: String,
    words: String,
) -> Result<(), RunnerError> {
    checked(&webview)?;
    app.notification()
        .builder()
        .title(shown(&title, TITLE_CHARS))
        .body(shown(&words, WORDS_CHARS))
        .show()
        .map_err(|_| RunnerError::Notification)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_notification_text_is_one_short_line_with_no_control_character() {
        assert_eq!(shown("Fix\nthe\u{1b}[31m test", 80), "Fix the [31m test");
        assert_eq!(shown(&"a".repeat(130), 120).chars().count(), 121);
    }
}
