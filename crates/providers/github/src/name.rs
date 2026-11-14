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

/// A branch name that cannot go into a path.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
#[error("a branch name must not have a segment that is a dot or two dots")]
pub(crate) struct UnusableBranch;

/// Checks that `branch` has no `.` or `..` segment, which a URL parser would resolve and so
/// move the request off the repository it names.
pub(crate) fn check_branch(branch: &str) -> Result<(), UnusableBranch> {
    if branch
        .split('/')
        .any(|segment| segment == "." || segment == "..")
    {
        Err(UnusableBranch)
    } else {
        Ok(())
    }
}

/// A branch name as one or more path segments: slashes stay, anything else unsafe is
/// percent-encoded (a branch may hold `@`, `#`, `?` or `%`). Only a branch that passed
/// [`check_branch`] goes in.
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
