//! The files tunnel's tickets (K4 Sections 1 and 9, WP-27 D-5): the same tokens as session
//! tickets, kept apart from them so a `/terminal` ticket never opens `/files` and a files
//! ticket never opens a terminal. Only the SHA-256 of a token is held, in memory (LOG-007).
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
    ids::{DeviceId, SessionId},
    sessions::{
        Refused,
        tickets::{TicketHash, hash_of},
    },
};

/// Most files tickets held at once. A ticket lives 30 s and holds no row, so expired ones are
/// dropped as new ones are issued; this bounds what a client asking in a loop can hold.
pub(super) const MAX_PENDING: usize = 32;

/// An issued files ticket: who may open the files tunnel of which live SSH session, from
/// where, and until when.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct FilesTicket {
    pub(super) operator: Operator,
    pub(super) client: ClientAddr,
    pub(super) session: SessionId,
    pub(super) device: DeviceId,
    pub(super) expires_at: DateTime<Utc>,
}

/// Too many files tickets are pending.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
#[error("no files ticket can be issued now")]
pub(super) struct Full;

/// The pending files tickets, by token hash. Cheap to clone; clones share the tickets.
#[derive(Clone, Default)]
pub(in crate::sessions) struct Tickets(Arc<Mutex<HashMap<TicketHash, FilesTicket>>>);

impl Tickets {
    /// The poisoned case is taken over: no critical section here panics between two writes.
    fn held(&self) -> MutexGuard<'_, HashMap<TicketHash, FilesTicket>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Holds `ticket` under `hash`, after dropping every ticket expired at `now`.
    ///
    /// # Errors
    /// [`Full`] when [`MAX_PENDING`] unexpired tickets are held.
    pub(super) fn issue(
        &self,
        hash: TicketHash,
        ticket: FilesTicket,
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

    /// Redeems a presented token (K4 Section 1). The token is removed first, valid or not, so
    /// it works at most once; then it is refused when expired at `now` (the boundary
    /// `now == expires_at` included) or presented from another address than `client`.
    ///
    /// # Errors
    /// [`Refused`], with no detail, in every refused case.
    pub(super) fn redeem(
        &self,
        presented: &str,
        client: ClientAddr,
        now: DateTime<Utc>,
    ) -> Result<FilesTicket, Refused> {
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
