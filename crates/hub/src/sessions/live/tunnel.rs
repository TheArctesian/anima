//! The registrations of what a session holds open: each tunnel, with the signal that ends it
//! from outside, and each files relay, which hub shutdown waits for.

use tokio::sync::watch;

use super::Live;
use crate::{
    ids::SessionId,
    sessions::{end::Actor, model::EndReason},
};

/// Why a tunnel is told to end from outside, and who asked: the operator who called
/// `endSession` (for the tunnel's `session.end` audit entry, LOG-005), or nobody when the hub
/// shuts down.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EndCause {
    pub reason: EndReason,
    pub by: Option<Actor>,
}

/// The end signal of one tunnel: `None` until the first cause, which wins. A files relay's
/// registration has one too, which nobody signals: shutdown only waits for it to close.
pub(super) type EndSignal = watch::Sender<Option<EndCause>>;

/// One open tunnel's registration. It receives the cause that ends the tunnel from outside
/// (`endSession`, shutdown). Dropping it deregisters the tunnel, so the tunnel task must end
/// the session row before it drops this: `endSession` waits for the drop, then reads the row.
pub struct Tunnel {
    pub(super) session: SessionId,
    pub(super) cause: watch::Receiver<Option<EndCause>>,
    pub(super) live: Live,
}

impl Tunnel {
    /// The cause signalled so far, if any, without waiting: for a tunnel task that can no
    /// longer await, such as one being dropped.
    pub(in crate::sessions) fn signalled(&self) -> Option<EndCause> {
        self.cause.borrow().clone()
    }

    /// The `Live` this tunnel is registered with, so its task can publish the end it wrote.
    pub(in crate::sessions) const fn live(&self) -> &Live {
        &self.live
    }

    /// Resolves with the first cause signalled for this tunnel. Cancel safe, so it can sit in
    /// a `select!`. The sender lives until this registration drops, so a closed channel means
    /// the hub's state is being torn down: that is a shutdown.
    pub async fn ended(&mut self) -> EndCause {
        loop {
            if let Some(cause) = self.cause.borrow_and_update().clone() {
                return cause;
            }
            if self.cause.changed().await.is_err() {
                return EndCause {
                    reason: EndReason::HubShutdown,
                    by: None,
                };
            }
        }
    }
}

impl Drop for Tunnel {
    fn drop(&mut self) {
        let mut state = self.live.state();
        if state
            .tunnels
            .get(&self.session)
            .is_some_and(|signal| self.cause.same_channel(&signal.subscribe()))
        {
            state.tunnels.remove(&self.session);
        }
    }
}

/// One files relay's registration, held until the relay has written the audit entry of any
/// transfer it cut short. Hub shutdown waits for it to drop, within the tunnel end wait;
/// `endSession` does not.
pub(in crate::sessions) struct FilesRelay {
    pub(super) id: u64,
    pub(super) _held: watch::Receiver<Option<EndCause>>,
    pub(super) live: Live,
}

impl Drop for FilesRelay {
    fn drop(&mut self) {
        self.live.state().files_relays.remove(&self.id);
    }
}

/// The tunnels and files relays shutdown or `endSession` waits for; `finished` waits until
/// each has deregistered.
pub(in crate::sessions) struct Closing(pub(super) Vec<EndSignal>);

impl Closing {
    /// Waits until every registration has dropped. Callers bound the wait.
    pub(in crate::sessions) async fn finished(self) {
        for signal in &self.0 {
            signal.closed().await;
        }
    }
}

/// Sets the cause unless one is already set: the first cause wins (D-9).
pub(super) fn signal(end: &EndSignal, cause: EndCause) {
    end.send_if_modified(|current| {
        let first = current.is_none();
        if first {
            *current = Some(cause);
        }
        first
    });
}
