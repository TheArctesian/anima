//! What the hub tells guacd about one connection (GW-012, GW-013).
//!
//! The client supplies only [`Display`] and the time zone (the K1
//! `DisplayInput` fields). Every `connect` value comes from [`ArgValues`],
//! which the hub fills from the database and the credential store.

use std::collections::HashMap;

/// Screen size and density of the operator's view, as the client reports it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Display {
    pub width: u32,
    pub height: u32,
    pub dpi: u32,
}

impl Display {
    /// Smallest accepted width or height, in pixels (K4 Section 4).
    pub const MIN_SIDE_PX: u32 = 1;
    /// Largest accepted width or height, in pixels (K4 Section 4).
    pub const MAX_SIDE_PX: u32 = 8192;
    /// Smallest accepted DPI (K4 Section 4).
    pub const MIN_DPI: u32 = 48;
    /// Largest accepted DPI (K4 Section 4).
    pub const MAX_DPI: u32 = 480;

    /// True when width and height are 1..=8192 and DPI is 48..=480.
    #[must_use]
    pub fn within_bounds(&self) -> bool {
        let sides = Self::MIN_SIDE_PX..=Self::MAX_SIDE_PX;
        sides.contains(&self.width)
            && sides.contains(&self.height)
            && (Self::MIN_DPI..=Self::MAX_DPI).contains(&self.dpi)
    }
}

/// Longest accepted time zone, in characters.
const MAX_TIMEZONE_CHARS: usize = 64;

/// True when `text` is an IANA-shaped zone name such as `UTC`,
/// `America/Argentina/Buenos_Aires` or `Etc/GMT+5`. The full rule:
///
/// - 1..=64 characters, each one of `A-Z`, `a-z`, `0-9`, `_`, `+`, `-`, `/`;
/// - the first character is an ASCII letter;
/// - no `/` segment is empty (no leading, trailing or doubled `/`).
///
/// The time zone comes from the client and reaches the SSH server as `TZ`, so
/// anything else is refused (GW-012): control characters, spaces, `:`-paths,
/// `.` and `..`, and a leading `/`, which glibc reads as an absolute file path.
/// The hub checks `DisplayInput.timezone` with this and falls back to the
/// site's zone before it builds [`ConnectParams`]; the handshake refuses a
/// time zone that fails it.
#[must_use]
pub fn is_zone_name(text: &str) -> bool {
    // The first-letter test also refuses the empty string.
    text.len() <= MAX_TIMEZONE_CHARS
        && text.bytes().next().is_some_and(|b| b.is_ascii_alphabetic())
        && text
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'+' | b'-' | b'/'))
        && text.split('/').all(|segment| !segment.is_empty())
}

/// Hub-trusted `connect` values by argument name.
///
/// Holds passwords and keys, so it implements neither `Debug` nor `Display`
/// (LOG-007). A name set twice keeps the last value.
#[derive(Default)]
pub struct ArgValues {
    values: HashMap<String, String>,
}

impl ArgValues {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the value guacd gets for the argument `name`, replacing any earlier one.
    pub fn set(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.values.insert(name.into(), value.into());
    }

    /// The value for `name`, if the hub set one.
    pub(crate) fn get(&self, name: &str) -> Option<&str> {
        self.values.get(name).map(String::as_str)
    }
}

impl FromIterator<(String, String)> for ArgValues {
    fn from_iter<T: IntoIterator<Item = (String, String)>>(iter: T) -> Self {
        Self {
            values: iter.into_iter().collect(),
        }
    }
}

/// Everything the handshake sends after `select`.
///
/// `display` and `timezone` come from the client; `name` (the operator's
/// username), `audio` and `values` come from the hub. No `Debug`: `values`
/// holds secrets.
pub struct ConnectParams {
    pub display: Display,
    pub timezone: String,
    pub name: String,
    /// The audio mimetypes the client plays, sent as `audio`: hub constants,
    /// empty when the session has no sound.
    pub audio: &'static [&'static str],
    pub values: ArgValues,
}
