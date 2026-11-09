//! `shareSession` (S-4, K4 §10): a share of a live session the caller started, while its
//! tunnel is open, with a token shown once.

use tokio::sync::watch;
use uuid::Uuid;

use super::{
    Error, Sharing,
    keeper::Keeper,
    model::{SessionShare, ShareEnd},
    registry::{Share, Shares},
    terms::Terms,
    trail,
};
use crate::{
    ids::{SessionId, ShareId},
    sessions::{
        Actor, EndCause, Session, store,
        tickets::{TOKEN_BYTES, TicketToken},
    },
};

/// A held share that has no keeper yet. Dropped before [`Unkept::kept`], because its opening
/// failed, panicked or was cancelled, it withdraws the share, so no share is held without a
/// keeper (S-5).
#[must_use = "dropping it withdraws the share"]
pub(super) struct Unkept {
    shares: Shares,
    id: ShareId,
    kept: bool,
}

impl Unkept {
    /// Watches over the share `id`, which `shares` has just opened.
    pub(super) const fn new(shares: Shares, id: ShareId) -> Self {
        Self {
            shares,
            id,
            kept: false,
        }
    }

    /// The share's keeper runs: the share is its to end.
    pub(super) fn kept(mut self) {
        self.kept = true;
    }
}

impl Drop for Unkept {
    fn drop(&mut self) {
        if !self.kept {
            self.shares.remove(self.id);
        }
    }
}

/// Random bytes from the operating system.
fn random_token() -> Result<TicketToken, Error> {
    let mut bytes = [0; TOKEN_BYTES];
    getrandom::fill(&mut bytes).map_err(Error::Random)?;
    Ok(TicketToken::from_random(bytes))
}

/// The end signal of `row`'s open tunnel: the session is live and a redeemed ticket registered
/// its tunnel, which nothing has told to end yet. A pending ticket is not an open tunnel.
fn open_tunnel(sharing: &Sharing, row: &Session) -> Option<watch::Receiver<Option<EndCause>>> {
    if !row.is_live() {
        return None;
    }
    let tunnel_end = sharing.live.tunnel_end(row.id)?;
    let ending = tunnel_end.borrow().is_some();
    (!ending).then_some(tunnel_end)
}

/// Shares `session` for `by` on `terms` (S-4): writes the audit entry `share.create` and its
/// event in one transaction, then starts the share's keeper, which ends it at expiry, on
/// `endShare` or when the session ends. Returns the share and its token, which the hub keeps
/// only as a hash.
///
/// # Errors
/// `UnknownSession` when there is no such session; `NotOwner` when another operator started
/// it; `NotOpen` when it has ended or its tunnel is not open; `Unavailable` when the hub is
/// shutting down or holds 32 shares; `Sessions`, `Database`, `Events` or `Random` when a
/// dependency fails, and then no share is held; `Stopped` when the task that writes
/// `share.create` panicked or the runtime is shutting down.
pub(super) async fn share_session(
    sharing: &Sharing,
    by: &Actor,
    session: SessionId,
    terms: Terms,
) -> Result<(SessionShare, TicketToken), Error> {
    if sharing.live.is_shutting_down() {
        return Err(Error::Unavailable);
    }
    let row = store::get(&sharing.db, session)
        .await?
        .ok_or(Error::UnknownSession(session))?;
    if row.operator != by.operator.id {
        return Err(Error::NotOwner);
    }
    let tunnel_end = open_tunnel(sharing, &row).ok_or(Error::NotOpen)?;
    let token = random_token()?;
    let share = Share {
        id: ShareId::from(Uuid::now_v7()),
        session,
        owner: by.operator.clone(),
        access: terms.access,
        max_viewers: terms.max_viewers,
        viewers: 0,
        created_at: by.at,
        expires_at: by.at + terms.lifetime,
        end: watch::Sender::new(None::<ShareEnd>),
    };
    let view = share.view();
    let ended = sharing
        .live
        .shares()
        .open(token.hash(), share)
        .map_err(|_| Error::Unavailable)?;
    let unkept = Unkept::new(sharing.live.shares().clone(), view.id);
    let keeper = Keeper {
        sharing: sharing.clone(),
        id: view.id,
        expires_at: view.expires_at,
        client: by.client,
        device: row.device.map(|id| (id, row.device_name.clone())),
        ended,
        tunnel_end,
    };
    // Dropping a join handle does not cancel its task: once the share is held, the task runs
    // to the end even when the caller goes away, so a held share always has its keeper. If
    // the task panics or the runtime drops it first, `unkept` withdraws the share.
    tokio::spawn(created_then_kept(keeper, unkept, by.clone(), view.clone()))
        .await
        .map_err(Error::Stopped)??;
    Ok((view, token))
}

/// Writes `share.create` by `by` for the held share `view`, then hands the `unkept` share to
/// `keeper`. When the write fails, it withdraws the share instead.
async fn created_then_kept(
    keeper: Keeper,
    unkept: Unkept,
    by: Actor,
    view: SessionShare,
) -> Result<(), Error> {
    if let Err(error) = trail::created(&keeper.sharing, &by, keeper.device.clone(), &view).await {
        // No `share.create`, no share: dropped, `unkept` withdraws it.
        drop(unkept);
        return Err(error);
    }
    tokio::spawn(keeper.keep());
    unkept.kept();
    Ok(())
}
