//! `anima-local-hub update`: the application starts it at each start, and it replaces the
//! programs of the installed hub only when the application carries other ones (D-6).

use std::io;

#[cfg(test)]
mod tests;

use super::install::{self, InstallError, Request, Then};
use super::steps::Step;
use crate::programs::digest;
use crate::{Carried, stop_marker};

/// Replaces the programs of the installed hub by those of `source` when their digest is not
/// the digest of the copy that runs, and starts the unit, unless the operator stopped the hub.
/// Returns whether the programs were replaced. The database, the key, the link's state and
/// the logs are not touched.
///
/// A hub that is not installed, or a copy that is the same, runs no step.
///
/// # Errors
/// When the digest cannot be made, or as [`install::install`].
pub fn update(
    request: &Request<'_>,
    source: &Carried,
    run: impl FnMut(&Step) -> io::Result<bool>,
) -> Result<bool, InstallError> {
    if !install::is_installed(request) {
        return Ok(false);
    }
    if digest::of(source)?.is_stored_in(request.folders) {
        return Ok(false);
    }
    let then = if stop_marker::is_set(request.folders) {
        Then::StayStopped
    } else {
        Then::Start
    };
    install::replace(request, source, run, then)?;
    Ok(true)
}
