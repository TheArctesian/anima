//! This device's link to a hub (one-click plan S6, 2.6): the shell joins with an invite, runs
//! `anima-link device` while the app runs, and reports its state. For a hosted hub a unit of
//! the user keeps the link running with the app closed, and the shell watches it (HMD-058). The client then reaches the
//! hub at `http://127.0.0.1:<port>`. No rule of the link is here (REPO-003).
//!
//! Only the main web view may ask (WEB-007). The invite is the one value that comes from the
//! web view; its form is checked first, it reaches `anima-link` on standard input only and it
//! is never logged (LOG-007).

mod device;
mod error;
mod folder;
mod joining;
#[cfg(unix)]
mod kept;
mod printed;
mod restarts;
#[cfg(unix)]
mod repair;
mod state;

#[cfg(all(test, unix))]
mod tests;

use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use serde::Serialize;
use tauri::{Runtime, State, Webview};
use tauri_plugin_log::log;

pub use device::DeviceLink;
use device::Launch;
pub use error::LinkError;
use folder::LinkFolders;
use joining::Joined;
#[cfg(unix)]
pub use kept::{daemon_socket, hosted_hub_opened};
#[cfg(unix)]
pub use state::DeviceState;
pub use state::Event;
use state::{Phase, Reason};
#[cfg(unix)]
use std::thread;

const PROGRAM: &str = "anima-link";
const RESTART_AFTER: Duration = Duration::from_secs(10);
const CTL_LIMIT: Duration = Duration::from_secs(3);

/// The one join or leave that may run at a time.
#[derive(Debug, Default)]
pub struct Changing(Arc<AtomicBool>);

/// Holds [`Changing`] for one join or leave; dropping it frees it.
struct Claim(Arc<AtomicBool>);

impl Changing {
    fn claim(&self) -> Result<Claim, LinkError> {
        if self.0.swap(true, Ordering::SeqCst) {
            return Err(LinkError::Busy);
        }
        Ok(Claim(Arc::clone(&self.0)))
    }
}

impl Drop for Claim {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

/// What the client sees of the link.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkStatus {
    pub joined: bool,
    pub state: Phase,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<Reason>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<f64>,
}

fn from_main(label: &str) -> Result<(), LinkError> {
    crate::web_view::caller(label).map_err(|_| LinkError::Caller)
}

fn folders() -> Result<LinkFolders, LinkError> {
    crate::user_folders::of_this_user()
        .map(|application| LinkFolders::below(&application))
        .ok_or(LinkError::NoHome)
}

/// What `anima-link` may see of the app's environment: the home folder and the folder for
/// temporary files.
fn environment(read: impl Fn(&str) -> Option<OsString>) -> Vec<(&'static str, OsString)> {
    #[cfg(unix)]
    let names = ["HOME", "TMPDIR"];
    #[cfg(windows)]
    let names = ["USERPROFILE", "LOCALAPPDATA", "SystemRoot", "TEMP", "TMP"];
    names
        .into_iter()
        .filter_map(|name| Some((name, read(name).filter(|value| !value.is_empty())?)))
        .collect()
}

fn launch(program: PathBuf, folder: &Path, port: u16) -> Launch {
    Launch {
        program,
        folder: folder.to_path_buf(),
        port,
        env: environment(|name| std::env::var_os(name)),
        restart_after: RESTART_AFTER,
    }
}

/// Starts the device link when this device has joined a hub. Called once at the app's start.
/// A join that stopped half way is undone first: the folder before it is put back.
pub fn start(device: &Arc<DeviceLink>) {
    let Ok(folders) = folders() else { return };
    if folders.recover().is_err() {
        log::warn!("the device link's folder before a join could not be put back");
    }
    let program = crate::carried::program(PROGRAM);
    resume(device, program.clone(), &folders);
    // A link that the app runs for a hosted hub goes to a unit of the user (HMD-058), off this
    // thread: the start of the app does not wait for it.
    #[cfg(unix)]
    if let Some(program) = program.filter(|_| folder::wants_unit(&folders.device)) {
        let device = Arc::clone(device);
        thread::spawn(move || kept::hand_over(&device, program, &folders.device));
    }
}

/// Runs the device link of `folders.device` on its saved port, when this device has joined.
fn resume(device: &DeviceLink, program: Option<PathBuf>, folders: &LinkFolders) {
    if !folders.device.is_dir() {
        return;
    }
    let Some(port) = folder::read_port(&folders.device) else {
        device.record(Event::Unable(Reason::NoPort));
        return;
    };
    #[cfg(unix)]
    if let Some(link) = kept::now(&folders.device, port) {
        // The unit runs the copy of an earlier app: it gets the program of this app (HMD-055).
        if let Some(carried) = &program {
            link.refresh(carried);
        }
        link.watch(device);
        return;
    }
    match program {
        Some(program) => device.start(launch(program, &folders.device, port)),
        None => device.record(Event::Unable(Reason::NotCarried)),
    }
}

