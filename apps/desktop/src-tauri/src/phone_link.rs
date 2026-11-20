//! This phone's link to a hub (mobile plan M3-1; MOB-031, MOB-032, MOB-035, MOB-036). A phone cannot start
//! `anima-link`, so the Go link runs inside the app as a library. The commands are the desktop link's:
//! `link_join`, `link_status` and `link_leave`, with the same answers, so the client reaches the hub at
//! `http://127.0.0.1:<port>` on every platform. No rule of the link is here (REPO-003).
//!
//! The invite is the one value that comes from the web view. It reaches the library in memory only, is
//! wiped after the call and is never logged (MOB-035, LOG-007).

#[expect(
    dead_code,
    reason = "the desktop link's NotCarried error does not happen inside the app"
)]
#[path = "link/error.rs"]
mod error;
mod library;

use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::Mutex,
};

use serde::Serialize;
use tauri::{AppHandle, Manager, Runtime, State, Webview};

pub use error::LinkError;

/// The folders of this phone's link, in the app's own data folder.
pub struct PhoneLink {
    /// The identity of the device that joined; there only after a join.
    folder: PathBuf,
    /// The folder of a join in progress.
    next: PathBuf,
    /// The local port, kept so that the hub's address, and the token kept for it, stay the same.
    port: PathBuf,
    /// The one join or leave that may run at a time.
    changing: Mutex<()>,
}

/// What a join gives, as on the desktop: the new key's short form and the local port.
#[derive(Debug, Serialize)]
pub struct Joined {
    fingerprint: String,
    port: u16,
}

/// The link's state, as the desktop's `link_status` gives it.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkStatus {
    joined: bool,
    state: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    port: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    public_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    latency_ms: Option<f64>,
}

impl PhoneLink {
    /// The link's folders below the app's data folder `data`.
    pub fn below(data: &Path) -> Self {
        Self {
            folder: data.join("link"),
            next: data.join("link-next"),
            port: data.join("link-port"),
            changing: Mutex::new(()),
        }
    }

    fn joined(&self) -> bool {
        self.folder.is_dir()
    }

    fn saved_port(&self) -> Option<u16> {
        fs::read_to_string(&self.port).ok()?.trim().parse().ok()
    }

    /// Starts the link of the device that joined; a link that runs stays. The saved port first, so that the
    /// address does not change; a port that another program holds is replaced and saved (MOB-036).
    pub fn start(&self) -> Result<u16, LinkError> {
        if !self.joined() {
            return Err(LinkError::Folder);
        }
        let saved = self.saved_port();
        let local =
            match saved.map(|port| library::start(&self.folder, &format!("127.0.0.1:{port}"))) {
                Some(Ok(local)) => local,
                _ => library::start(&self.folder, "127.0.0.1:0")?,
            };
        let port = port_of(&local).ok_or(LinkError::Failed)?;
        if saved != Some(port) {
            fs::write(&self.port, port.to_string()).map_err(|_| LinkError::Folder)?;
        }
        Ok(port)
    }

    /// Joins with `invite` into a new folder, then puts it in place of the old one and starts the link.
    fn join(&self, invite: &str) -> Result<Joined, LinkError> {
        let _changing = self.changing.try_lock().map_err(|_| LinkError::Busy)?;
        library::stop();
        remove(&self.next).map_err(|_| LinkError::Folder)?;
        let joined = library::join(&self.next, invite);
        let fingerprint = match joined {
            Ok(fingerprint) => fingerprint,
            Err(error) => {
                let _ = remove(&self.next);
                let _ = self.start();
                return Err(error);
            }
        };
        remove(&self.folder).map_err(|_| LinkError::Folder)?;
        fs::rename(&self.next, &self.folder).map_err(|_| LinkError::Folder)?;
        let _ = fs::remove_file(&self.port);
        let port = self.start()?;
        Ok(Joined { fingerprint, port })
    }

