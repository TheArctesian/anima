//! Putting the `anima` program the app carries on the operator's `PATH` (INS-002 to INS-009): on
//! macOS a link to `Contents/MacOS/anima` in the app bundle, on Linux a launcher that runs the
//! `AppImage`. Only in a folder this user can write, which the operator confirms; never with an
//! administrator password, and never over a file an Anima app did not make. "Remove" deletes only
//! such a link or launcher.
//!
//! Only a release build carries `anima`: the builder builds it from the same commit with the
//! release version and passes `bundle.externalBin: ["binaries/anima"]` to `tauri build` (BLD-043,
//! OD-17). A debug build says it carries none.
//!
//! Only the main web view may ask (WEB-007). It passes only the folder it was shown, which must
//! be the one proposed now. Nothing here logs a path (LOG-007).

mod carrier;
mod directory;
pub mod dispatch;
mod error;
mod found;
mod launcher;
mod login_path;
mod placing;
mod state;

#[cfg(test)]
mod tests;

use std::{
    io,
    path::{Path, PathBuf},
};

use tauri::{Runtime, Webview};
use tauri_plugin_log::log;

use carrier::{Carrier, PROGRAM};
use directory::Proposal;
pub use error::CommandLineError;
use found::look;
use state::{Fit, Status, fit, status};

/// This computer as the commands need it: the carrier, the home folder and the `PATH` folders.
struct Here {
    carrier: Carrier,
    home: PathBuf,
    path: Vec<PathBuf>,
}

impl Here {
    /// What each candidate folder's `anima` is to this app, in candidate order.
    fn fits(&self) -> Vec<(PathBuf, Fit)> {
        directory::candidates(&self.home)
            .into_iter()
            .map(|folder| {
                let path = folder.join(PROGRAM);
                let fit = fit(look(&path), &self.carrier);
                (path, fit)
            })
            .collect()
    }

    fn proposal(&self) -> Proposal {
        directory::propose(&self.home, &self.path, writable)
    }

    fn status(&self) -> Status {
        status(&self.fits(), &self.proposal())
    }
}

/// Whether `folder` is an existing folder this user may write.
fn writable(folder: &Path) -> bool {
    folder.is_dir() && nix::unistd::access(folder, nix::unistd::AccessFlags::W_OK).is_ok()
}

/// The carrier of the running app: the macOS bundle or the Linux `AppImage`. A debug build carries
/// none: its target folder may hold an `anima` built from another commit.
fn carrier() -> Result<Carrier, CommandLineError> {
    if cfg!(debug_assertions) {
        return Err(CommandLineError::NotBundled);
    }
    let executable = std::env::current_exe()
        .and_then(std::fs::canonicalize)
        .map_err(|_| CommandLineError::NotBundled)?;
    let program_exists = carrier::beside(&executable).is_some_and(|program| program.is_file());
    if cfg!(target_os = "macos") {
        carrier::bundle(&executable, program_exists)
    } else if cfg!(target_os = "linux") {
        let appimage = std::env::var_os("APPIMAGE");
        let appdir = std::env::var_os("APPDIR");
        carrier::app_image(
            appimage.as_deref(),
            appdir.as_deref(),
            &executable,
            program_exists,
        )
    } else {
        Err(CommandLineError::Unsupported)
    }
}

/// The user's login shell from the account database, else `$SHELL`.
fn login_shell() -> Option<PathBuf> {
    nix::unistd::User::from_uid(nix::unistd::getuid())
        .ok()
        .flatten()
        .map(|user| user.shell)
        .or_else(|| std::env::var_os("SHELL").map(PathBuf::from))
        .filter(|shell| shell.is_absolute() && shell.is_file())
}

async fn here() -> Result<Here, CommandLineError> {
    let carrier = carrier()?;
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .filter(|home| home.is_absolute())
        .ok_or(CommandLineError::NoHome)?;
    let path = login_path::folders(login_shell().as_deref(), std::env::var_os("PATH")).await;
    Ok(Here {
        carrier,
        home,
        path,
    })
}

