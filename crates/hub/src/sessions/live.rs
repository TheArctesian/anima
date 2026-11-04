//! What the hub holds in memory about sessions (D-1, R-032): unredeemed tickets, tickets
//! spent by a refused redemption, and the end signal of each open tunnel.
//!
//! Every method takes the lock for a short, synchronous critical section and never across an
//! `.await`; time comes in as a parameter (CQ-021).

use std::{
    collections::HashMap,
    sync::{Arc, Mutex, MutexGuard, PoisonError},
    time::Duration,
};

use chrono::{DateTime, Utc};
use tokio::sync::watch;

use super::{
    end::Actor,
    model::EndReason,
    tickets::{PendingTicket, TicketHash, hash_of},
};
use crate::{events::ClientAddr, ids::SessionId};

/// Most tickets issued and not yet redeemed or swept, spent ones included. A ticket lives
/// 30 s, so this bounds what one operator can hold in memory by starting sessions in a loop.
pub(super) const MAX_PENDING: usize = 128;

/// How long `endSession` waits by default for an open tunnel to end its row (D-10).
pub(super) const TUNNEL_END_WAIT: Duration = Duration::from_secs(5);

/// Why a tunnel is told to end from outside, and who asked: the operator who called
/// `endSession` (for the tunnel's `session.end` audit entry, LOG-005), or nobody when the hub
/// shuts down.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EndCause {
    pub reason: EndReason,
    pub by: Option<Actor>,
}

/// The end signal of one tunnel: `None` until the first cause, which wins.
type EndSignal = watch::Sender<Option<EndCause>>;

#[derive(Default)]
struct State {
    tickets: HashMap<TicketHash, PendingTicket>,
    /// Tickets whose token was presented and refused. The token is gone, so they never open
    /// a tunnel; they wait here until they expire so the sweep ends their rows.
    spent: HashMap<SessionId, PendingTicket>,
    tunnels: HashMap<SessionId, EndSignal>,
    shutting_down: bool,
}

impl State {
    /// Removes the session's spent or unredeemed ticket.
    fn withdraw(&mut self, session: SessionId) -> Option<PendingTicket> {
        if let Some(spent) = self.spent.remove(&session) {
            return Some(spent);
        }
        let hash = self
            .tickets
            .iter()
            .find_map(|(hash, ticket)| (ticket.session == session).then_some(*hash))?;
        self.tickets.remove(&hash)
    }
}

/// Tickets and open tunnels, shared by GraphQL and the tunnel route. Cheap to clone; every
/// clone sees the same state. `main.rs` builds one with `Live::default()` (R-032).
#[derive(Clone)]
pub struct Live {
    state: Arc<Mutex<State>>,
    tunnel_end_wait: Duration,
}

impl Default for Live {
    fn default() -> Self {
        Self::with_tunnel_end_wait(TUNNEL_END_WAIT)
    }
}

/// A ticket was not accepted: unknown, used, expired or presented from another address.
/// Carries no detail, so the HTTP 403 cannot tell them apart (K4 §2).
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
#[error("the ticket is not valid")]
pub struct Refused;

/// The hub holds no more pending tickets, or it is shutting down.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
#[error("no ticket can be issued now")]
pub(super) struct Unavailable;

/// A redeemed ticket and the tunnel registration it became.
pub struct Redeemed {
    pub ticket: PendingTicket,
    pub tunnel: Tunnel,
}

/// One open tunnel's registration. It receives the cause that ends the tunnel from outside
/// (`endSession`, shutdown). Dropping it deregisters the tunnel, so the tunnel task must end
/// the session row before it drops this: `endSession` waits for the drop, then reads the row.
pub struct Tunnel {
    session: SessionId,
    cause: watch::Receiver<Option<EndCause>>,
    live: Live,
}

impl Tunnel {
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

/// The tunnels that were told to end; `finished` waits until each has deregistered.
pub struct Closing(Vec<EndSignal>);

impl Closing {
    /// Waits until every signalled tunnel has dropped its `Tunnel`. Callers bound the wait.
    pub async fn finished(self) {
        for signal in &self.0 {
            signal.closed().await;
        }
    }
}

/// What `Live::end_by` found for a session, decided under one lock.
pub(super) enum Ending {
    /// An open tunnel was told to end; it writes the row's end.
    Signalled(Closing),
    /// A pending or spent ticket was withdrawn; the caller ends the row.
    Withdrawn(PendingTicket),
    /// Neither: the session is ended, unknown, or its row was left live.
    Neither,
}

/// Sets the cause unless one is already set: the first cause wins (D-9).
fn signal(end: &EndSignal, cause: EndCause) {
    end.send_if_modified(|current| {
        let first = current.is_none();
        if first {
            *current = Some(cause);
        }
        first
    });
}

impl Live {
    /// A `Live` whose `endSession` waits up to `wait` for an open tunnel to end its row.
    #[must_use]
    pub fn with_tunnel_end_wait(wait: Duration) -> Self {
        Self {
            state: Arc::default(),
            tunnel_end_wait: wait,
        }
    }

