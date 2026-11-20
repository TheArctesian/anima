//! The device link kept by a unit of the user (HMD-058, R-240): the link program is copied out
//! of the application to `~/.local/bin`, `anima-link unit install` writes and starts the unit,
//! and the app watches the running link in place of a child of its own. The link then runs
//! while the person is logged in, with the app closed. No rule of the unit is here: the link
//! program has them (`link/unit`).

use std::{
    ffi::OsString,
    fs::{self, DirBuilder},
    os::unix::fs::{DirBuilderExt, FileTypeExt, MetadataExt},
    path::{Path, PathBuf},
    time::Duration,
};

use std::{sync::Arc, thread, time::Instant};

use tauri::{Manager, Runtime, Webview};
use tauri_plugin_log::log;

use super::{LinkError, device::DeviceLink, folder, folders, launch, printed};
use crate::carried::run::{Invocation, run};
use crate::daemon::{placement, this_user};

/// `unit install` runs up to three steps of 30 s each, one of them twice.
const UNIT_LIMIT: Duration = Duration::from_secs(150);
const ASK_LIMIT: Duration = Duration::from_secs(3);
/// How long the link of a new unit has to answer on its port.
const READY_LIMIT: Duration = Duration::from_secs(20);
const READY_POLL: Duration = Duration::from_millis(250);

/// One device link for a unit: the copy of the program that the unit runs, its state folder
/// and its port.
#[derive(Debug, Clone)]
pub struct Kept {
    pub program: PathBuf,
    pub folder: PathBuf,
    pub port: u16,
    /// The socket that the link makes for the daemon of this user (K8 Section 1, HMD-054); none
    /// when its folder cannot be a private one.
    pub daemon_socket: Option<PathBuf>,
    pub env: Vec<(&'static str, OsString)>,
}

/// The name of the socket in the folder of the daemon, as the daemon knows it.
const DAEMON_SOCKET: &str = "link.sock";
/// The longest path that the link takes for a socket.
const LONGEST_SOCKET: usize = 100;

/// The folder of the daemon of this user, where the daemon itself looks for it: on macOS
/// `~/Library/Application Support/anima-daemon`, else `$XDG_CONFIG_HOME/anima-daemon` or
/// `~/.config/anima-daemon`.
pub fn daemon_folder(home: &Path, macos: bool, config_home: Option<&Path>) -> PathBuf {
    if macos {
        return home.join("Library/Application Support/anima-daemon");
    }
    config_home
        .filter(|folder| folder.is_absolute())
        .map_or_else(|| home.join(".config"), Path::to_path_buf)
        .join("anima-daemon")
}

/// The socket for the daemon in `folder`, once the folder is there and private: it is made
/// with mode 0700 when it is absent. None when the folder is a link, belongs to a different
/// user or gives its group or others any access, and when the path is too long for a socket:
/// the link would refuse it, and the link must still start.
pub fn private_socket(folder: &Path) -> Option<PathBuf> {
    let _ = DirBuilder::new().recursive(true).mode(0o700).create(folder);
    let found = fs::symlink_metadata(folder).ok()?;
    let private = found.is_dir()
        && found.uid() == nix::unistd::getuid().as_raw()
        // No bit of the group or of others: the last two octal digits are zero.
        && found.mode() % 0o100 == 0;
    let socket = folder.join(DAEMON_SOCKET);
    (private && socket.as_os_str().len() <= LONGEST_SOCKET).then_some(socket)
}

/// Where the copy of the link program is: `~/.local/bin/anima-link` under `home`.
pub fn destination(home: &Path) -> PathBuf {
    home.join(".local/bin").join(super::PROGRAM)
}

/// What `anima-link unit` may see of the app's environment: the home folder, and on Linux the
/// folders that name the configuration and the session of the user.
pub fn environment(read: impl Fn(&str) -> Option<OsString>) -> Vec<(&'static str, OsString)> {
    ["HOME", "TMPDIR", "XDG_CONFIG_HOME", "XDG_RUNTIME_DIR"]
        .into_iter()
        .filter_map(|name| Some((name, read(name).filter(|value| !value.is_empty())?)))
        .collect()
}

fn ask(kept: &Kept, args: &[&str], limit: Duration) -> Option<Vec<u8>> {
    let args: Vec<OsString> = args.iter().map(OsString::from).collect();
    let args: Vec<&OsString> = args.iter().collect();
    let invocation = Invocation {
        program: &kept.program,
        args: &args,
        env: &kept.env,
        limit,
    };
    run(&invocation).ok()
}

impl Kept {
    /// Writes the unit and starts it; a unit that runs is started again with this program.
    pub fn install(&self) -> Result<(), LinkError> {
        let folder = self.folder.to_str().ok_or(LinkError::Folder)?;
        let listen = format!("127.0.0.1:{}", self.port);
        let mut args = vec!["unit", "install", "-state", folder, "-listen", &listen];
        if let Some(socket) = self.daemon_socket.as_deref().and_then(Path::to_str) {
            args.extend(["-daemon-socket", socket]);
        }
        ask(self, &args, UNIT_LIMIT)
            .map(drop)
            .ok_or(LinkError::Failed)
    }

    /// Stops the unit and deletes its file. A unit that is not there is not an error.
    pub fn remove(&self) -> Result<(), LinkError> {
        ask(self, &["unit", "remove"], UNIT_LIMIT)
            .map(drop)
            .ok_or(LinkError::Failed)
    }

