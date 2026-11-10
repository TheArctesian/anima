//! The one task per share that ends it (S-5): it waits for the first of expiry, `endShare` and
//! the end of the owner's session, removes the share, then writes `share.end` and its event.
//! A keeper frees its share's slot before it writes, so beyond S-3's 32 waiting keepers a few
//! more may still be writing.

use chrono::{DateTime, Utc};
use tokio::sync::watch;

use super::{
    Sharing,
    model::ShareEnd,
    trail::{self, Ended},
};
use crate::{
    events::ClientAddr,
    ids::{DeviceId, ShareId},
    sessions::EndCause,
};

/// What one share's keeper waits on and writes with.
pub(super) struct Keeper {
    pub(super) sharing: Sharing,
    pub(super) id: ShareId,
    pub(super) expires_at: DateTime<Utc>,
    /// Where the owner shared from: the address the audit entry names unless `endShare` ends
    /// the share from another one.
    pub(super) client: ClientAddr,
    /// The session's device, by id and name, for the event.
    pub(super) device: Option<(DeviceId, String)>,
    /// The share's own end signal, which `endShare` sets.
    pub(super) ended: watch::Receiver<Option<ShareEnd>>,
    /// The end signal of the owner's tunnel (`Live::tunnel_end`): it carries the cause the
    /// tunnel is told to end with, and closes when the tunnel deregisters.
    pub(super) tunnel_end: watch::Receiver<Option<EndCause>>,
}

/// Resolves with the cause `endShare` set. Cancel safe. The sender lives in the registry
/// until this keeper removes the share, so it never closes while this waits.
async fn signalled(ended: &mut watch::Receiver<Option<ShareEnd>>) -> ShareEnd {
    loop {
        if let Some(end) = *ended.borrow_and_update() {
            return end;
        }
        if ended.changed().await.is_err() {
            return std::future::pending().await;
        }
    }
}

/// Resolves once the owner's tunnel is told to end or has deregistered. Cancel safe.
async fn owner_ended(tunnel_end: &mut watch::Receiver<Option<EndCause>>) {
    loop {
        if tunnel_end.borrow_and_update().is_some() {
            return;
        }
        if tunnel_end.changed().await.is_err() {
            return;
        }
    }
}

/// The address `share.end` records for `end`: `endShare`'s caller, else where the owner
/// `shared_from`.
pub(super) fn recorded_from(end: ShareEnd, shared_from: ClientAddr) -> ClientAddr {
    match end {
        ShareEnd::Ended(by) => by,
        ShareEnd::Expired | ShareEnd::SessionEnded => shared_from,
    }
}

impl Keeper {
    /// Waits for the share's first end, then ends it ([`Keeper::end`]).
    pub(super) async fn keep(mut self) {
        let woke_for = self.first_end().await;
        self.end(woke_for).await;
    }

    /// Ends the share this keeper woke for `woke_for`: takes it from the registry with the
    /// cause that stands ([`Shares::take`]), so a cause `endShare` set first wins and the share
    /// ends once, with one cause. Then writes `share.end` with that cause. A failed write is
    /// logged with the share's id and no token; the share is removed all the same.
    ///
    /// The owner tunnel's receiver drops only after the write, so `endSession` and hub
    /// shutdown, which wait for it within their tunnel end wait, return after `share.end`.
    ///
    /// [`Shares::take`]: super::Shares::take
    pub(super) async fn end(self, woke_for: ShareEnd) {
        let Some((share, end)) = self.sharing.live.shares().take(self.id, woke_for) else {
            return;
        };
        let ended = Ended {
            id: self.id,
            owner: share.owner.clone(),
            client: recorded_from(end, self.client),
            device: self.device,
            end,
        };
        // Viewers see the sender close once the share is gone.
        drop(share);
        if let Err(error) = trail::ended(&self.sharing, ended).await {
            tracing::error!(
                share = %self.id,
                %error,
                "the end of a session share could not be recorded"
            );
        }
    }

    /// The first cause to end the share. When several are ready at once, `endShare` comes
    /// first, then the session's end, then expiry.
    async fn first_end(&mut self) -> ShareEnd {
        let left = (self.expires_at - self.sharing.clock.now())
            .to_std()
            .unwrap_or_default();
        tokio::select! {
            biased;
            end = signalled(&mut self.ended) => end,
            () = owner_ended(&mut self.tunnel_end) => ShareEnd::SessionEnded,
            () = tokio::time::sleep(left) => ShareEnd::Expired,
        }
    }
}
