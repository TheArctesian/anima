//! `anima-local-hub update`: the application starts it at each start, and it replaces the
//! programs of the installed hub only when the application carries other ones (D-6) and
//! not older ones.

use std::io;

#[cfg(all(test, unix))]
mod tests;

use super::install::{self, InstallError, Request, Then};
use super::steps::Step;
use super::{change_lock, listen};
use crate::programs::digest;
use crate::programs::{self, version};
use crate::{Carried, run, stop_marker};

/// What an update found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// The programs of `source` are in place.
    Replaced,
    /// Nothing was replaced: no hub is installed, or the copy is the same.
    Unchanged,
    /// The copy that runs is of a higher version than the programs that `source` holds, so
    /// the data folder may be newer than they are. Nothing was changed.
    Older { carried: String, installed: String },
}

impl Outcome {
    #[must_use]
    pub const fn replaced(&self) -> bool {
        matches!(self, Self::Replaced)
    }

    /// Why nothing was replaced, when the programs of the application were refused.
    #[must_use]
    pub fn refusal(&self) -> Option<String> {
        match self {
            Self::Older { carried, installed } => Some(format!(
                "the application carries version {carried}, and the hub runs version {installed}"
            )),
            _ => None,
        }
    }

    /// The one line that `update` prints: `updated` first, and `older` when the programs of
    /// the application were refused for their version.
    #[must_use]
    pub const fn json_line(&self) -> &'static str {
        match self {
            Self::Replaced => "{\"updated\":true}",
            Self::Unchanged => "{\"updated\":false}",
            Self::Older { .. } => "{\"updated\":false,\"older\":true}",
        }
    }
}

/// Replaces the programs of the installed hub by those of `source` when their digest is not
/// the digest of the copy that runs and their version (`request.version`) is not lower than
/// the version of that copy. It starts the unit, unless the operator stopped the hub, and
/// writes the unit file with the address that `install` recorded. The database, the key, the
/// link's state and the logs are not touched.
///
/// A hub that is not installed runs no step. A same digest never rewrites the unit file. When the copy is the same, the unit is started
/// if nothing runs, the operator did not stop the hub and the last start failed: a start that
/// failed after a replacement is tried again.
///
/// # Errors
/// [`InstallError::Busy`] when another change runs; when the digest cannot be made; or as
/// [`install::install`].
pub fn update(
    request: &Request<'_>,
    source: &Carried,
    mut run: impl FnMut(&Step) -> io::Result<bool>,
) -> Result<Outcome, InstallError> {
    // No hub, no folder: an update makes nothing where nothing was installed.
    if request.folders.root().symlink_metadata().is_err() {
        return Ok(Outcome::Unchanged);
    }
    let _change = change_lock::take(request.folders)?;
    install::check_private(request)?;
    programs::recover(request.folders)?;
    if !install::is_installed(request) {
        return Ok(Outcome::Unchanged);
    }
    let request = Request {
        listen: listen::recorded(request.folders),
        ..request.clone()
    };
    let folders = request.folders;
    if digest::of(source)?.is_stored_in(folders) {
        if !stop_marker::is_set(folders) && !run::is_running(folders) {
            install::start_unit(&request, run)?;
        }
        return Ok(Outcome::Unchanged);
    }
    let installed = version::installed(folders);
    if version::is_older(request.version, installed.as_deref()) {
        return Ok(Outcome::Older {
            carried: request.version.to_owned(),
            installed: installed.unwrap_or_default(),
        });
    }
    let then = if stop_marker::is_set(folders) {
        Then::StayStopped
    } else {
        Then::Start
    };
    install::replace(&request, source, &mut run, then)?;
    Ok(Outcome::Replaced)
}
