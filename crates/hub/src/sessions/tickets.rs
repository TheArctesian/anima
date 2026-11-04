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

/// What `startSession` gives the client once: the session and the token for `/tunnel`.
pub struct Ticket {
    pub session: SessionId,
    token: TicketToken,
    pub expires_at: DateTime<Utc>,
}

impl Ticket {
    pub(super) const fn new(
        session: SessionId,
        token: TicketToken,
        expires_at: DateTime<Utc>,
    ) -> Self {
        Self {
            session,
            token,
            expires_at,
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
            .field("token", &"[redacted]")
            .field("expires_at", &self.expires_at)
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

    /// Whether the ticket opens a tunnel for `client` at `now`.
    pub(super) fn admits(&self, client: ClientAddr, now: DateTime<Utc>) -> bool {
        !self.is_expired(now) && self.client == client
    }
}
