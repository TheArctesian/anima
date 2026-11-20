//! The desktop app installs the metrics daemon on the computer it runs on (MET-019, D-10 d): it
//! copies the daemon it carries to `~/.local/bin`, enrols it with a code from the hub and has it
//! install its own user unit or `LaunchAgent`. User level only; nothing runs as root.
//!
//! Only the main web view may ask (WEB-007), and it passes only the code and the hub's origin,
//! both checked here. The code reaches the daemon on standard input only, and no log line, error
//! or argument holds it (LOG-007). The shell wipes its own copy of the code (`Zeroizing`); the
//! copies in the IPC message and in the web view are not wiped.
//!
//! A bundle carries the daemon with the other programs (HMD-052, R-206): the bundle step of the
//! builder (`src/builder/carried/`) and `trial-bundle.sh` build `anima-daemon` from the same
//! commit and name it in `bundle.externalBin`. A debug build carries none and says so, and the
//! client shows the install command instead.

mod error;
mod installing;
pub(crate) mod placement;
mod request;
mod run;
pub(crate) mod trust;

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

pub use error::DaemonError;
pub use installing::Installing;
use request::{EnrolmentCode, HubOrigin};
use run::{INSTALL_ARGS, Run};
use trust::User;

/// The hub answers an enrolment within its own 10 s timeout; a macOS Keychain write adds a little.
const ENROL_DEADLINE: Duration = Duration::from_secs(60);
/// `install` runs up to five steps of 30 s each (two take the unit of the old program out,
/// R-126), one retried after a pause; the rest is margin.
const INSTALL_DEADLINE: Duration = Duration::from_secs(240);

/// Where accounts and groups are read, to find the user's private group.
const PASSWD: &str = "/etc/passwd";
const GROUP: &str = "/etc/group";

/// This computer as the install needs it: the bundled daemon, the user and the daemon's variables.
struct Here {
    source: PathBuf,
    home: PathBuf,
    user: User,
    env: Vec<(&'static str, OsString)>,
}

/// What bounds each daemon run: its deadline, and the app's stop on exit.
struct Limits<'a> {
    enrol: Duration,
    install: Duration,
    stop: &'a AtomicBool,
}

/// Copies the daemon into place, enrols it with `code` at `hub`, then has it install its unit.
/// Each run starts the path that was checked. Returns that path.
fn install_here(
    here: &Here,
    code: &EnrolmentCode,
    hub: &HubOrigin,
    limits: &Limits<'_>,
) -> Result<PathBuf, DaemonError> {
    let destination = placement::destination(&here.home);
    let installed = placement::place(&here.source, &destination, here.user)
        .map_err(DaemonError::of_placement)?;
    let input = code.input_line();
    let enrol = Run {
        program: &installed,
        args: &run::enrol_args(hub.as_str()),
        input: Some(&input),
        env: &here.env,
        deadline: limits.enrol,
        stop: limits.stop,
    };
    run::run(&enrol).map_err(|error| DaemonError::of_run(error, DaemonError::Enrol))?;
    let install = Run {
        program: &installed,
        args: &INSTALL_ARGS,
        input: None,
        env: &here.env,
        deadline: limits.install,
        stop: limits.stop,
    };
    run::run(&install).map_err(|error| DaemonError::of_run(error, DaemonError::Install))?;
    Ok(installed)
}

/// This computer, read from the process: `HOME` must be an absolute path. An account file that
/// cannot be read means no private group, so a folder its group may write is refused.
fn here(source: PathBuf) -> Result<Here, DaemonError> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .filter(|home| home.is_absolute())
        .ok_or(DaemonError::NoHome)?;
    let user = this_user();
    let env = run::daemon_env(&home, |name| std::env::var_os(name));
    Ok(Here {
        source,
        home,
        user,
        env,
    })
}

/// The user of this process, with their private group. An account file that cannot be read
/// means no private group, so a folder its group may write is refused.
pub(crate) fn this_user() -> User {
    let uid = nix::unistd::getuid().as_raw();
    let passwd = fs::read_to_string(PASSWD).ok();
    let group = fs::read_to_string(GROUP).ok();
    User {
        uid,
        private_group: trust::private_group(uid, passwd.as_deref(), group.as_deref()),
    }
}

/// The daemon beside `executable`, in a release build only: a debug build's target folder may
/// hold a daemon built from another commit, which is never offered.
fn bundled(executable: &Path, release: bool) -> Result<PathBuf, DaemonError> {
    if !release {
        return Err(DaemonError::NotBundled);
    }
    placement::bundled_beside(executable)
        .filter(|daemon| daemon.is_file())
        .ok_or(DaemonError::NotBundled)
}

/// The daemon this app carries, when it carries one.
fn carried() -> Result<PathBuf, DaemonError> {
    if !cfg!(any(target_os = "macos", target_os = "linux")) {
        return Err(DaemonError::Unavailable);
    }
    let executable = std::env::current_exe().map_err(|_| DaemonError::NotBundled)?;
    bundled(&executable, !cfg!(debug_assertions))
}

/// Refuses every caller but the main web view; the capability already does, this is the second
/// wall (WEB-007).
fn from_main(label: &str) -> Result<(), DaemonError> {
    crate::web_view::caller(label).map_err(|_| DaemonError::Caller)
}

/// Checks the request, claims the one install and runs it off the async runtime. The claim
/// goes with the work, so it is freed when the work ends.
async fn install<R: Runtime>(
    webview: &Webview<R>,
    installing: &Installing,
    code: Zeroizing<String>,
    origin: &str,
) -> Result<(), DaemonError> {
    from_main(webview.label())?;
    let code = EnrolmentCode::parse(&code).ok_or(DaemonError::Code)?;
    let hub = hub_origin(origin).ok_or(DaemonError::Origin)?;
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
        .unwrap_or(Err(DaemonError::Failed))
}

/// The origin that the daemon is enrolled at: the `https` origin that the web view names, or,
/// for a hub that the app reaches through its link at `http://127.0.0.1:<port>`, the socket
/// that the link of that port made for the daemon (HMD-054). The web view never names a path.
fn hub_origin(origin: &str) -> Option<HubOrigin> {
    HubOrigin::parse(origin).or_else(|| {
        let socket = crate::link::daemon_socket(request::link_port(origin)?)?;
        HubOrigin::link(&socket)
    })
}

/// Whether this build of the app carries the daemon, so the client can offer to install it.
#[tauri::command]
pub async fn daemon_bundled<R: Runtime>(webview: Webview<R>) -> Result<bool, DaemonError> {
    from_main(webview.label())?;
    Ok(carried().is_ok())
}

/// Installs the daemon on this computer for this user with `code` from the hub at `origin`.
#[tauri::command]
pub async fn daemon_install<R: Runtime>(
    webview: Webview<R>,
    installing: State<'_, Installing>,
    code: String,
    origin: String,
) -> Result<(), DaemonError> {
    let result = install(&webview, &installing, Zeroizing::new(code), &origin).await;
    match &result {
        Ok(()) => log::info!("the daemon was installed on this computer"),
        Err(error) => log::warn!("the daemon was not installed: {}", error.code()),
    }
    result
}