    fn status(&self) -> LinkStatus {
        let running = library::status();
        let port = running.up.then(|| port_of(&running.local)).flatten();
        LinkStatus {
            joined: self.joined(),
            state: if running.up { "up" } else { "stopped" },
            port,
            public_key: (running.up && !running.public_key.is_empty())
                .then_some(running.public_key),
            path: running.path.filter(|_| running.up),
            latency_ms: running.latency_ms.filter(|_| running.up),
        }
    }

    fn leave(&self) -> Result<(), LinkError> {
        let _changing = self.changing.try_lock().map_err(|_| LinkError::Busy)?;
        library::stop();
        let _ = fs::remove_file(&self.port);
        remove(&self.folder).map_err(|_| LinkError::Folder)
    }
}

/// The port of a loopback address such as `127.0.0.1:41234`.
fn port_of(local: &str) -> Option<u16> {
    local
        .rsplit_once(':')?
        .1
        .parse()
        .ok()
        .filter(|port| *port != 0)
}

/// Removes `folder` and what it holds; a folder that is not there is not an error.
fn remove(folder: &Path) -> io::Result<()> {
    match fs::remove_dir_all(folder) {
        Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error),
        _ => Ok(()),
    }
}

/// Keeps the app's link and starts it when this phone joined a hub. An app with no data folder has no link.
pub fn manage<R: Runtime>(app: &AppHandle<R>) {
    if let Ok(data) = app.path().app_local_data_dir() {
        let link = PhoneLink::below(&data);
        if link.joined() {
            let _ = link.start();
        }
        app.manage(link);
    }
}

/// Starts the link again when the app comes back from the background, where iOS stops it (MOB-036).
pub fn resume<R: Runtime>(app: &AppHandle<R>) {
    if let Some(link) = app.try_state::<PhoneLink>()
        && link.joined()
        && !library::status().up
    {
        let _ = link.start();
    }
}

async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, LinkError> + Send + 'static,
) -> Result<T, LinkError> {
    tauri::async_runtime::spawn_blocking(work)
        .await
        .unwrap_or(Err(LinkError::Failed))
}

#[tauri::command]
pub async fn link_join<R: Runtime>(app: AppHandle<R>, invite: String) -> Result<Joined, LinkError> {
    let invite = zeroize::Zeroizing::new(invite);
    blocking(move || {
        let link = app.try_state::<PhoneLink>().ok_or(LinkError::NoHome)?;
        link.join(&invite)
    })
    .await
}

#[tauri::command]
#[expect(
    clippy::needless_pass_by_value,
    reason = "a Tauri command takes its arguments by value"
)]
pub fn link_status(link: State<'_, PhoneLink>) -> LinkStatus {
    link.status()
}

/// The key of the link that owns this origin. A reused port must not give another hub an old token.
pub(crate) fn listener_key(origin: &str) -> Option<String> {
    let running = library::status();
    (running.up
        && !running.device_key.is_empty()
        && origin.strip_prefix("http://") == Some(running.local.as_str()))
    .then_some(running.device_key)
}

#[tauri::command]
pub async fn hub_listener_check(
    webview: Webview,
    origin: String,
) -> Result<&'static str, LinkError> {
    if webview.label() != "main" {
        return Err(LinkError::Caller);
    }
    blocking(move || {
        Ok(if listener_key(&origin).is_some() {
            "own"
        } else {
            "none"
        })
    })
    .await
}

#[tauri::command]
pub async fn link_leave<R: Runtime>(app: AppHandle<R>) -> Result<(), LinkError> {
    blocking(move || {
        let link = app.try_state::<PhoneLink>().ok_or(LinkError::NoHome)?;
        link.leave()
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::port_of;

    #[test]
    fn m3_1_the_port_is_read_from_the_local_address() {
        assert_eq!(port_of("127.0.0.1:41234"), Some(41234));
        assert_eq!(port_of("127.0.0.1:0"), None);
        assert_eq!(port_of("nothing"), None);
    }
}
