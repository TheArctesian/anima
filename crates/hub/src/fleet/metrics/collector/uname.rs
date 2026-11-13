//! The operating system a device's `uname -s` names (DEV-025). Pure.
//!
//! The answer comes from the device, so it is read as hostile text: only an exact `Linux` or
//! `Darwin`, with the white space around it removed, counts. It is never decoded, logged or
//! stored.

use crate::fleet::devices::OperatingSystem;

/// The operating system `uname` names: `Linux` is Linux and `Darwin` is macOS; `None` for any
/// other answer.
pub(super) fn operating_system(uname: &[u8]) -> Option<OperatingSystem> {
    match uname.trim_ascii() {
        b"Linux" => Some(OperatingSystem::Linux),
        b"Darwin" => Some(OperatingSystem::Macos),
        _ => None,
    }
}
