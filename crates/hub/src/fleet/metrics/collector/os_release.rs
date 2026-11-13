//! The distribution a device's os-release names (DEV-016). Pure.
//!
//! The file comes from the device, so it is read as hostile text: only the last `ID=` line
//! counts, its value may be quoted once with `"` or `'`, and it is kept only when
//! [`Distribution::new`] accepts it. Nothing else of the file is kept.

use crate::fleet::devices::Distribution;

/// The distribution of the last `ID=` line of `os_release`; `None` when there is no such line
/// or its value is not a valid distribution id.
pub(super) fn distribution(os_release: &[u8]) -> Option<Distribution> {
    let value = os_release
        .split(|byte| *byte == b'\n')
        .filter_map(|line| line.trim_ascii().strip_prefix(b"ID="))
        .next_back()?;
    let value = std::str::from_utf8(value).ok()?;
    Distribution::new(unquoted(value))
}

/// `value` without one pair of matching outer quotes.
fn unquoted(value: &str) -> &str {
    ['"', '\'']
        .into_iter()
        .find_map(|quote| value.strip_prefix(quote)?.strip_suffix(quote))
        .unwrap_or(value)
}
