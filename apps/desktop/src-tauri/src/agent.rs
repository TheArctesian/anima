//! The desktop app installs the metrics agent on the computer it runs on (MET-019, D-10 d): it
//! copies the agent it carries to `~/.local/bin`, enrols it with a code from the hub and has it
//! install its own user unit or `LaunchAgent`. User level only; nothing runs as root.
//!
//! Only the main web view may ask (WEB-007), and it passes only the code and the hub's origin,
//! both checked here. The code reaches the agent on standard input only, and no log line, error
//! or argument holds it (LOG-007). The shell wipes its own copy of the code (`Zeroizing`); the
//! copies in the IPC message and in the web view are not wiped.
//!
//! Only a release build carries the agent. The release build must build `anima-agent` for the
//! app's target triple from the same commit, place it at
//! `apps/desktop/src-tauri/binaries/anima-agent-<target-triple>`, and pass
//! `--config tauri.release.conf.json` to `tauri build`, which adds it as a sidecar beside the
//! app's executable. A debug build, or one built without that config, says it carries none, and
//! the client shows the install command instead.

mod error;
mod installing;
mod placement;
mod request;
mod run;
mod trust;

#[cfg(test)]
mod tests;

use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
    time::Duration,
};

use tauri::{Runtime, State, Webview};
use tauri_plugin_log::log;
use zeroize::Zeroizing;

pub use error::AgentError;
pub use installing::Installing;
use request::{EnrolmentCode, HubOrigin};
use run::{INSTALL_ARGS, Run};
use trust::User;

/// The hub answers an enrolment within its own 10 s timeout; a macOS Keychain write adds a little.
const ENROL_DEADLINE: Duration = Duration::from_secs(60);
/// `install` runs up to three `systemctl` or `launchctl` steps of 30 s each, one retried.
const INSTALL_DEADLINE: Duration = Duration::from_secs(150);

/// Where accounts and groups are read, to find the user's private group.
const PASSWD: &str = "/etc/passwd";
const GROUP: &str = "/etc/group";

/// This computer as the install needs it: the bundled agent, the user and the agent's variables.
struct Here {
    source: PathBuf,
    home: PathBuf,
    user: User,
    env: Vec<(&'static str, OsString)>,
}

/// What bounds each agent run: its deadline, and the app's stop on exit.
struct Limits<'a> {
    enrol: Duration,
    install: Duration,
    stop: &'a AtomicBool,
}

/// Copies the agent into place, enrols it with `code` at `hub`, then has it install its unit.
/// Each run starts the path that was checked. Returns that path.
fn install_here(
    here: &Here,
    code: &EnrolmentCode,
    hub: &HubOrigin,
    limits: &Limits<'_>,
) -> Result<PathBuf, AgentError> {
    let destination = placement::destination(&here.home);
    let installed = placement::place(&here.source, &destination, here.user)
        .map_err(AgentError::of_placement)?;
    let input = code.input_line();
    let enrol = Run {
        program: &installed,
        args: &run::enrol_args(hub.as_str()),
        input: Some(&input),
        env: &here.env,
        deadline: limits.enrol,
        stop: limits.stop,
    };
    run::run(&enrol).map_err(|error| AgentError::of_run(error, AgentError::Enrol))?;
    let install = Run {
        program: &installed,
        args: &INSTALL_ARGS,
        input: None,
        env: &here.env,
        deadline: limits.install,
        stop: limits.stop,
    };
    run::run(&install).map_err(|error| AgentError::of_run(error, AgentError::Install))?;
    Ok(installed)
}

/// This computer, read from the process: `HOME` must be an absolute path. An account file that
/// cannot be read means no private group, so a folder its group may write is refused.
fn here(source: PathBuf) -> Result<Here, AgentError> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .filter(|home| home.is_absolute())
        .ok_or(AgentError::NoHome)?;
    let uid = nix::unistd::getuid().as_raw();
    let passwd = fs::read_to_string(PASSWD).ok();
    let group = fs::read_to_string(GROUP).ok();
    let user = User {
        uid,
        private_group: trust::private_group(uid, passwd.as_deref(), group.as_deref()),
    };
    let env = run::agent_env(&home, |name| std::env::var_os(name));
    Ok(Here {
        source,
        home,
        user,
        env,
    })
}

/// The agent beside `executable`, in a release build only: a debug build's target folder may
/// hold an agent built from another commit, which is never offered.
fn bundled(executable: &Path, release: bool) -> Result<PathBuf, AgentError> {
    if !release {
        return Err(AgentError::NotBundled);
    }
    placement::bundled_beside(executable)
        .filter(|agent| agent.is_file())
        .ok_or(AgentError::NotBundled)
}

/// The agent this app carries, when it carries one.
fn carried() -> Result<PathBuf, AgentError> {
    if !cfg!(any(target_os = "macos", target_os = "linux")) {
        return Err(AgentError::Unavailable);
    }
    let executable = std::env::current_exe().map_err(|_| AgentError::NotBundled)?;
    bundled(&executable, !cfg!(debug_assertions))
}

/// Refuses every caller but the main web view; the capability already does, this is the second
/// wall (WEB-007).
fn from_main<R: Runtime>(webview: &Webview<R>) -> Result<(), AgentError> {
    crate::web_view::caller(webview.label()).map_err(|_| AgentError::Caller)
}

/// Checks the request, claims the one install and runs it off the async runtime. The claim
/// goes with the work, so it is freed when the work ends.
async fn install<R: Runtime>(
    webview: &Webview<R>,
    installing: &Installing,
    code: Zeroizing<String>,
    origin: &str,
) -> Result<(), AgentError> {
    from_main(webview)?;
    let code = EnrolmentCode::parse(&code).ok_or(AgentError::Code)?;
    let hub = HubOrigin::parse(origin).ok_or(AgentError::Origin)?;
    let claim = installing.claim()?;
    let work = move || {
        let limits = Limits {
            enrol: ENROL_DEADLINE,
            install: INSTALL_DEADLINE,
            stop: &claim.stop,
        };
        install_here(&here(carried()?)?, &code, &hub, &limits).map(drop)
    };
    tauri::async_runtime::spawn_blocking(work)
        .await
        .unwrap_or(Err(AgentError::Failed))
}

/// Whether this build of the app carries the agent, so the client can offer to install it.
#[tauri::command]
pub async fn agent_bundled<R: Runtime>(webview: Webview<R>) -> Result<bool, AgentError> {
    from_main(&webview)?;
    Ok(carried().is_ok())
}

/// Installs the agent on this computer for this user with `code` from the hub at `origin`.
#[tauri::command]
pub async fn agent_install<R: Runtime>(
    webview: Webview<R>,
    installing: State<'_, Installing>,
    code: String,
    origin: String,
) -> Result<(), AgentError> {
    let result = install(&webview, &installing, Zeroizing::new(code), &origin).await;
    match &result {
        Ok(()) => log::info!("the agent was installed on this computer"),
        Err(error) => log::warn!("the agent was not installed: {}", error.code()),
    }
    result
}
