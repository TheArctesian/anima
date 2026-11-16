//! The version of the programs in a copy. An update never puts an older set over a newer one:
//! another copy of the application, also an older one, starts `update` as well.

use std::fs;
use std::path::Path;

use super::{CopyError, io_error};
use crate::Folders;

/// The version of this program, and so of the set it carries: the release builder gives
/// `ANIMA_VERSION` to the cargo build, and any other build has the version of the package.
pub const THIS: &str = match option_env!("ANIMA_VERSION") {
    Some(version) => version,
    None => env!("CARGO_PKG_VERSION"),
};

const FILE: &str = "version";

/// Three numbers: major, minor and patch.
fn numbers(text: &str) -> Option<[u64; 3]> {
    let mut parts = text.trim().split('.').map(|part| part.parse::<u64>().ok());
    let found = [parts.next()??, parts.next()??, parts.next()??];
    parts.next().is_none().then_some(found)
}

/// Whether `carried` is lower than `installed`, the two compared as numbers. An installed
/// version that is missing or is not three numbers blocks nothing, nor does a carried one
/// that is not three numbers.
#[must_use]
pub fn is_older(carried: &str, installed: Option<&str>) -> bool {
    match (numbers(carried), installed.and_then(numbers)) {
        (Some(carried), Some(installed)) => carried < installed,
        _ => false,
    }
}

/// The version of the copy in `programs/current`, when it has a version file.
#[must_use]
pub fn installed(folders: &Folders) -> Option<String> {
    let text = fs::read_to_string(folders.programs().join(FILE)).ok()?;
    Some(text.trim().to_owned())
}

/// Writes the version file of the copy at `next`.
pub(crate) fn write(next: &Path, version: &str) -> Result<(), CopyError> {
    let path = next.join(FILE);
    fs::write(&path, format!("{version}\n")).map_err(io_error("write", &path))
}
