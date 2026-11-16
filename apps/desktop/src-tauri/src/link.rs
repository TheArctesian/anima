//! This device's link to a hub (one-click plan S6, 2.6): the shell joins with an invite, runs
//! `anima-link device` while the app runs, and reports its state. The client then reaches the
//! hub at `http://127.0.0.1:<port>`. No rule of the link is here (REPO-003).
//!
//! Only the main web view may ask (WEB-007). The invite is the one value that comes from the
//! web view; its form is checked first, it reaches `anima-link` on standard input only and it
//! is never logged (LOG-007).

mod device;
mod error;
mod folder;
mod joining;
mod printed;
mod state;

#[cfg(test)]
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
use state::{Event, Phase, Reason};

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
    ["HOME", "TMPDIR"]
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
pub fn start(device: &DeviceLink) {
    let Ok(folders) = folders() else { return };
    if !folders.device.is_dir() {
        return;
    }
    let Some(port) = folder::read_port(&folders.device) else {
        device.record(Event::Unable(Reason::NoPort));
        return;
    };
    match crate::carried::program(PROGRAM) {
        Some(program) => device.start(launch(program, &folders.device, port)),
        None => device.record(Event::Unable(Reason::NotCarried)),
    }
}

async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, LinkError> + Send + 'static,
) -> Result<T, LinkError> {
    tauri::async_runtime::spawn_blocking(work)
        .await
        .unwrap_or(Err(LinkError::Failed))
}

/// Joins this device to the hub of `invite`, replaces the link of an earlier join, and starts
/// the device link. Returns the device key's fingerprint and the local port.
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
        device.stop();
        folders.take_next().map_err(|_| LinkError::Folder)?;
        device.start(launch(program, &folders.device, joined.port));
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
        folders.remove().map_err(|_| LinkError::Folder)
    })
    .await;
    match &result {
        Ok(()) => log::info!("this device left its hub"),
        Err(error) => log::warn!("this device did not leave its hub: {}", error.code()),
    }
    result
}
