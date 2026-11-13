//! Putting a repository name and a branch into a request path.

const HEX: &[u8; 16] = b"0123456789ABCDEF";

/// A repository name that cannot go into a path.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
#[error("a repository name is owner/name, with letters, digits, dots, dashes and underscores")]
pub(crate) struct UnusableName;

/// Checks that `full_name` is `owner/name` and safe to put in a path.
pub(crate) fn check(full_name: &str) -> Result<(), UnusableName> {
    let plain = |part: &str| {
        !part.is_empty()
            && part != "."
            && part != ".."
            && part
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
    };
    match full_name.split_once('/') {
        Some((owner, name)) if plain(owner) && plain(name) => Ok(()),
        _ => Err(UnusableName),
    }
}

/// A branch name as one or more path segments: slashes stay, anything else unsafe is
/// percent-encoded (a branch may hold `@`, `#`, `?` or `%`).
pub(crate) fn encode_branch(branch: &str) -> String {
    let mut out = String::with_capacity(branch.len());
    for byte in branch.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~' | b'/') {
            out.push(char::from(byte));
        } else {
            out.push('%');
            out.push(char::from(HEX[usize::from(byte >> 4)]));
            out.push(char::from(HEX[usize::from(byte & 0x0F)]));
        }
    }
    out
}
