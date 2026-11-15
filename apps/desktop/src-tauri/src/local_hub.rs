//! The hub on this computer (one-click plan S4): the shell installs it, removes it and reads
//! its state. The work is `anima-local-hub`, a program the application carries beside its own;
//! the shell finds it, runs it with a small fixed environment and a time limit, and reads its
//! answer. No rule of the hub is here (REPO-003).
//!
//! Only the main web view may ask (WEB-007). The commands take no argument: no path, no program
//! and no address comes from the web view. Nothing here holds or logs a secret (LOG-007).
//!
//! Only a release build that the builder gave the programs carries them. Any other build says
//! it carries none.

mod carried;
mod error;
mod program;

#[cfg(test)]
mod tests;

use std::{
    ffi::OsString,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use tauri::{Runtime, State, Webview};
use tauri_plugin_log::log;

use carried::Carried;
pub use error::LocalHubError;

/// More than the worst case of `install`: the stop command and the wait for the hub to stop
/// (120 s each), then up to three start commands on Linux (120 s each), and the copy. A kill
/// in the middle would leave the hub stopped.
const INSTALL_LIMIT: Duration = Duration::from_secs(720);
/// `remove`: the stop command, the wait and on Linux one more command, 120 s each.
const REMOVE_LIMIT: Duration = Duration::from_secs(420);
const STATUS_LIMIT: Duration = Duration::from_secs(10);

/// The variables the program gets from the app, when they are set: where the user's folders
/// are, and the session bus that `systemctl --user` needs.
const PASSED_ON: [&str; 4] = [
    "HOME",
    "XDG_DATA_HOME",
    "XDG_RUNTIME_DIR",
    "DBUS_SESSION_BUS_ADDRESS",
];

/// The one install or remove that may run at a time.
#[derive(Debug, Default)]
pub struct Changing(Arc<AtomicBool>);

/// Holds [`Changing`] for one install or remove; dropping it frees it.
struct Claim(Arc<AtomicBool>);

impl Changing {
    fn claim(&self) -> Result<Claim, LocalHubError> {
        if self.0.swap(true, Ordering::SeqCst) {
            return Err(LocalHubError::Busy);
        }
        Ok(Claim(Arc::clone(&self.0)))
    }
}

impl Drop for Claim {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

/// What the program may see of the app's environment. `HOME` must be an absolute path.
fn environment(
    read: impl Fn(&str) -> Option<OsString>,
) -> Result<Vec<(&'static str, OsString)>, LocalHubError> {
    let home_is_absolute = read("HOME").is_some_and(|home| Path::new(&home).is_absolute());
    if !home_is_absolute {
        return Err(LocalHubError::NoHome);
    }
    Ok(PASSED_ON
        .into_iter()
        .filter_map(|name| Some((name, read(name)?)))
        .collect())
}

/// The programs this app carries, when it carries them.
fn carried() -> Result<Carried, LocalHubError> {
    if !cfg!(any(target_os = "macos", target_os = "linux")) {
        return Err(LocalHubError::Unavailable);
    }
    let executable = std::env::current_exe().map_err(|_| LocalHubError::NotCarried)?;
    carried::beside(
        &executable,
        cfg!(target_os = "macos"),
        !cfg!(debug_assertions),
    )
}

/// Runs the carried program with `args`, and returns what it printed.
fn ask(carried: &Carried, args: &[&OsString], limit: Duration) -> Result<Vec<u8>, LocalHubError> {
    let env = environment(|name| std::env::var_os(name))?;
    program::run(&carried.program, args, &env, limit).map_err(|error| {
        log::warn!("anima-local-hub: {error}");
        LocalHubError::from(&error)
    })
}

fn install_args(tree: &Path) -> [OsString; 3] {
    ["install".into(), "--tree".into(), tree.into()]
}

/// The status as the program printed it: one JSON object with `installed` and `running`.
fn parse_status(printed: &[u8]) -> Result<serde_json::Value, LocalHubError> {
    let status: serde_json::Value =
        serde_json::from_slice(printed).map_err(|_| LocalHubError::Failed)?;
    let is_status = status
        .get("installed")
        .is_some_and(serde_json::Value::is_boolean)
        && status
            .get("running")
            .is_some_and(serde_json::Value::is_boolean);
    if !is_status {
        return Err(LocalHubError::Failed);
    }
    Ok(status)
}

/// Refuses every caller but the main web view; the capability already does, this is the second
/// wall (WEB-007).
fn from_main(label: &str) -> Result<(), LocalHubError> {
    crate::web_view::caller(label).map_err(|_| LocalHubError::Caller)
}

/// Claims the one change and runs `work` off the async runtime. The claim goes with the work,
/// so it is freed when the work ends.
async fn change(
    changing: &Changing,
    work: impl FnOnce(&Carried) -> Result<Vec<u8>, LocalHubError> + Send + 'static,
) -> Result<(), LocalHubError> {
    let claim = changing.claim()?;
    let work = move || {
        let _claim = claim;
        work(&carried()?).map(drop)
    };
    tauri::async_runtime::spawn_blocking(work)
        .await
        .unwrap_or(Err(LocalHubError::Failed))
}

/// Whether this build of the app carries the hub's programs, so the client can offer a hub
/// on this computer.
#[tauri::command]
pub async fn local_hub_carried<R: Runtime>(webview: Webview<R>) -> Result<bool, LocalHubError> {
    from_main(webview.label())?;
    Ok(carried().is_ok())
}

/// Installs or updates the hub on this computer for this user, and starts it.
#[tauri::command]
pub async fn local_hub_install<R: Runtime>(
    webview: Webview<R>,
    changing: State<'_, Changing>,
) -> Result<(), LocalHubError> {
    from_main(webview.label())?;
    let result = change(&changing, |carried| {
        let args = install_args(&carried.tree);
        ask(carried, &args.each_ref(), INSTALL_LIMIT)
    })
    .await;
    match &result {
        Ok(()) => log::info!("the hub was installed on this computer"),
        Err(error) => log::warn!("the hub was not installed: {}", error.code()),
    }
    result
}

/// Stops the hub on this computer and removes its unit and programs. Its database and key stay.
#[tauri::command]
pub async fn local_hub_remove<R: Runtime>(
    webview: Webview<R>,
    changing: State<'_, Changing>,
) -> Result<(), LocalHubError> {
    from_main(webview.label())?;
    let result = change(&changing, |carried| {
        ask(carried, &[&OsString::from("remove")], REMOVE_LIMIT)
    })
    .await;
    match &result {
        Ok(()) => log::info!("the hub was removed from this computer"),
        Err(error) => log::warn!("the hub was not removed: {}", error.code()),
    }
    result
}

/// Whether the hub is installed and runs, its state and where it listens.
#[tauri::command]
pub async fn local_hub_status<R: Runtime>(
    webview: Webview<R>,
) -> Result<serde_json::Value, LocalHubError> {
    from_main(webview.label())?;
    let work = || -> Result<serde_json::Value, LocalHubError> {
        let printed = ask(&carried()?, &[&OsString::from("status")], STATUS_LIMIT)?;
        parse_status(&printed)
    };
    tauri::async_runtime::spawn_blocking(work)
        .await
        .unwrap_or(Err(LocalHubError::Failed))
}
