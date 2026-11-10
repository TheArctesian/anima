//! The desktop app installs the metrics agent on the computer it runs on (MET-019, D-10 d): it
//! copies the agent it carries to `~/.local/bin`, enrols it with a code from the hub and has it
//! install its own user unit or `LaunchAgent`. User level only; nothing runs as root.
//!
//! The web view passes only the code and the hub's origin, both checked here. The code reaches
//! the agent on standard input only, and no log line, error or argument holds it (LOG-007).
//!
//! The agent is in the app only in release builds (`tauri.release.conf.json`); elsewhere the app
//! says it carries none, and the client shows the install command instead.

mod placement;
mod request;
mod run;

#[cfg(test)]
mod tests;

use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

use serde::{Serialize, Serializer, ser::SerializeStruct};
use tauri_plugin_log::log;

use placement::PlacementError;
use request::{EnrolmentCode, HubOrigin};
use run::{INSTALL_ARGS, Run, RunError};

/// The hub answers an enrolment within its own 10 s timeout; a macOS Keychain write adds a little.
const ENROL_DEADLINE: Duration = Duration::from_secs(60);
/// `install` runs up to three `systemctl` or `launchctl` steps of 30 s each, one retried.
const INSTALL_DEADLINE: Duration = Duration::from_secs(150);

/// Why the agent was not installed. Each message is fixed text: no code, path or OS error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum AgentError {
    #[error("the agent cannot be installed on this system")]
    Unavailable,
    #[error("this build of the app carries no agent")]
    NotBundled,
    #[error("an install is already running")]
    Busy,
    #[error("the hub gave a code in a form the agent does not accept")]
    Code,
    #[error("the hub address is not a plain https origin on port 443")]
    Origin,
    #[error("the app could not find your home folder")]
    NoHome,
    #[error("~/.local/bin, or a folder above it, can be changed by other users")]
    Untrusted,
    #[error("the agent could not be copied to ~/.local/bin")]
    Copy,
    #[error("the agent could not enrol with the hub; make a new code and try again")]
    Enrol,
    #[error("the agent enrolled but did not start; run `anima-agent install` in a terminal")]
    Install,
    #[error("the agent did not finish in time")]
    TimedOut,
    #[error("the install did not finish")]
    Failed,
}

impl AgentError {
    /// Stable code the client switches on.
    fn code(self) -> &'static str {
        match self {
            Self::Unavailable => "unavailable",
            Self::NotBundled => "not-bundled",
            Self::Busy => "busy",
            Self::Code => "code",
            Self::Origin => "origin",
            Self::NoHome => "no-home",
            Self::Untrusted => "untrusted",
            Self::Copy => "copy",
            Self::Enrol => "enrol",
            Self::Install => "install",
            Self::TimedOut => "timed-out",
            Self::Failed => "failed",
        }
    }

    fn of_placement(error: PlacementError) -> Self {
        match error {
            PlacementError::Untrusted => Self::Untrusted,
            PlacementError::Source => Self::NotBundled,
            PlacementError::Write => Self::Copy,
        }
    }

    /// `failed` is what the step's own failure means; the rest are the same for both steps.
    fn of_run(error: RunError, failed: Self) -> Self {
        match error {
            RunError::TimedOut => Self::TimedOut,
            RunError::Failed => failed,
            RunError::CouldNotStart | RunError::Input | RunError::Unreadable => Self::Failed,
        }
    }
}

/// Serialises as `{ "code": ..., "message": ... }` with the fixed `Display` text.
impl Serialize for AgentError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut out = serializer.serialize_struct("AgentError", 2)?;
        out.serialize_field("code", self.code())?;
        out.serialize_field("message", &self.to_string())?;
        out.end()
    }
}

