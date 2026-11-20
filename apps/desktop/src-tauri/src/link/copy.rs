//! The copy of the link program that a unit of the user runs (HMD-058): its place in the
//! folder of the app, the place where an earlier app put it, and the copy itself.

use std::{
    fs,
    path::{Path, PathBuf},
};

use super::LinkError;
use crate::daemon::{placement, this_user};

/// Where the copy of the link program is: `<application>/link/program/anima-link`. The app
/// makes `link` with mode 0700, so the link's own check of the program's folder passes on any
/// system, also where the person's `~/.local/bin` can be written by their group (DEF-007).
pub fn destination(application: &Path) -> PathBuf {
    application.join("link/program").join(super::PROGRAM)
}

/// Where an app before DEF-007 put the copy. A unit made then runs it until the app
/// moves the unit to [`destination`].
pub fn earlier_copy(home: &Path) -> PathBuf {
    home.join(".local/bin").join(super::PROGRAM)
}

/// The copy that a unit of this user runs now: the copy at [`destination`], else an earlier
/// copy that is there, else [`destination`].
pub fn current_copy(application: &Path, home: &Path) -> PathBuf {
    let copy = destination(application);
    let earlier = earlier_copy(home);
    if !copy.is_file() && earlier.is_file() {
        earlier
    } else {
        copy
    }
}

/// Whether two files hold the same bytes; false when one cannot be read.
pub fn same(one: &Path, other: &Path) -> bool {
    match (fs::read(one), fs::read(other)) {
        (Ok(one), Ok(other)) => one == other,
        _ => false,
    }
}

/// Copies the carried link program to its place in `application`, unless the copy there
/// already holds the same bytes. Returns the path that the unit runs.
pub fn place(carried: &Path, application: &Path) -> Result<PathBuf, LinkError> {
    let destination = destination(application);
    if same(carried, &destination) {
        return Ok(destination);
    }
    placement::place(carried, &destination, this_user()).map_err(|_| LinkError::Folder)
}

/// Deletes the copy of the link program. A copy that is not there is not an error.
pub fn unplace(program: &Path) {
    let _ = fs::remove_file(program);
}
