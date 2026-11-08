//! API tokens: 32 random bytes shown once as base64url; only their SHA-256 is kept (D-7).
//!
//! Pure: the random bytes and the time are parameters.

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, TimeDelta, Utc};
use sha2::{Digest as _, Sha256};
use zeroize::Zeroizing;

/// Random bytes in a token.
pub(super) const TOKEN_BYTES: usize = 32;

/// Length of a token's text: 32 bytes in unpadded base64url.
const TOKEN_CHARS: usize = 43;

/// A login token is valid this long (D-7).
pub(super) const LOGIN_LIFETIME: TimeDelta = TimeDelta::days(30);

/// A builder token is valid this long: the VPN client key period (BLD-022, VPN-003).
pub(super) const BUILDER_LIFETIME: TimeDelta = TimeDelta::days(180);

/// `last_used_at` is written at most this often (D-7).
const TOUCH_INTERVAL: TimeDelta = TimeDelta::minutes(1);

/// What a token may do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    Read,
    Write,
    /// Only the builder operations: claim and report build jobs (BLD-022, D-1).
    Builder,
}

impl Scope {
    /// The text stored in `api_tokens.scope`.
    pub(super) const fn as_str(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Write => "write",
            Self::Builder => "builder",
        }
    }

    /// The scope stored as `text`; `None` for anything else.
    pub(super) fn from_stored(text: &str) -> Option<Self> {
        match text {
            "read" => Some(Self::Read),
            "write" => Some(Self::Write),
            "builder" => Some(Self::Builder),
            _ => None,
        }
    }
}

/// A token as the operator holds it. Wiped on drop. Deliberately implements no `Debug`,
/// `Display`, `Clone` or `Serialize`, so it cannot reach a log or an error (LOG-007).
pub struct Token(Zeroizing<String>);

impl Token {
    /// Encodes the random bytes, then wipes them.
    pub(super) fn from_random(mut bytes: [u8; TOKEN_BYTES]) -> Self {
        let text = Zeroizing::new(URL_SAFE_NO_PAD.encode(bytes));
        zeroize::Zeroize::zeroize(&mut bytes);
        Self(text)
    }

    /// The text to give the operator once, in the login response.
    pub(super) fn expose(&self) -> &str {
        &self.0
    }

    /// The hash stored for this token.
    pub(super) fn hash(&self) -> TokenHash {
        hash_of(self.expose()).expect("a token encodes 32 bytes")
    }
}

/// The SHA-256 of a token's bytes: what `api_tokens.token_hash` holds and lookups use.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct TokenHash(pub(super) [u8; 32]);

/// The hash of a presented token, or `None` if the text cannot be a token. Only canonical
/// unpadded base64url of exactly 32 bytes is accepted, so each token has one spelling and
/// no lookup runs for junk.
pub(super) fn hash_of(presented: &str) -> Option<TokenHash> {
    if presented.len() != TOKEN_CHARS {
        return None;
    }
    let bytes = Zeroizing::new(URL_SAFE_NO_PAD.decode(presented).ok()?);
    if bytes.len() != TOKEN_BYTES {
        return None;
    }
    Some(TokenHash(Sha256::digest(&*bytes).into()))
}

/// A token is usable until, not at, its expiry.
pub(super) fn usable(expires_at: DateTime<Utc>, now: DateTime<Utc>) -> bool {
    now < expires_at
}

/// Whether `last_used_at` is old enough to be written again (at most once a minute).
pub(super) fn touch_due(last_used_at: Option<DateTime<Utc>>, now: DateTime<Utc>) -> bool {
    last_used_at.is_none_or(|last| now - last >= TOUCH_INTERVAL)
}
