//! The device link's folders and its saved port (F-2, F-7).
//!
//! ```text
//! <application>/link/device/        the state folder of `anima-link device`
//! <application>/link/device/anima-port   the local port chosen at the join
//! <application>/link/device.next/   a join in progress; replaces `device` when it succeeds
//! <application>/link/device.old/    the folder before the last join, while it is removed
//! ```

use std::{
    fs::{self, DirBuilder, OpenOptions},
    io::{self, ErrorKind, Read, Write},
    os::unix::fs::{DirBuilderExt, OpenOptionsExt},
    path::{Path, PathBuf},
};

const PORT_FILE: &str = "anima-port";
/// More than a port number and a line end.
const MOST_PORT_FILE: u64 = 16;

/// The folders of the device link below the application's folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkFolders {
    pub device: PathBuf,
    pub next: PathBuf,
    old: PathBuf,
}

impl LinkFolders {
    pub fn below(application: &Path) -> Self {
        let link = application.join("link");
        Self {
            device: link.join("device"),
            next: link.join("device.next"),
            old: link.join("device.old"),
        }
    }

    /// Makes the folders above the state folders, owner-only (0700) where it makes them, and
    /// removes a join that did not finish.
    pub fn prepare_next(&self) -> io::Result<()> {
        let link = self.device.parent().ok_or(ErrorKind::InvalidInput)?;
        DirBuilder::new().recursive(true).mode(0o700).create(link)?;
        remove_folder(&self.next)
    }

    /// The new folder takes the place of the one before, which is removed.
    pub fn take_next(&self) -> io::Result<()> {
        remove_folder(&self.old)?;
        match fs::rename(&self.device, &self.old) {
            Ok(()) => {}
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
        fs::rename(&self.next, &self.device)?;
        remove_folder(&self.old)
    }

    /// Removes every folder of the device link.
    pub fn remove(&self) -> io::Result<()> {
        remove_folder(&self.next)?;
        remove_folder(&self.old)?;
        remove_folder(&self.device)
    }
}

/// Removes a folder and what is in it; one that is not there is not an error. A symbolic link
/// is removed, not followed.
fn remove_folder(folder: &Path) -> io::Result<()> {
    match fs::symlink_metadata(folder) {
        Ok(found) if found.is_dir() => fs::remove_dir_all(folder),
        Ok(_) => fs::remove_file(folder),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

/// The port in the port file: decimal digits and an optional line end, 1 to 65535.
pub fn parse_port(text: &[u8]) -> Option<u16> {
    let digits = text.strip_suffix(b"\n").unwrap_or(text);
    let fits = !digits.is_empty() && digits.len() <= 5 && digits.iter().all(u8::is_ascii_digit);
    if !fits {
        return None;
    }
    std::str::from_utf8(digits)
        .ok()?
        .parse()
        .ok()
        .filter(|port| *port != 0)
}

/// The saved port of the state folder `folder`. A symbolic link is not followed.
pub fn read_port(folder: &Path) -> Option<u16> {
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(nix::libc::O_NOFOLLOW)
        .open(folder.join(PORT_FILE))
        .ok()?;
    let mut text = Vec::new();
    file.take(MOST_PORT_FILE).read_to_end(&mut text).ok()?;
    parse_port(&text)
}

/// Saves `port` in the new state folder `folder`, in a new owner-only file (0600).
pub fn write_port(folder: &Path, port: u16) -> io::Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(folder.join(PORT_FILE))?;
    writeln!(file, "{port}")?;
    file.sync_all()
}