/// This computer as the install needs it: the bundled agent, the user and the agent's variables.
struct Here {
    source: PathBuf,
    home: PathBuf,
    uid: u32,
    env: Vec<(&'static str, String)>,
}

/// How long each agent run may take.
#[derive(Debug, Clone, Copy)]
struct Deadlines {
    enrol: Duration,
    install: Duration,
}

/// Copies the agent into place, enrols it with `code` at `hub`, then has it install its unit.
/// Returns the installed binary's path.
fn install_here(
    here: &Here,
    code: &EnrolmentCode,
    hub: &HubOrigin,
    deadlines: Deadlines,
) -> Result<PathBuf, AgentError> {
    let installed = placement::destination(&here.home);
    placement::place(&here.source, &installed, here.uid).map_err(AgentError::of_placement)?;
    let input = code.input_line();
    let enrol = Run {
        program: &installed,
        args: &run::enrol_args(hub.as_str()),
        input: Some(&input),
        env: &here.env,
        deadline: deadlines.enrol,
    };
    run::run(&enrol).map_err(|error| AgentError::of_run(error, AgentError::Enrol))?;
    let install = Run {
        program: &installed,
        args: &INSTALL_ARGS,
        input: None,
        env: &here.env,
        deadline: deadlines.install,
    };
    run::run(&install).map_err(|error| AgentError::of_run(error, AgentError::Install))?;
    Ok(installed)
}

/// This computer, read from the process: `HOME` must be an absolute path.
fn here(source: PathBuf) -> Result<Here, AgentError> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .filter(|home| home.is_absolute())
        .ok_or(AgentError::NoHome)?;
    let env = run::agent_env(&home, |name| std::env::var(name).ok());
    Ok(Here {
        source,
        home,
        uid: nix::unistd::getuid().as_raw(),
        env,
    })
}

/// The agent this app carries, when it carries one.
fn bundled() -> Result<PathBuf, AgentError> {
    if !cfg!(any(target_os = "macos", target_os = "linux")) {
        return Err(AgentError::Unavailable);
    }
    let executable = std::env::current_exe().map_err(|_| AgentError::NotBundled)?;
    placement::bundled_beside(&executable)
        .filter(|agent| Path::is_file(agent))
        .ok_or(AgentError::NotBundled)
}

/// Whether one install is running; a second is refused while it does.
#[derive(Debug, Default)]
pub struct Installing(AtomicBool);

/// Holds [`Installing`] for one install; dropping it frees it.
struct Claim<'a>(&'a Installing);

impl Installing {
    fn claim(&self) -> Result<Claim<'_>, AgentError> {
        self.0
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map(|_| Claim(self))
            .map_err(|_| AgentError::Busy)
    }
}

impl Drop for Claim<'_> {
    fn drop(&mut self) {
        self.0.0.store(false, Ordering::Release);
    }
}

/// Whether this build of the app carries the agent, so the client can offer to install it.
#[tauri::command]
pub fn agent_bundled() -> bool {
    bundled().is_ok()
}

/// Installs the agent on this computer for this user with `code` from the hub at `origin`.
#[tauri::command]
pub async fn agent_install(
    installing: tauri::State<'_, Installing>,
    code: String,
    origin: String,
) -> Result<(), AgentError> {
    let code = EnrolmentCode::parse(&zeroize::Zeroizing::new(code));
    let result = match (code, HubOrigin::parse(&origin)) {
        (None, _) => Err(AgentError::Code),
        (_, None) => Err(AgentError::Origin),
        (Some(code), Some(hub)) => match installing.claim() {
            Ok(_claim) => {
                let deadlines = Deadlines {
                    enrol: ENROL_DEADLINE,
                    install: INSTALL_DEADLINE,
                };
                let work = move || install_here(&here(bundled()?)?, &code, &hub, deadlines);
                tauri::async_runtime::spawn_blocking(work)
                    .await
                    .unwrap_or(Err(AgentError::Failed))
                    .map(drop)
            }
            Err(busy) => Err(busy),
        },
    };
    match &result {
        Ok(()) => log::info!("the agent was installed on this computer"),
        Err(error) => log::warn!("the agent was not installed: {}", error.code()),
    }
    result
}
