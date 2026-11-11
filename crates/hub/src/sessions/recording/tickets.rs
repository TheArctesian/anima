//! Playback tickets (K4 §1 and §12, G-7): the same tokens as session tickets, held in their
//! own table so a playback ticket opens only `/recording` and no other ticket opens it. Only
//! the SHA-256 of a token is held, in memory, never in the database or a log (LOG-007).
//!
//! Every method takes the lock for a short, synchronous critical section and never across an
//! `.await`; time comes in as a parameter (CQ-021).

use std::{
    collections::HashMap,
    sync::{Arc, Mutex, MutexGuard, PoisonError},
};

use chrono::{DateTime, Utc};

use crate::{
    events::{ClientAddr, Operator},
    ids::{RecordingId, SessionId},
    sessions::{
        Refused,
        tickets::{TicketHash, hash_of},
    },
};

/// Most playback tickets held at once. A ticket lives 30 s and holds no row, so expired ones
/// are dropped as new ones are issued; this bounds what a client asking in a loop can hold.
pub(super) const MAX_PENDING: usize = 32;

/// An issued playback ticket: who may play which recording, from where, and until when.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::sessions) struct PlaybackTicket {
    pub(in crate::sessions) operator: Operator,
    pub(in crate::sessions) client: ClientAddr,
    pub(in crate::sessions) recording: RecordingId,
    /// The recording's session, as the ticket names it (K4 §12).
    pub(in crate::sessions) session: SessionId,
    pub(in crate::sessions) expires_at: DateTime<Utc>,
}

/// Too many playback tickets are pending.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
#[error("no playback ticket can be issued now")]
pub(super) struct Full;

/// The pending playback tickets, by token hash. Cheap to clone; clones share the tickets.
#[derive(Clone, Default)]
pub(in crate::sessions) struct Tickets(Arc<Mutex<HashMap<TicketHash, PlaybackTicket>>>);

impl Tickets {
    /// The poisoned case is taken over: no critical section here panics between two writes.
    fn held(&self) -> MutexGuard<'_, HashMap<TicketHash, PlaybackTicket>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Holds `ticket` under `hash`, after dropping every ticket expired at `now`.
    ///
    /// # Errors
    /// [`Full`] when [`MAX_PENDING`] unexpired tickets are held.
    pub(super) fn issue(
        &self,
        hash: TicketHash,
        ticket: PlaybackTicket,
        now: DateTime<Utc>,
    ) -> Result<(), Full> {
        let mut held = self.held();
        held.retain(|_, pending| now < pending.expires_at);
        if held.len() >= MAX_PENDING {
            return Err(Full);
        }
        held.insert(hash, ticket);
        Ok(())
    }

    /// Drops the ticket under `hash`, if it is held: its issue was not recorded.
    pub(super) fn withdraw(&self, hash: TicketHash) {
        self.held().remove(&hash);
    }

    /// Redeems a presented token (K4 §1). The token is removed first, valid or not, so it
    /// works at most once; then it is refused when expired at `now` (the boundary
    /// `now == expires_at` included) or presented from another address than `client`.
    ///
    /// # Errors
    /// [`Refused`], with no detail, in every refused case.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "the /recording route (WP-48 T6) is the first caller"
        )
    )]
    pub(in crate::sessions) fn redeem(
        &self,
        presented: &str,
        client: ClientAddr,
        now: DateTime<Utc>,
    ) -> Result<PlaybackTicket, Refused> {
        let hash = hash_of(presented).ok_or(Refused)?;
        let ticket = self.held().remove(&hash).ok_or(Refused)?;
        if now >= ticket.expires_at || ticket.client != client {
            return Err(Refused);
        }
        Ok(ticket)
    }

    /// How many tickets are held, expired ones included.
    #[cfg(test)]
    pub(super) fn len(&self) -> usize {
        self.held().len()
    }
}
