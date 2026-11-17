//! The text an inventory may hold, and the readers of the operating system version (K8
//! Section 4, CAP-026). Every value is held to a short fixed alphabet, so no text from a device
//! reaches the hub that could hold a control character or a sentence.

/// The longest version, tool name, build target or operating system version, in bytes.
pub const MAX_TEXT: usize = 64;

/// A version: 1 to 64 bytes of ASCII letters, digits and `.`, `+`, `_`, `-`.
#[must_use]
pub fn is_version(text: &str) -> bool {
    fits(text, |byte| {
        byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'+' | b'_' | b'-')
    })
}

/// A Rust build target: 1 to 64 bytes of lower-case ASCII letters, digits and `_`, `.`, `-`.
#[must_use]
pub fn is_build_target(text: &str) -> bool {
    fits(text, lower_name_byte)
}

/// A tool name, known or not: the alphabet of a build target.
#[must_use]
pub fn is_tool_name(text: &str) -> bool {
    fits(text, lower_name_byte)
}

const fn lower_name_byte(byte: u8) -> bool {
    byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'.' | b'-')
}

fn fits(text: &str, allowed: impl Fn(u8) -> bool) -> bool {
    (1..=MAX_TEXT).contains(&text.len()) && text.bytes().all(allowed)
}

/// The version at the start of `word`: it starts with a digit and runs while the bytes are
/// version bytes, so `27.3.1,` gives `27.3.1`. `None` when it does not start with a digit or
/// runs past 64 bytes.
pub(super) fn leading_version(word: &str) -> Option<String> {
    if !word.starts_with(|c: char| c.is_ascii_digit()) {
        return None;
    }
    let end = word
        .find(|c: char| !(c.is_ascii_alphanumeric() || matches!(c, '.' | '+' | '_' | '-')))
        .unwrap_or(word.len());
    let version = &word[..end];
    is_version(version).then(|| version.to_owned())
}

/// The UTF-8 lines of `output`, trimmed; a line that is not UTF-8 is skipped.
pub(super) fn text_lines(output: &[u8]) -> impl Iterator<Item = &str> {
    output
        .split(|byte| *byte == b'\n')
        .filter_map(|line| std::str::from_utf8(line).ok())
        .map(str::trim)
}

/// The first line of `output` when the whole line is one version.
fn whole_version(output: &[u8]) -> Option<String> {
    let line = text_lines(output).next()?;
    is_version(line).then(|| line.to_owned())
}

/// The macOS version from `sw_vers -productVersion`, for example `27.0.1`.
#[must_use]
pub fn macos_version(output: &[u8]) -> Option<String> {
    whole_version(output)
}

/// The Linux distribution's version: `VERSION_ID` of `/etc/os-release`, quoted or not, for
/// example `24.04`. `None` for a rolling distribution, which has none.
#[must_use]
pub fn os_release_version(text: &[u8]) -> Option<String> {
    let value = text_lines(text).find_map(|line| line.strip_prefix("VERSION_ID="))?;
    let unquoted = ['"', '\'']
        .into_iter()
        .find_map(|quote| value.strip_prefix(quote)?.strip_suffix(quote))
        .unwrap_or(value);
    is_version(unquoted).then(|| unquoted.to_owned())
}

/// The kernel release from `/proc/sys/kernel/osrelease`, for example `6.8.0-48-generic`: the
/// Linux version when the distribution gives none.
#[must_use]
pub fn kernel_release(text: &[u8]) -> Option<String> {
    whole_version(text)
}
