//! The Settings part "Coding agents" (MCP-039): the entry `anima` in each coding agent on this
//! computer, read and changed through the library of `anima mcp install`. This module adds the
//! caller check, the installed `anima`, and the error codes, nothing else. The `anima` program is
//! not started, and no token is read.

use anima_cli::mcp::install::setup::{Action, Here, check_command};
use anima_cli::mcp::install::{Asked, Row, change, listing};
use serde::{Serialize, Serializer, ser::SerializeStruct};
use tauri::{Runtime, Webview};
use tauri_plugin_log::log;

use crate::command_line;

/// Why the agents were not read or changed. Carries no path and no file text (LOG-007).
///
/// - `caller`: not the main web view (WEB-007).
/// - `no_command_line`: the command line tool is not installed, so no entry can name it.
/// - `command`: the installed `anima` is not a fixed program that only this user can replace.
/// - `user`: the home folder does not belong to the user that runs the application.
/// - `failed`: anything else, also a change that names no agent or asks to force one: the page
///   asks for each agent that it listed, and never forces (MCP-029, MCP-036).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum AgentsError {
    #[error("only the main window may ask")]
    Caller,
    #[error("the command line tool is not installed")]
    NoCommandLine,
    #[error("the installed anima is not a fixed program")]
    Command,
    #[error("the home folder does not belong to this user")]
    User,
    #[error("the coding agents could not be read or changed")]
    Failed,
}

impl AgentsError {
    /// Stable code the client switches on.
    const fn code(self) -> &'static str {
        match self {
            Self::Caller => "caller",
            Self::NoCommandLine => "no_command_line",
            Self::Command => "command",
            Self::User => "user",
            Self::Failed => "failed",
        }
    }
}

/// Serialises as `{ "code": ..., "message": ... }` with the fixed `Display` text.
impl Serialize for AgentsError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut out = serializer.serialize_struct("AgentsError", 2)?;
        out.serialize_field("code", self.code())?;
        out.serialize_field("message", &self.to_string())?;
        out.end()
    }
}

/// What the page asks: the action (uninstall or install), the agents by id (all when empty), and
/// whether an entry that starts a different program may be taken out (MCP-036).
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Wanted {
    uninstall: bool,
    #[serde(default)]
    agents: Vec<String>,
    #[serde(default)]
    force: bool,
}

static ONE_AT_A_TIME: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Runs `job` with this computer and the installed `anima`, off the async runtime.
async fn with<R: Runtime>(
    webview: Webview<R>,
    job: impl FnOnce(&Here, &anima_cli::mcp::install::setup::Checked) -> Vec<Row> + Send + 'static,
) -> Result<Vec<Row>, AgentsError> {
    crate::web_view::caller(webview.label()).map_err(|_| AgentsError::Caller)?;
    let installed = command_line::installed()
        .await
        .ok_or(AgentsError::NoCommandLine)?;
    let done = tauri::async_runtime::spawn_blocking(move || {
        let here = Here::for_this_user(installed.home, installed.path).ok_or(AgentsError::User)?;
        let anima = check_command(&installed.anima, here.user).map_err(|_| AgentsError::Command)?;
        if let Some(runs) = &installed.runs {
            check_command(runs, here.user).map_err(|_| AgentsError::Command)?;
        }
        // One read or change at a time: two changes of one agent's own program could race.
        let _one = ONE_AT_A_TIME
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        Ok(job(&here, &anima))
    })
    .await
    .unwrap_or(Err(AgentsError::Failed));
    if let Err(error) = &done {
        log::warn!(
            "the coding agents were not read or changed: {}",
            error.code()
        );
    }
    done
}

const fn action(uninstall: bool) -> Action {
    if uninstall {
        Action::Uninstall
    } else {
        Action::Install
    }
}

/// Each coding agent with its state and what `uninstall` or the install would touch (MCP-027,
/// MCP-029). It changes nothing.
#[tauri::command]
pub async fn mcp_agents_read<R: Runtime>(
    webview: Webview<R>,
    uninstall: bool,
) -> Result<Vec<Row>, AgentsError> {
    with(webview, move |here, anima| {
        listing(here, anima, action(uninstall))
    })
    .await
}

/// Sets the agents up, or takes the entry out, and answers with each agent as it is afterwards
/// (MCP-028, MCP-036). One refused agent does not stop the others; its row says why.
#[tauri::command]
pub async fn mcp_agents_change<R: Runtime>(
    webview: Webview<R>,
    wanted: Wanted,
) -> Result<Vec<Row>, AgentsError> {
    // The page lists each agent before a change and names each one; "all" and "force" are for
    // the command line, where the person reads the list in the terminal.
    if wanted.agents.is_empty() || wanted.force {
        return Err(AgentsError::Failed);
    }
    let result = with(webview, move |here, anima| {
        change(
            here,
            anima,
            &Asked {
                action: action(wanted.uninstall),
                ids: &wanted.agents,
                force: wanted.force,
            },
        )
    })
    .await;
    if let Ok(rows) = &result {
        log::info!(
            "the coding agents were changed; {} refused",
            rows.iter().filter(|row| row.problem.is_some()).count()
        );
    }
    result
}
