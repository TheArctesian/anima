//! The operator's choice between runs (AWK-001): one small JSON file in the app's
//! configuration folder. A missing or unreadable file is the default: off, no floor.

use std::{
    fs, io,
    path::{Path, PathBuf},
};

use super::level::Wanted;

const FILE: &str = "stay-awake.json";

/// Exists while Anima has changed the macOS sleep setting and not set it back (AWK-007).
const MARKER: &str = "stay-awake-sleep-disabled";

/// The marker file in `folder`.
#[must_use]
pub fn marker(folder: &Path) -> PathBuf {
    folder.join(MARKER)
}

/// The saved choice, with a floor inside the accepted range.
#[must_use]
pub fn load(folder: &Path) -> Wanted {
    let wanted: Wanted = fs::read(folder.join(FILE))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default();
    wanted.with_floor(wanted.battery_floor)
}

/// Saves the choice: written beside the file, then moved over it, so a failure leaves the old one.
///
/// # Errors
/// The I/O error of the folder, the write or the move.
pub fn save(folder: &Path, wanted: Wanted) -> io::Result<()> {
    fs::create_dir_all(folder)?;
    let draft = folder.join(format!("{FILE}.new"));
    fs::write(&draft, serde_json::to_vec(&wanted)?)?;
    fs::rename(draft, folder.join(FILE))
}
