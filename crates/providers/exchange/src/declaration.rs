//! What a source declares about itself (SRC-001, SRC-002, SRC-010, SRC-021).

use chrono::{DateTime, Utc};

use crate::exchange::Problem;

/// The shortest poll interval a declaration may ask for (SRC-021).
pub const MIN_POLL_INTERVAL_SECONDS: u32 = 900;

/// A source as the operator sees it before connecting.
#[derive(Clone, Copy, Debug)]
pub struct Declaration {
    /// Stable identifier, for example `github`.
    pub key: &'static str,
    pub name: &'static str,
    pub acquisition: Acquisition,
    /// What the connection can do, shown as text.
    pub scopes: &'static [&'static str],
    /// `None`: read-only scopes. `Some`: why the source asks for more (SRC-010).
    pub write_reason: Option<&'static str>,
    /// What the source gives, for example "Repositories".
    pub gives: &'static [&'static str],
    pub poll_interval_seconds: u32,
    pub api_base: &'static str,
}

impl Declaration {
    /// Whether the poll interval is allowed (SRC-021).
    #[must_use]
    pub const fn poll_interval_is_allowed(&self) -> bool {
        self.poll_interval_seconds >= MIN_POLL_INTERVAL_SECONDS
    }
}

/// How the hub gets a token for the source. More variants arrive by contract change.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub enum Acquisition {
    TokenPull(Minting),
}

/// How a pulled token is made.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub enum Minting {
    /// A token minted with a key the hub holds for the operator's App.
    AppKey(AppKey),
}

/// The provider-specific parts of minting a token from an App key.
#[derive(Clone, Copy, Debug)]
pub struct AppKey {
    /// The path to `POST` for an installation, below `api_base`.
    pub mint_path: fn(installation: &str) -> String,
    /// Reads the mint reply.
    pub parse_minted: fn(body: &[u8]) -> Result<Minted, Problem>,
}

/// A token and the time it stops working. No `Debug`: the token is a secret.
pub struct Minted {
    pub token: String,
    pub expires_at: DateTime<Utc>,
}
