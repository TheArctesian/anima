//! The address that `install --listen` gave the hub. The unit file names it, and an `update`
//! that writes the unit file again must name the same one.

use std::fs;
use std::io::{self, Write as _};
use std::net::SocketAddr;

use anima_database::owner_only;

use crate::Folders;

/// Records `listen` in the hub's folder, or forgets the recorded one when it is `None`.
///
/// # Errors
/// When the file cannot be written or removed.
pub(super) fn record(folders: &Folders, listen: Option<SocketAddr>) -> io::Result<()> {
    let file = folders.listen_file();
    let Some(listen) = listen else {
        return match fs::remove_file(&file) {
            Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error),
            _ => Ok(()),
        };
    };
    let mut new = owner_only::new_file(&file)?;
    writeln!(new, "{listen}").and_then(|()| new.sync_all())
}

/// The recorded address. A file that holds anything but a loopback socket address counts as
/// none: the unit then has the default (NET-003).
#[must_use]
pub(super) fn recorded(folders: &Folders) -> Option<SocketAddr> {
    let text = fs::read_to_string(folders.listen_file()).ok()?;
    text.trim()
        .parse::<SocketAddr>()
        .ok()
        .filter(|listen| listen.ip().is_loopback())
}
