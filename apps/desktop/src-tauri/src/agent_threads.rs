//! The threads of this computer (THR-011): the sessions of Claude Code and Codex of the user
//! that runs the application, read from their files through the library of
//! `anima agents threads`. This module adds the caller check and the error codes, nothing else.
//! It starts no program, sends nothing to the hub and logs no text of a session (LOG-007).

use anima_cli::agents::threads::shown::Shown;
use anima_cli::agents::threads::{self, Program, ThreadRow, files};
use serde::{Serialize, Serializer, ser::SerializeStruct};
use tauri::{Runtime, Webview};
use tauri_plugin_log::log;

/// Why the threads were not read. Carries no path and no text of a session (LOG-007).
///
/// - `caller`: not the main web view (WEB-007).
/// - `home`: the home folder of this user is not known.
/// - `not_found`: no thread, or more than one, has the ID.
/// - `failed`: anything else.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ThreadsError {
    #[error("only the main window may ask")]
    Caller,
    #[error("the home folder of this user is not known")]
    Home,
    #[error("no thread of this computer has this ID")]
    NotFound,
    #[error("the thread could not be read")]
    Failed,
}

impl ThreadsError {
    /// Stable code the client switches on.
    const fn code(self) -> &'static str {
        match self {
            Self::Caller => "caller",
            Self::Home => "home",
            Self::NotFound => "not_found",
            Self::Failed => "failed",
        }
    }
}

/// Serialises as `{ "code": ..., "message": ... }` with the fixed `Display` text.
impl Serialize for ThreadsError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut out = serializer.serialize_struct("ThreadsError", 2)?;
        out.serialize_field("code", self.code())?;
        out.serialize_field("message", &self.to_string())?;
        out.end()
    }
}

/// Runs `job` off the async runtime for the main web view only.
async fn with<R: Runtime, T: Send + 'static>(
    webview: &Webview<R>,
    job: impl FnOnce() -> Result<T, ThreadsError> + Send + 'static,
) -> Result<T, ThreadsError> {
    crate::web_view::caller(webview.label()).map_err(|_| ThreadsError::Caller)?;
    let done = tauri::async_runtime::spawn_blocking(job)
        .await
        .unwrap_or(Err(ThreadsError::Failed));
    if let Err(error) = &done {
        log::warn!("the threads were not read: {}", error.code());
    }
    done
}

/// The threads of this computer, the last changed first (THR-011).
#[tauri::command]
pub async fn agent_threads_list<R: Runtime>(
    webview: Webview<R>,
    program: Option<String>,
    limit: Option<usize>,
) -> Result<Vec<ThreadRow>, ThreadsError> {
    let program = match program.as_deref() {
        Some("claude_code") => Some(Program::ClaudeCode),
        Some("codex") => Some(Program::Codex),
        _ => None,
    };
    with(&webview, move || {
        let home = threads::home().map_err(|_| ThreadsError::Home)?;
        let limit = limit.unwrap_or(threads::MAX_LIMIT);
        Ok(threads::list(&home, program.map(Program::source), limit))
    })
    .await
}

/// One thread of this computer with its entries (THR-011).
#[tauri::command]
pub async fn agent_thread_read<R: Runtime>(
    webview: Webview<R>,
    id: String,
) -> Result<Shown, ThreadsError> {
    with(&webview, move || {
        let home = threads::home().map_err(|_| ThreadsError::Home)?;
        let file = threads::find(&home, &id).map_err(|_| ThreadsError::NotFound)?;
        let thread = files::read(&file).map_err(|_| ThreadsError::Failed)?;
        Ok(Shown::of(&file, &thread))
    })
    .await
}
