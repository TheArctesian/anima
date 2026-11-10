//! The live shares, by the SHA-256 of their token (S-1, S-2, K4 §10): the only form of a share
//! token the hub keeps, in memory (LOG-007).
//!
//! Every method takes the lock for a short, synchronous critical section and never across an
//! `.await`.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex, MutexGuard, PoisonError},
};

use chrono::{DateTime, Utc};
use tokio::sync::watch;

use super::{
    Error,
    model::{SessionShare, ShareAccess, ShareEnd},
};
use crate::{
    events::{ClientAddr, Operator},
    ids::{SessionId, ShareId},
    sessions::tickets::TicketHash,
};

/// Most live shares a hub holds (D-9, S-3). It also bounds the keeper tasks, one per share.
pub(super) const MAX_SHARES: usize = 32;

/// The end signal of one share: `None` until the first cause, which wins. Its keeper and,
/// from T4 on, each viewer hold a receiver; the sender drops when the keeper removes the share.
pub(super) type EndSignal = watch::Sender<Option<ShareEnd>>;

/// One live share. It holds no token: the registry keys it by the token's hash.
pub(super) struct Share {
    pub(super) id: ShareId,
    pub(super) session: SessionId,
    /// The operator who started the session and shared it.
    pub(super) owner: Operator,
    pub(super) access: ShareAccess,
    pub(super) max_viewers: u8,
    /// Viewers connected now: 0 until the watch tunnel admits viewers.
    pub(super) viewers: u8,
    pub(super) created_at: DateTime<Utc>,
    pub(super) expires_at: DateTime<Utc>,
    pub(super) end: EndSignal,
}

impl Share {
    /// The share as the operator sees it.
    pub(super) fn view(&self) -> SessionShare {
        SessionShare {
            id: self.id,
            session: self.session,
            access: self.access,
            max_viewers: self.max_viewers,
            viewers: self.viewers,
            created_at: self.created_at,
            expires_at: self.expires_at,
        }
    }

    /// Whether a cause has ended the share, though its keeper may not have removed it yet.
    fn is_ending(&self) -> bool {
        self.end.borrow().is_some()
    }
}

/// The hub holds [`MAX_SHARES`] shares.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
#[error("no share can be held now")]
pub(super) struct Full;

/// Sets the cause unless one is already set: the first cause wins. Returns the cause that
/// stands.
pub(super) fn settle(end: &EndSignal, cause: ShareEnd) -> ShareEnd {
    end.send_if_modified(|current| {
        let first = current.is_none();
        if first {
            *current = Some(cause);
        }
        first
    });
    end.borrow().unwrap_or(cause)
}

/// The live shares, by token hash. Cheap to clone; clones share the shares.
#[derive(Clone, Default)]
pub(in crate::sessions) struct Shares(Arc<Mutex<HashMap<TicketHash, Share>>>);

impl Shares {
    /// The poisoned case is taken over: no critical section here panics between two writes.
    fn held(&self) -> MutexGuard<'_, HashMap<TicketHash, Share>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Holds `share` under its token's `hash` and returns a receiver of its end signal.
    ///
    /// # Errors
    /// [`Full`] when [`MAX_SHARES`] shares are held, ending ones included, or (never in
    /// practice: 256 random bits) a share already has this hash.
    pub(super) fn open(
        &self,
        hash: TicketHash,
        share: Share,
    ) -> Result<watch::Receiver<Option<ShareEnd>>, Full> {
        let mut held = self.held();
        if held.len() >= MAX_SHARES || held.contains_key(&hash) {
            return Err(Full);
        }
        let ended = share.end.subscribe();
        held.insert(hash, share);
        Ok(ended)
    }

    /// The shares of `session` that no cause has ended, oldest first.
    pub(in crate::sessions) fn of_session(&self, session: SessionId) -> Vec<SessionShare> {
        let mut shares: Vec<SessionShare> = self
            .held()
            .values()
            .filter(|share| share.session == session && !share.is_ending())
            .map(Share::view)
            .collect();
        shares.sort_by_key(|share| (share.created_at, share.id.to_string()));
        shares
    }

    /// Ends the share for its owner, `by`, calling from `client` (`endShare`): signals its
    /// keeper, which removes it.
    ///
    /// # Errors
    /// `UnknownShare` when no share has this id or a cause has already ended it; `NotOwner`
    /// when `by` is not the operator who shared it.
    pub(super) fn end_by(
        &self,
        id: ShareId,
        by: &Operator,
        client: ClientAddr,
    ) -> Result<(), Error> {
        let held = self.held();
        let share = held
            .values()
            .find(|share| share.id == id && !share.is_ending())
            .ok_or(Error::UnknownShare(id))?;
        if share.owner.id != by.id {
            return Err(Error::NotOwner);
        }
        settle(&share.end, ShareEnd::Ended(client));
        Ok(())
    }

    /// Removes and returns the share with this id.
    pub(super) fn remove(&self, id: ShareId) -> Option<Share> {
        Self::removed(&mut self.held(), id)
    }

    /// Removes the share with this id for its keeper, which woke for `woke_for`, and settles its
    /// end (S-5): a cause already set, by `endShare`, stands; else `woke_for` does. Both happen
    /// under one lock, so no `endShare` lands between them and the cause returned is final.
    pub(super) fn take(&self, id: ShareId, woke_for: ShareEnd) -> Option<(Share, ShareEnd)> {
        let mut held = self.held();
        let share = Self::removed(&mut held, id)?;
        let end = settle(&share.end, woke_for);
        Some((share, end))
    }

    fn removed(held: &mut HashMap<TicketHash, Share>, id: ShareId) -> Option<Share> {
        let hash = held
            .iter()
            .find_map(|(hash, share)| (share.id == id).then_some(*hash))?;
        held.remove(&hash)
    }

    /// The token hashes held, for tests that check no token is kept.
    #[cfg(test)]
    pub(super) fn hashes(&self) -> Vec<TicketHash> {
        self.held().keys().copied().collect()
    }
}
