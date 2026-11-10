//! API tokens: 32 random bytes shown once as base64url; only their SHA-256 is kept (D-7). Each
//! kind of token travels one way only (D-3, D-23, WP-43 A-3).
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

/// What an API key's secret starts with, so secret scanners can find a leaked one (D-23).
const KEY_PREFIX: &str = "anima_";

/// Length of an API key's secret: the prefix and a token's text.
const KEY_CHARS: usize = KEY_PREFIX.len() + TOKEN_CHARS;

/// Characters of an API key's secret the operator sees again: `anima_` and six more, enough to
/// tell keys apart, far too few to use one (D-23).
const KEY_DISPLAY_CHARS: usize = 12;

/// A login token is valid this long (D-7).
pub(super) const LOGIN_LIFETIME: TimeDelta = TimeDelta::days(30);

/// A browser token is valid this long, with no refresh (D-3).
pub(super) const BROWSER_LIFETIME: TimeDelta = TimeDelta::days(7);

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

/// What a token is for, which decides how it may travel (WP-43 A-3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenKind {
    /// The desktop app's sign-in: an `Authorization: Bearer` header or the `connection_init`
    /// payload.
    Login,
    /// A browser's sign-in: only the `__Host-anima` cookie.
    Browser,
    /// A named key for scripts: a bearer value spelled `anima_` plus the token text.
    ApiKey,
    /// An enrolled builder computer's token: a bearer value, the token text alone. It always has
    /// the `builder` scope and stands for no operator (BLD-022, D-1).
    Builder,
}

impl TokenKind {
    /// The text stored in `api_tokens.kind`.
    pub(super) const fn as_str(self) -> &'static str {
        match self {
            Self::Login => "login",
            Self::Browser => "browser",
            Self::ApiKey => "api_key",
            Self::Builder => "builder",
        }
    }

    /// The kind stored as `text`; `None` for anything else.
    pub(super) fn from_stored(text: &str) -> Option<Self> {
        match text {
            "login" => Some(Self::Login),
            "browser" => Some(Self::Browser),
            "api_key" => Some(Self::ApiKey),
            "builder" => Some(Self::Builder),
            _ => None,
        }
    }
}

/// How a token reached the hub.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Transport {
    /// `Authorization: Bearer` or the `connection_init` payload.
    Bearer,
    /// The `__Host-anima` cookie of a request from the public origin.
    Cookie,
}

/// How a presented token was spelled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Spelling {
    /// The token text alone.
    Plain,
    /// `anima_` and the token text: an API key.
    Key,
}

/// Whether a token of `kind`, spelled so, may be used from `transport`: a browser token only
/// from the cookie, a login token, a builder token and an API key only as a bearer, and only an
/// API key with the `anima_` prefix. Anything else is no identity, so a browser token taken from a cookie jar is
/// no bearer and a cookie cannot carry a key (A-3).
pub(super) fn admitted(kind: TokenKind, spelling: Spelling, transport: Transport) -> bool {
    match (kind, transport) {
        (TokenKind::Browser, Transport::Cookie)
        | (TokenKind::Login | TokenKind::Builder, Transport::Bearer) => spelling == Spelling::Plain,
        (TokenKind::ApiKey, Transport::Bearer) => spelling == Spelling::Key,
        (TokenKind::Browser, Transport::Bearer)
        | (TokenKind::Login | TokenKind::ApiKey | TokenKind::Builder, Transport::Cookie) => false,
    }
}

/// Whether a stored kind and scope belong together: the `builder` scope on a builder token and
/// on nothing else (BLD-022, D-1). Migration 0030 checks the same; a row that breaks it is no
/// identity.
pub(super) fn consistent(kind: TokenKind, scope: Scope) -> bool {
    (kind == TokenKind::Builder) == (scope == Scope::Builder)
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

    /// The token spelled as an API key: `anima_` and its text, so secret scanners can find a
    /// leaked one (D-23). Shown once, in `createApiKey`'s answer. Wiped on drop.
    pub(super) fn key_secret(&self) -> Zeroizing<String> {
        let mut secret = Zeroizing::new(String::with_capacity(KEY_CHARS));
        secret.push_str(KEY_PREFIX);
        secret.push_str(self.expose());
        secret
    }

    /// The first 12 characters of the key's secret, which `apiKeys` shows (K1 `ApiKey.prefix`).
    pub(super) fn key_display_prefix(&self) -> String {
        self.key_secret()[..KEY_DISPLAY_CHARS].to_owned()
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

/// The hash and the spelling of a presented token, or `None` if the text can be neither a
/// token nor an API key. A key is exactly `anima_` and 43 characters, so a plain token that
/// happens to start with `anima_` is still read as plain; both hash the 32 decoded bytes. An
/// agent token or an enrolment code is neither (R-092 D-1).
pub(super) fn read_presented(presented: &str) -> Option<(TokenHash, Spelling)> {
    if is_agent_secret(presented) {
        return None;
    }
    match presented.strip_prefix(KEY_PREFIX) {
        Some(body) if presented.len() == KEY_CHARS => Some((hash_of(body)?, Spelling::Key)),
        _ => Some((hash_of(presented)?, Spelling::Plain)),
    }
}

/// An agent token: this prefix and 43 base64url characters (32 bytes, K8, MET-013).
const AGENT_TOKEN_PREFIX: &str = "anima_agent_";
const AGENT_TOKEN_CHARS: usize = 43;

/// An enrolment code: this prefix and 22 base64url characters (16 bytes, K8, MET-012).
const ENROLMENT_CODE_PREFIX: &str = "anima_enrol_";
const ENROLMENT_CODE_CHARS: usize = 22;

/// Whether `presented` is spelled exactly as one of the metrics agent's secrets: an agent token
/// or an enrolment code, its prefix and then exactly its K8 number of base64url characters.
/// These live in `metrics`, never in `access.api_tokens`, so the guard refuses them by spelling
/// on every transport before any lookup (R-092 D-1). An API key is `anima_` and 43 characters,
/// so one whose text begins `agent_` or `enrol_` is still a key.
///
/// The length rule of [`read_presented`] also refuses both shapes today (55 and 34 characters
/// are neither a token nor a key); this rule keeps holding if token lengths change.
pub(super) fn is_agent_secret(presented: &str) -> bool {
    spelled(presented, AGENT_TOKEN_PREFIX, AGENT_TOKEN_CHARS)
        || spelled(presented, ENROLMENT_CODE_PREFIX, ENROLMENT_CODE_CHARS)
}

/// Whether `presented` is `prefix` and exactly `chars` base64url characters.
fn spelled(presented: &str, prefix: &str, chars: usize) -> bool {
    presented.strip_prefix(prefix).is_some_and(|body| {
        body.len() == chars
            && body
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
    })
}

/// A token is usable until, not at, its expiry.
pub(super) fn usable(expires_at: DateTime<Utc>, now: DateTime<Utc>) -> bool {
    now < expires_at
}

/// Whether `last_used_at` is old enough to be written again (at most once a minute).
pub(super) fn touch_due(last_used_at: Option<DateTime<Utc>>, now: DateTime<Utc>) -> bool {
    last_used_at.is_none_or(|last| now - last >= TOUCH_INTERVAL)
}
