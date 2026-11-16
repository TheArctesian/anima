//! Where a ref and a file are: whether a ref is a commit, and a file's page on GitHub
//! (PRJ-010). Pure.

const HEX: &[u8; 16] = b"0123456789ABCDEF";

/// Whether `git_ref` names one commit: 40 hex digits. A branch can move; a commit cannot.
#[must_use]
pub fn is_commit(git_ref: &str) -> bool {
    git_ref.len() == 40 && git_ref.bytes().all(|b| b.is_ascii_hexdigit())
}

/// `text` with each byte outside `A-Za-z0-9-._~` and `/` percent-encoded, so a name with a
/// space, `#`, `?` or `%` stays one part of the address.
fn encoded(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~' | b'/') {
            out.push(char::from(byte));
        } else {
            out.push('%');
            out.push(char::from(HEX[usize::from(byte >> 4)]));
            out.push(char::from(HEX[usize::from(byte & 0x0f)]));
        }
    }
    out
}

/// The page of the file at `path` on `git_ref` of the repository whose page is `home`.
#[must_use]
pub fn file(home: &str, git_ref: &str, path: &str) -> String {
    format!(
        "{}/blob/{}/{}",
        home.trim_end_matches('/'),
        encoded(git_ref),
        encoded(path)
    )
}