/// Puts this app's `anima` in `folder`, which must be the proposed folder or the folder of an
/// Anima app's own entry. Refuses anything an Anima app did not make.
fn install_in(here: &Here, folder: &Path) -> Result<(), CommandLineError> {
    let proposal = here.proposal();
    let fits = here.fits();
    let destination = folder.join(PROGRAM);
    let ours = fits
        .iter()
        .any(|(path, fit)| *path == destination && matches!(fit, Fit::Current | Fit::Elsewhere(_)));
    if folder != proposal.folder && !ours {
        return Err(CommandLineError::Changed);
    }
    if folder == proposal.folder && proposal.create {
        placing::make_folder(folder).map_err(|_| CommandLineError::Unwritable)?;
    }
    let replace = match fit(look(&destination), &here.carrier) {
        Fit::Current => return Ok(()),
        Fit::Foreign(occupant) => return Err(CommandLineError::Occupied(occupant)),
        Fit::Absent => false,
        Fit::Elsewhere(_) => true,
    };
    placing::put(&destination, &here.carrier, replace).map_err(|error| {
        // Something appeared there since the look: say what it is.
        match (error.kind(), fit(look(&destination), &here.carrier)) {
            (io::ErrorKind::AlreadyExists, Fit::Foreign(occupant)) => {
                CommandLineError::Occupied(occupant)
            }
            _ => CommandLineError::Unwritable,
        }
    })
}

/// Removes every link or launcher an Anima app made in the candidate folders.
fn remove_all(here: &Here) -> Result<(), CommandLineError> {
    for (path, fit) in here.fits() {
        if matches!(fit, Fit::Current | Fit::Elsewhere(_)) {
            placing::take_away(&path).map_err(|_| CommandLineError::Unwritable)?;
        }
    }
    Ok(())
}

/// Refuses every caller but the main web view; the capability already does, this is the second
/// wall (WEB-007).
fn from_main(label: &str) -> Result<(), CommandLineError> {
    crate::web_view::caller(label).map_err(|_| CommandLineError::Caller)
}

/// The state for the row: unavailable with the reason, or what [`state::status`] says.
async fn read() -> Status {
    match here().await {
        Ok(here) => here.status(),
        Err(error) => Status::Unavailable {
            reason: error.code(),
        },
    }
}

/// Runs `change` off the async runtime, then reads the state again.
async fn changed(
    change: impl FnOnce(&Here) -> Result<(), CommandLineError> + Send + 'static,
) -> Result<Status, CommandLineError> {
    let here = here().await?;
    tauri::async_runtime::spawn_blocking(move || change(&here).map(|()| here.status()))
        .await
        .unwrap_or(Err(CommandLineError::Failed))
}

/// The state of the command line tool on this computer (INS-006).
#[tauri::command]
pub async fn command_line_read<R: Runtime>(
    webview: Webview<R>,
) -> Result<Status, CommandLineError> {
    from_main(webview.label())?;
    Ok(read().await)
}

/// Installs the command line tool in `folder`, the one the operator confirmed (INS-004, INS-005).
#[tauri::command]
pub async fn command_line_install<R: Runtime>(
    webview: Webview<R>,
    folder: String,
) -> Result<Status, CommandLineError> {
    from_main(webview.label())?;
    let folder = PathBuf::from(folder);
    let result = changed(move |here| install_in(here, &folder)).await;
    match &result {
        Ok(_) => log::info!("the command line tool was installed"),
        Err(error) => log::warn!("the command line tool was not installed: {}", error.code()),
    }
    result
}

/// Removes the command line tool's link or launcher (INS-006).
#[tauri::command]
pub async fn command_line_remove<R: Runtime>(
    webview: Webview<R>,
) -> Result<Status, CommandLineError> {
    from_main(webview.label())?;
    let result = changed(remove_all).await;
    match &result {
        Ok(_) => log::info!("the command line tool was removed"),
        Err(error) => log::warn!("the command line tool was not removed: {}", error.code()),
    }
    result
}
