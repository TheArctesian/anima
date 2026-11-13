//! Reads a channel document from a file or standard input (CMD-025). The document holds the
//! channel's secrets; see `input` for how it is read and why an error never quotes it.

use std::path::Path;

use serde::de::DeserializeOwned;

use crate::failure::Failure;
use crate::input::{self, MAX_DOCUMENT};
use crate::invocation::Invocation;

/// The document in `from` (`-` for standard input) as `T`.
///
/// # Errors
/// [`Failure::Input`] for an unreadable document; [`Failure::Usage`] for an invalid one, or
/// for `-` at a terminal under `--no-input`.
pub fn read<T: DeserializeOwned>(invocation: &Invocation, from: &Path) -> Result<T, Failure> {
    let what = "the channel document";
    let bytes = input::read(invocation.prompter, from, what, MAX_DOCUMENT)?;
    input::json(&bytes, what, "anima notify channel create --help")
}