    pub(super) const fn tunnel_end_wait(&self) -> Duration {
        self.tunnel_end_wait
    }

    /// The poisoned case is taken over: no critical section here can leave the maps half
    /// written, because none of them panics between two writes.
    fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Holds a new ticket under its hash.
    pub(super) fn issue(&self, hash: TicketHash, ticket: PendingTicket) -> Result<(), Unavailable> {
        let mut state = self.state();
        if state.shutting_down || state.tickets.len() + state.spent.len() >= MAX_PENDING {
            return Err(Unavailable);
        }
        state.tickets.insert(hash, ticket);
        Ok(())
    }

    /// Redeems a presented token (K4 §1). The token is removed first, valid or not, so it
    /// works at most once; then it is refused when expired at `now` (the boundary
    /// `now == expires_at` included), presented from another address, or the hub is shutting
    /// down. A refused ticket is kept, without its token, until the sweep ends its row. An
    /// accepted ticket becomes a registered tunnel in the same critical section, so
    /// `end_by` always finds a started session pending, spent or live.
    ///
    /// # Errors
    /// `Refused`, with no detail, in every refused case.
    pub fn redeem(
        &self,
        presented: &str,
        client: ClientAddr,
        now: DateTime<Utc>,
    ) -> Result<Redeemed, Refused> {
        let hash = hash_of(presented).ok_or(Refused)?;
        let mut state = self.state();
        let ticket = state.tickets.remove(&hash).ok_or(Refused)?;
        if !ticket.admits(client, now)
            || state.shutting_down
            || state.tunnels.contains_key(&ticket.session)
        {
            state.spent.insert(ticket.session, ticket);
            return Err(Refused);
        }
        let (end, cause) = watch::channel(None);
        state.tunnels.insert(ticket.session, end);
        drop(state);
        let tunnel = Tunnel {
            session: ticket.session,
            cause,
            live: self.clone(),
        };
        Ok(Redeemed { ticket, tunnel })
    }

    /// Ends the session for `by` (`endSession`, D-10), deciding under one lock so a
    /// redemption cannot slip between the two cases: an open tunnel is told to end with
    /// `OPERATOR` and who asked; otherwise a pending or spent ticket is withdrawn.
    pub(super) fn end_by(&self, session: SessionId, by: &Actor) -> Ending {
        let mut state = self.state();
        if let Some(end) = state.tunnels.get(&session) {
            let cause = EndCause {
                reason: EndReason::Operator,
                by: Some(by.clone()),
            };
            signal(end, cause);
            return Ending::Signalled(Closing(vec![end.clone()]));
        }
        state
            .withdraw(session)
            .map_or(Ending::Neither, Ending::Withdrawn)
    }

    /// Removes and returns the session's unredeemed or spent ticket, if it has one.
    pub(super) fn withdraw(&self, session: SessionId) -> Option<PendingTicket> {
        self.state().withdraw(session)
    }

    /// Removes and returns every unredeemed or spent ticket expired at `now`. At most
    /// `MAX_PENDING`.
    pub(super) fn take_expired(&self, now: DateTime<Utc>) -> Vec<PendingTicket> {
        let mut state = self.state();
        let mut expired: Vec<PendingTicket> = state
            .tickets
            .extract_if(|_, ticket| ticket.is_expired(now))
            .map(|(_, ticket)| ticket)
            .collect();
        expired.extend(
            state
                .spent
                .extract_if(|_, ticket| ticket.is_expired(now))
                .map(|(_, ticket)| ticket),
        );
        expired
    }

    /// Puts back withdrawn tickets whose rows could not be ended. They go back without a
    /// token, so they stay refused, and the next sweep ends them once they expire.
    pub(super) fn restore(&self, tickets: Vec<PendingTicket>) {
        self.state()
            .spent
            .extend(tickets.into_iter().map(|ticket| (ticket.session, ticket)));
    }

    /// Tells every open tunnel to end with `HUB_SHUTDOWN` and refuses new tickets and
    /// redemptions from now on (D-10). The caller bounds the wait on the returned `Closing`.
    #[must_use]
    pub fn shut_down(&self) -> Closing {
        let mut state = self.state();
        state.shutting_down = true;
        let ends: Vec<EndSignal> = state.tunnels.values().cloned().collect();
        drop(state);
        for end in &ends {
            let cause = EndCause {
                reason: EndReason::HubShutdown,
                by: None,
            };
            signal(end, cause);
        }
        Closing(ends)
    }
}