/// Puts the joined folder in place of the one before and runs its device link. When the swap
/// fails, the link before runs again on its own port. The folder before is removed last; a
/// failure there is logged only, as the join itself succeeded.
fn replace(
    device: &DeviceLink,
    program: PathBuf,
    folders: &LinkFolders,
    port: u16,
) -> Result<(), LinkError> {
    device.stop();
    // A unit of the join before must not run while its folder is swapped.
    #[cfg(unix)]
    if let Some(before) = kept::now(&folders.device, port)
        && before.remove().is_err()
    {
        log::warn!("the unit of the device link before the join was not removed");
    }
    if folders.take_next().is_err() {
        resume(device, Some(program), folders);
        let _ = folders.prepare_next();
        return Err(LinkError::Folder);
    }
    if folders.remove_old().is_err() {
        log::warn!("the device link's folder before the join was not removed");
    }
    device.start(launch(program, &folders.device, port));
    Ok(())
}

async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, LinkError> + Send + 'static,
) -> Result<T, LinkError> {
    tauri::async_runtime::spawn_blocking(work)
        .await
        .unwrap_or(Err(LinkError::Failed))
}

/// Joins this device to the hub of `invite`, replaces the link of an earlier join, and starts
/// the device link. On a hosted hub a unit of the user then keeps the link running with the app
/// closed (HMD-058); when the unit cannot be made, the app runs the link itself as before. Returns the device key's fingerprint and the local port.
#[tauri::command]
pub async fn link_join<R: Runtime>(
    webview: Webview<R>,
    changing: State<'_, Changing>,
    device: State<'_, Arc<DeviceLink>>,
    invite: String,
) -> Result<Joined, LinkError> {
    from_main(webview.label())?;
    let invite = zeroize::Zeroizing::new(invite);
    let line = printed::invite(&invite)?;
    drop(invite);
    let claim = changing.claim()?;
    let device = Arc::clone(&device);
    let result = blocking(move || {
        let _claim = claim;
        let program = crate::carried::program(PROGRAM).ok_or(LinkError::NotCarried)?;
        let folders = folders()?;
        let env = environment(|name| std::env::var_os(name));
        let joined = joining::join(&program, &folders, &line, &env)?;
        replace(&device, program.clone(), &folders, joined.port)?;
        #[cfg(unix)]
        if device.in_background() {
            let _ = folder::want_unit(&folders.device);
            kept::keep_or_run(&device, program, &folders.device, joined.port);
        }
        Ok(joined)
    })
    .await;
    match &result {
        Ok(_) => log::info!("this device joined a hub"),
        Err(error) => log::warn!("this device did not join a hub: {}", error.code()),
    }
    result
}

/// The link's state; its path and latency while it is up.
#[tauri::command]
pub async fn link_status<R: Runtime>(
    webview: Webview<R>,
    device: State<'_, Arc<DeviceLink>>,
) -> Result<LinkStatus, LinkError> {
    from_main(webview.label())?;
    let folders = folders()?;
    let now = device.state();
    let mut status = LinkStatus {
        joined: folders.device.is_dir(),
        state: now.phase,
        port: now.port,
        reason: now.reason,
        path: None,
        latency_ms: None,
    };
    if now.phase == Phase::Up {
        let path = blocking(move || Ok(link_path(&folders.device))).await?;
        (status.path, status.latency_ms) = path.unzip();
    }
    Ok(status)
}

/// `anima-link ctl status` of the running device link: its path and latency.
fn link_path(folder: &Path) -> Option<(String, f64)> {
    let program = crate::carried::program(PROGRAM)?;
    let env = environment(|name| std::env::var_os(name));
    let args = [
        OsString::from("ctl"),
        OsString::from("-state"),
        folder.as_os_str().to_owned(),
        OsString::from("status"),
    ];
    let invocation = crate::carried::run::Invocation {
        program: &program,
        args: &args.each_ref(),
        env: &env,
        limit: CTL_LIMIT,
    };
    let printed = crate::carried::run::run(&invocation).ok()?;
    printed::link_path(&printed)
}

/// Stops the device link and removes its folder with its key: this device leaves the hub.
#[tauri::command]
pub async fn link_leave<R: Runtime>(
    webview: Webview<R>,
    changing: State<'_, Changing>,
    device: State<'_, Arc<DeviceLink>>,
) -> Result<(), LinkError> {
    from_main(webview.label())?;
    let claim = changing.claim()?;
    let device = Arc::clone(&device);
    let result = blocking(move || {
        let _claim = claim;
        let folders = folders()?;
        device.stop();
        #[cfg(unix)]
        if let Some(link) = kept::now(&folders.device, 0) {
            link.remove()?;
            kept::unplace(&link.program);
        }
        folders.remove().map_err(|_| LinkError::Folder)
    })
    .await;
    match &result {
        Ok(()) => log::info!("this device left its hub"),
        Err(error) => log::warn!("this device did not leave its hub: {}", error.code()),
    }
    result
}
