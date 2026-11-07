//! Tunnel tickets (K4 §1, API-009): 32 random bytes shown once as base64url; only their
//! SHA-256 is kept, in memory (LOG-007).
//!
//! Pure: the random bytes and the time are parameters (CQ-021).

use std::fmt;

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, TimeDelta, Utc};
use sha2::{Digest as _, Sha256};
use zeroize::Zeroizing;

use super::model::Protocol;
use crate::{
    events::{ClientAddr, Operator},
    ids::{DeviceId, SessionId},
};

/// Random bytes in a ticket token.
pub(super) const TOKEN_BYTES: usize = 32;

/// Length of a token's text: 32 bytes in unpadded base64url.
const TOKEN_CHARS: usize = 43;

/// A ticket is redeemable this long after it is issued (K4 §1).
pub(super) const LIFETIME: TimeDelta = TimeDelta::seconds(30);

/// The files tunnel's path (K4 Section 1 `tunnelPath`, Section 9).
pub(super) const FILES_PATH: &str = "/files";

/// A token as the client holds it. Wiped on drop. Implements no `Debug`, `Display`, `Clone`
/// or `Serialize`, so it cannot reach a log or an error (LOG-007).
pub(super) struct TicketToken(Zeroizing<String>);

impl TicketToken {
    /// Encodes the random bytes, then wipes them.
    pub(super) fn from_random(mut bytes: [u8; TOKEN_BYTES]) -> Self {
        let text = Zeroizing::new(URL_SAFE_NO_PAD.encode(bytes));
        zeroize::Zeroize::zeroize(&mut bytes);
        Self(text)
    }

    pub(super) fn expose(&self) -> &str {
        &self.0
    }

    /// The hash the ticket table is keyed by.
    pub(super) fn hash(&self) -> TicketHash {
        hash_of(self.expose()).expect("a ticket token encodes 32 bytes")
    }
}

/// The SHA-256 of a token's bytes: the only form of a token the hub keeps.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct TicketHash([u8; 32]);

/// The hash of a presented token, or `None` if the text cannot be a token. Only canonical
/// unpadded base64url of exactly 32 bytes is accepted, so each token has one spelling.
pub(super) fn hash_of(presented: &str) -> Option<TicketHash> {
    if presented.len() != TOKEN_CHARS {
        return None;
    }
    let bytes = Zeroizing::new(URL_SAFE_NO_PAD.decode(presented).ok()?);
    if bytes.len() != TOKEN_BYTES {
        return None;
    }
    Some(TicketHash(Sha256::digest(&*bytes).into()))
}

/// What `startSession` or `openFiles` gives the client once: the session, its protocol, the
/// token and the tunnel's path the token opens.
pub struct Ticket {
    pub session: SessionId,
    pub protocol: Protocol,
    token: TicketToken,
    pub expires_at: DateTime<Utc>,
    /// Where the client opens the tunnel: the protocol's, or [`FILES_PATH`] for a files
    /// ticket (K4 Section 1 `tunnelPath`).
    pub tunnel_path: &'static str,
}

impl Ticket {
    pub(super) const fn new(
        session: SessionId,
        protocol: Protocol,
        token: TicketToken,
        expires_at: DateTime<Utc>,
    ) -> Self {
        Self {
            session,
            protocol,
            token,
            expires_at,
            tunnel_path: protocol.tunnel_path(),
        }
    }

    /// A ticket for the files tunnel of the live SSH session `session` (K4 Section 9).
    pub(super) const fn files(
        session: SessionId,
        token: TicketToken,
        expires_at: DateTime<Utc>,
    ) -> Self {
        Self {
            session,
            protocol: Protocol::Ssh,
            token,
            expires_at,
            tunnel_path: FILES_PATH,
        }
    }

    /// The token text, for the `startSession` response only.
    #[must_use]
    pub fn token(&self) -> &str {
        self.token.expose()
    }
}

impl fmt::Debug for Ticket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Ticket")
            .field("session", &self.session)
            .field("protocol", &self.protocol)
            .field("token", &"[redacted]")
            .field("expires_at", &self.expires_at)
            .field("tunnel_path", &self.tunnel_path)
            .finish()
    }
}

/// An issued, unredeemed ticket: what the tunnel needs to open the session it was issued for.
/// The display values are as the client sent them, already within bounds (GW-012).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingTicket {
    pub operator: Operator,
    pub session: SessionId,
    pub client: ClientAddr,
    pub device: DeviceId,
    pub protocol: Protocol,
    pub display: guac_protocol::Display,
    /// As the client sent it; the tunnel checks its shape before guacd sees it.
    pub timezone: Option<String>,
    pub expires_at: DateTime<Utc>,
}

impl PendingTicket {
    /// Expired at, not only after, `expires_at`.
    pub(super) fn is_expired(&self, now: DateTime<Utc>) -> bool {
        now >= self.expires_at
    }

    /// Whether the ticket opens a tunnel for `presented` at `now`: not expired, from the
    /// client it was issued to, on its own tunnel path (K4 Section 1 `tunnelPath`).
    pub(super) fn admits(&self, presented: &Presented<'_>, now: DateTime<Utc>) -> bool {
        !self.is_expired(now)
            && self.client == presented.client
            && (presented.tunnel_path)(self.protocol) == presented.path
    }
}

/// A token as a tunnel request presents it: from which client, on which path, and where
/// that route says each protocol's tickets go. Holds the token borrowed, never copied.
#[derive(Clone, Copy)]
pub struct Presented<'a> {
    pub token: &'a str,
    pub client: ClientAddr,
    /// The route's own path.
    pub path: &'a str,
    /// Where a ticket for each protocol is opened: `Protocol::tunnel_path` in the hub.
    pub tunnel_path: fn(Protocol) -> &'static str,
}

impl<'a> Presented<'a> {
    /// `token` from `client` on `/terminal`, as the app opens an SSH session (K4 Section 8.1).
    #[must_use]
    pub fn terminal(token: &'a str, client: ClientAddr) -> Self {
        Self {
            token,
            client,
            path: Protocol::Ssh.tunnel_path(),
            tunnel_path: Protocol::tunnel_path,
        }
    }
}
