//! What a request path asks the web client for, read from its percent-decoded form so an
//! encoded spelling gets the same answer as the plain one (K7).

/// The prefix of `SvelteKit`'s build output: a missing file there is 404, never `index.html`,
/// so HTML is never served as a script (A-2).
const APP: &[u8] = b"/_app/";
/// Hashed build output that never changes under its name.
const IMMUTABLE: &[u8] = b"/_app/immutable/";

/// What a request path asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wanted {
    /// A segment starts with `.` (a dotfile, `.` or `..`) or holds a NUL: always 404.
    Hidden,
    /// `/` or `/index.html`: the `index.html` read when the web client was built.
    Index,
    /// A file under `/_app/`, or 404.
    Asset { immutable: bool },
    /// A file, or `index.html` for a client-side route.
    Page,
}

/// What `path` (the request URI's path, still percent-encoded) asks for. Segments are split
/// at `/` and at `\`, so a backslash spelling of `..` is hidden too. A run of `/`, plain or
/// encoded, counts as one, as it does when `ServeDir` finds the file: `//_app/x.js` is an
/// asset and `//index.html` the index.
#[must_use]
pub fn wanted(path: &str) -> Wanted {
    let mut decoded = percent_decoded(path.as_bytes());
    decoded.dedup_by(|later, earlier| *later == b'/' && *earlier == b'/');
    let hidden = decoded
        .split(|byte| matches!(byte, b'/' | b'\\'))
        .any(|segment| segment.first() == Some(&b'.') || segment.contains(&0));
    if hidden {
        Wanted::Hidden
    } else if decoded == b"/" || decoded == b"/index.html" {
        Wanted::Index
    } else if decoded.starts_with(APP) {
        Wanted::Asset {
            immutable: decoded.starts_with(IMMUTABLE),
        }
    } else {
        Wanted::Page
    }
}

/// `bytes` with each `%` and two hex digits replaced by the byte they name; any other `%` is
/// kept as it is. One pass only, as a file server decodes.
fn percent_decoded(bytes: &[u8]) -> Vec<u8> {
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut rest = bytes;
    while let Some((&first, tail)) = rest.split_first() {
        let escaped = match tail {
            [high, low, ..] if first == b'%' => hex(*high).zip(hex(*low)),
            _ => None,
        };
        if let Some((high, low)) = escaped {
            decoded.push(high << 4 | low);
            rest = &tail[2..];
        } else {
            decoded.push(first);
            rest = tail;
        }
    }
    decoded
}

/// The value of one hex digit.
const fn hex(digit: u8) -> Option<u8> {
    match digit {
        b'0'..=b'9' => Some(digit - b'0'),
        b'a'..=b'f' => Some(digit - b'a' + 10),
        b'A'..=b'F' => Some(digit - b'A' + 10),
        _ => None,
    }
}