    /// Whether the unit of the user is installed.
    pub fn installed(&self) -> bool {
        ask(self, &["unit", "show"], ASK_LIMIT).is_some_and(|shown| printed::unit_installed(&shown))
    }

    /// The port that the running link listens on; none while no link runs on this folder.
    pub fn serving(&self) -> Option<u16> {
        let folder = self.folder.to_str()?;
        let status = ask(self, &["ctl", "-state", folder, "status"], ASK_LIMIT)?;
        printed::link_local(&status)
    }

    /// Whether the link of the unit made the socket of the daemon.
    fn has_daemon_socket(&self) -> bool {
        self.daemon_socket.as_deref().is_some_and(|socket| {
            fs::symlink_metadata(socket).is_ok_and(|found| found.file_type().is_socket())
        })
    }

    /// Gives the unit the link program of this app when its copy holds different bytes, and
    /// starts the unit again when the program changed or when the unit was made before the
    /// socket of the daemon (HMD-054). A copy that cannot be written stays, and the unit runs on.
    pub fn refresh(&self, carried: &Path) {
        let changed = !same(carried, &self.program);
        if !changed && (self.daemon_socket.is_none() || self.has_daemon_socket()) {
            return;
        }
        let Some(home) = crate::user_folders::home() else {
            return;
        };
        if !changed || place(carried, &home).is_ok() {
            let _ = self.install();
        }
    }

    /// Has the app watch the running link.
    pub fn watch(self, device: &DeviceLink) {
        let port = self.port;
        device.watch(port, move || self.serving());
    }
}

/// Whether two files hold the same bytes; false when one cannot be read.
fn same(one: &Path, other: &Path) -> bool {
    match (fs::read(one), fs::read(other)) {
        (Ok(one), Ok(other)) => one == other,
        _ => false,
    }
}

/// Copies the carried link program to its place under `home`, unless the copy there already
/// holds the same bytes. Returns the path that the unit runs.
pub fn place(carried: &Path, home: &Path) -> Result<PathBuf, LinkError> {
    let destination = destination(home);
    if same(carried, &destination) {
        return Ok(destination);
    }
    placement::place(carried, &destination, this_user()).map_err(|_| LinkError::Folder)
}

/// Deletes the copy of the link program. A copy that is not there is not an error.
pub fn unplace(program: &Path) {
    let _ = fs::remove_file(program);
}

/// The link of `folder` as a unit of the user keeps it, with the copy of the program in the
/// home folder.
fn at(folder: &Path, port: u16) -> Result<Kept, LinkError> {
    let home = crate::user_folders::home().ok_or(LinkError::NoHome)?;
    let config_home = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from);
    let daemon = daemon_folder(&home, cfg!(target_os = "macos"), config_home.as_deref());
    Ok(Kept {
        program: destination(&home),
        folder: folder.to_path_buf(),
        port,
        daemon_socket: private_socket(&daemon),
        env: environment(|name| std::env::var_os(name)),
    })
}

/// The link of `folder` when a unit of the user keeps it now.
pub fn now(folder: &Path, port: u16) -> Option<Kept> {
    at(folder, port)
        .ok()
        .filter(|link| link.program.is_file() && link.installed())
}

/// Waits until the link of the unit answers on its port.
fn answers(link: &Kept, limit: Duration) -> bool {
    let end = Instant::now() + limit;
    loop {
        if link.serving() == Some(link.port) {
            return true;
        }
        if Instant::now() >= end {
            return false;
        }
        thread::sleep(READY_POLL);
    }
}

/// Hands the link of `folder` to a unit of the user (HMD-058): the program is copied out of the
/// app, the child of the app stops, the unit starts, and the app watches it once it answers. A
/// unit whose link does not answer is removed again.
fn keep(device: &DeviceLink, program: &Path, folder: &Path, port: u16) -> Result<(), LinkError> {
    let home = crate::user_folders::home().ok_or(LinkError::NoHome)?;
    let link = Kept {
        program: place(program, &home)?,
        ..at(folder, port)?
    };
    device.stop();
    let started = link.install();
    if started.is_err() || !answers(&link, READY_LIMIT) {
        let _ = link.remove();
        return Err(started.err().unwrap_or(LinkError::Failed));
    }
    link.watch(device);
    Ok(())
}

/// Hands the link to a unit of the user; when that fails, the app runs the link again, so it
/// works while the app is open.
pub fn keep_or_run(device: &DeviceLink, program: PathBuf, folder: &Path, port: u16) {
    if keep(device, &program, folder, port).is_err() {
        log::warn!("the device link was not handed to a unit of the user");
        device.start(launch(program, folder, port));
    }
}

/// Hands the link that the app runs for `folder` to a unit of the user, unless one keeps it.
pub fn hand_over(device: &DeviceLink, program: PathBuf, folder: &Path) {
    let Some(port) = folder::read_port(folder) else {
        return;
    };
    if now(folder, port).is_none() {
        keep_or_run(device, program, folder, port);
    }
}

/// A hosted hub gave this device its access (HMD-058): from now a unit of the user keeps the
/// link of this device. A join that follows hands the link over. With `joined`, the device is
/// on the link of the hub already: its folder is marked, and the next start of the app hands
/// the link over, so that the sign-in of now is not cut.
pub fn hosted_hub_opened<R: Runtime>(webview: &Webview<R>, joined: bool) {
    webview.state::<Arc<DeviceLink>>().keep_in_background();
    if joined
        && let Ok(folders) = folders()
        && folder::want_unit(&folders.device).is_err()
    {
        log::warn!("the folder of the device link could not be marked for a unit");
    }
}
