//! What a share is to the operator (K1 `ShareAccess`, `SessionShare`, `ShareLink`) and why it
//! ends. Pure.

#![expect(
    clippy::unused_async_trait_impl,
    reason = "the async-graphql Enum derive generates async trait impls"
)]

use std::fmt;

use async_graphql::Enum;
use chrono::{DateTime, Utc};
use zeroize::Zeroizing;

use crate::{
    events::ClientAddr,
    ids::{SessionId, ShareId},
    sessions::tickets::TicketToken,
};

/// What a viewer may do: watch, or also type and point (D-9). `CONTROL` is accepted as given;
/// the confirm is the client's.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
pub enum ShareAccess {
    View,
    Control,
}

impl ShareAccess {
    /// The text the audit entry `share.create` holds.
    pub(super) const fn as_str(self) -> &'static str {
        match self {
            Self::View => "view",
            Self::Control => "control",
        }
    }
}

/// A live share as the operator sees it (K1 `SessionShare`). It holds no token.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionShare {
    pub(super) id: ShareId,
    pub(super) session: SessionId,
    pub(super) access: ShareAccess,
    pub(super) max_viewers: u8,
    /// Viewers connected now.
    pub(super) viewers: u8,
    pub(super) created_at: DateTime<Utc>,
    pub(super) expires_at: DateTime<Utc>,
}

/// What `shareSession` gives the owner once: the share and its link, which holds the token
/// after `#` only (D-7). The link is wiped on drop and never printed.
pub(super) struct ShareLink {
    pub(super) share: SessionShare,
    url: Zeroizing<String>,
}

impl ShareLink {
    /// `<origin>/shared#<token>`: browsers never send the fragment, so the token reaches no
    /// server log on the way to the viewer.
    pub(super) fn new(share: SessionShare, origin: &str, token: &TicketToken) -> Self {
        let url = Zeroizing::new(format!("{origin}/shared#{}", token.expose()));
        Self { share, url }
    }

    /// The link, for the `shareSession` response only.
    pub(super) fn url(&self) -> &str {
        &self.url
    }
}

impl fmt::Debug for ShareLink {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ShareLink")
            .field("share", &self.share)
            .field("url", &"[redacted]")
            .finish()
    }
}

/// Why a share ended (S-5): the first cause wins.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ShareEnd {
    /// Its `expiresAt` passed.
    Expired,
    /// Its owner called `endShare` from this address.
    Ended(ClientAddr),
    /// The owner's session ended: its tunnel was told to end, or it deregistered (K4 §5).
    SessionEnded,
}

impl ShareEnd {
    /// The `cause` the audit entry `share.end` holds.
    pub(super) const fn as_str(self) -> &'static str {
        match self {
            Self::Expired => "expired",
            Self::Ended(_) => "ended",
            Self::SessionEnded => "session_ended",
        }
    }

    /// The event's sentence: fixed text, no value from the share.
    pub(super) const fn sentence(self) -> &'static str {
        match self {
            Self::Expired => "A session share expired.",
            Self::Ended(_) => "An operator ended a session share.",
            Self::SessionEnded => "A session share ended with its session.",
        }
    }
}
