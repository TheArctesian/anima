//! The account that a program uses, as it may leave the machine (AIS-022, R-313):
//! a key made from the vendor's identifier with the salt of the hub, the name of the plan, and
//! the limit windows. Never the identifier, the name of the account, an address or a token.
//! Pure.

use std::fmt::Write as _;

use chrono::{DateTime, Utc};
use hmac::{Hmac, KeyInit as _, Mac as _};
use sha2::Sha256;

use super::Source;

/// The most accounts one batch may hold: one for each source, with room for newer sources.
pub const MAX_ACCOUNTS: usize = 4;
/// The most limit windows of one account.
pub const MAX_LIMITS: usize = 8;
/// The bytes of the account salt of a hub.
pub const SALT_BYTES: usize = 32;
/// The hexadecimal digits of a key.
pub const KEY_DIGITS: usize = 64;
const MAX_PLAN: usize = 32;
const MAX_WINDOW: usize = 32;

/// One account of one source on the machine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Account {
    pub source: Source,
    /// [`account_key`] of the vendor's identifier.
    pub key: String,
    pub plan: Option<String>,
    pub limits: Vec<Limit>,
}

/// One limit window of an account: how much of it is used, and when it starts again.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Limit {
    /// The name of the window as the program gives it (`five_hour`), or its length (`300 min`).
    pub window: String,
    pub used_percent: Option<u8>,
    pub resets_at: Option<DateTime<Utc>>,
    pub reached: bool,
}

/// The key of the account `identifier` of `source` with the account salt of a hub: the
/// HMAC-SHA-256 of `<source>:<identifier>`, in lower-case hexadecimal (K8 Section 4). The same
/// account on two machines of one hub gives one key; a different salt gives a different key, and
/// the key does not give the identifier back.
///
/// # Panics
/// Never: HMAC takes a key of any length.
#[must_use]
pub fn account_key(salt: &[u8; SALT_BYTES], source: Source, identifier: &str) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(salt).expect("HMAC takes a key of any length");
    mac.update(source.as_str().as_bytes());
    mac.update(b":");
    mac.update(identifier.as_bytes());
    mac.finalize()
        .into_bytes()
        .iter()
        .fold(String::with_capacity(KEY_DIGITS), |mut key, byte| {
            let _ = write!(key, "{byte:02x}");
            key
        })
}

/// Whether `text` is a key: [`KEY_DIGITS`] lower-case hexadecimal digits.
#[must_use]
pub fn is_key(text: &str) -> bool {
    text.len() == KEY_DIGITS
        && text
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

/// Whether `text` is the name of a plan: 1 to 32 bytes of `[A-Za-z0-9 ._-]`.
#[must_use]
pub fn is_plan(text: &str) -> bool {
    (1..=MAX_PLAN).contains(&text.len())
        && text
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b' ' | b'.' | b'_' | b'-'))
}

/// Whether `text` is the name of a limit window: 1 to 32 bytes of `[a-z0-9_ ]`.
#[must_use]
pub fn is_window(text: &str) -> bool {
    (1..=MAX_WINDOW).contains(&text.len())
        && text.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b' ')
        })
}
