//! What the hub holds in memory about sessions (D-1, R-032): unredeemed tickets, tickets
//! spent by a refused redemption, the end signal of each open tunnel and files relay
//! (`tunnel`), the host keys and RDP server certificates connection tests saw (`observed`,
//! SEC-006, SEC-007), the feed of session changes (`changes`, RA-034), and the rule for the
//! addresses sessions may reach.
//!
//! Every method takes the lock for a short, synchronous critical section and never across an
//! `.await`; time comes in as a parameter (CQ-021).

mod changes;
mod observed;
mod tunnel;

use std::{
    collections::HashMap,
    sync::{Arc, Mutex, MutexGuard, PoisonError},
    time::Duration,
};

use chrono::{DateTime, Utc};
use tokio::sync::watch;

pub(super) use self::tunnel::{Closing, FilesRelay};
pub use self::tunnel::{EndCause, Tunnel};
use self::{
    changes::Changes,
    tunnel::{EndSignal, signal},
};
use super::{
    end::Actor,
    files,
    model::EndReason,
    public_key::PublicKey,
    target::TargetRule,
    tickets::{PendingTicket, Presented, TicketHash, hash_of},
    trust::{Observations, PresentedCertificate},
};
use crate::{ids::SessionId, targets::AddressPolicy};

/// Most tickets issued and not yet redeemed or swept, spent ones included. A ticket lives
/// 30 s, so this bounds what one operator can hold in memory by starting sessions in a loop.
pub(super) const MAX_PENDING: usize = 128;

/// How long `endSession` waits by default for an open tunnel to end its row (D-10).
pub(super) const TUNNEL_END_WAIT: Duration = Duration::from_secs(5);

#[derive(Default)]
struct State {
    tickets: HashMap<TicketHash, PendingTicket>,
    /// Tickets whose token was presented and refused. The token is gone, so they never open
    /// a tunnel; they wait here until they expire so the sweep ends their rows.
    spent: HashMap<SessionId, PendingTicket>,
    tunnels: HashMap<SessionId, EndSignal>,
    /// Open files relays by registration number, and the next number.
    files_relays: HashMap<u64, EndSignal>,
    next_files_relay: u64,
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

    /// Whether the session's ticket is still unredeemed.
    fn holds_ticket(&self, session: SessionId) -> bool {
        self.tickets
            .values()
            .any(|ticket| ticket.session == session)
    }
}

/// Tickets, open tunnels, and host-key and certificate observations, shared by GraphQL and the
/// tunnel route. Cheap to clone; every clone sees the same state. `sessions::open` builds one
/// with `Live::default()`, whose target rule has no deny list; `main.rs` gives it the hub's
/// `ANIMA_PROBE_DENY` policy with [`Live::denying`].
#[derive(Clone)]
pub struct Live {
    state: Arc<Mutex<State>>,
    /// Its own lock: a connection test never waits on a ticket redemption.
    observations: Arc<Mutex<Observations<PublicKey>>>,
    /// RDP server certificates, apart from host keys: an endpoint may present both.
    certificates: Arc<Mutex<Observations<PresentedCertificate>>>,
    tunnel_end_wait: Duration,
    target_rule: TargetRule,
    /// The files tunnel's own tickets, so a session ticket never opens `/files`.
    files: files::Tickets,
    changes: Changes,
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

/// What `Live::end_by` found for a session, decided under one lock.
pub(super) enum Ending {
    /// An open tunnel was told to end; it writes the row's end.
    Signalled(Closing),
    /// A pending or spent ticket was withdrawn; the caller ends the row.
    Withdrawn(PendingTicket),
    /// Neither: the session is ended, unknown, or its row was left live.
    Neither,
}

impl Live {
    /// A `Live` whose `endSession` waits up to `wait` for an open tunnel to end its row.
    #[must_use]
    pub fn with_tunnel_end_wait(wait: Duration) -> Self {
        Self {
            state: Arc::default(),
            observations: Arc::default(),
            certificates: Arc::default(),
            tunnel_end_wait: wait,
            target_rule: TargetRule::default(),
            files: files::Tickets::default(),
            changes: Changes::default(),
        }
    }

    /// This `Live` with every session connection (the terminal, the Guacamole tunnel, the SSH
    /// probe and `testConnection`) also refusing what `policy` refuses: the
    /// `ANIMA_PROBE_DENY` networks and the policy's always-refused ranges. It
    /// replaces any policy given before.
    #[must_use]
    pub fn denying(self, policy: AddressPolicy) -> Self {
        Self {
            target_rule: self.target_rule.denying(policy),
            ..self
        }
    }

    /// This `Live` with probes that may reach an SSH server on the loopback interface, for
    /// tests only: every other refused address stays refused, and so does loopback in the
    /// deny list (R-030, R-032).
    #[cfg(any(test, feature = "test-support"))]
    #[must_use]
    pub fn reaching_loopback_for_tests(self) -> Self {
        Self {
            target_rule: self.target_rule.reaching_loopback_for_tests(),
            ..self
        }
    }

    pub(super) const fn tunnel_end_wait(&self) -> Duration {
        self.tunnel_end_wait
    }

    /// The files tunnel's tickets.
    pub(super) const fn files(&self) -> &files::Tickets {
        &self.files
    }

    /// The end signal of the session's open tunnel, for a tunnel that lives only as long as
    /// it (the files tunnel, K4 Section 9): it carries the first cause, and closes when the
    /// session's tunnel deregisters. `None` when the session has no open tunnel.
    pub(super) fn tunnel_end(
        &self,
        session: SessionId,
    ) -> Option<watch::Receiver<Option<EndCause>>> {
        self.state()
            .tunnels
            .get(&session)
            .map(watch::Sender::subscribe)
    }

    /// Registers a files relay beside the session's open tunnel, deciding under one lock so
    /// shutdown cannot slip in between: the tunnel's end signal and the relay's registration,
    /// which `shut_down` waits for until it drops. `None` when the session has no open tunnel
    /// or the hub is shutting down.
    pub(super) fn files_relay(
        &self,
        session: SessionId,
    ) -> Option<(watch::Receiver<Option<EndCause>>, FilesRelay)> {
        let mut state = self.state();
        if state.shutting_down {
            return None;
        }
        let tunnel_end = state.tunnels.get(&session)?.subscribe();
        let id = state.next_files_relay;
        state.next_files_relay = id.wrapping_add(1);
        let (registration, held) = watch::channel(None);
        state.files_relays.insert(id, registration);
        drop(state);
        let relay = FilesRelay {
            id,
            _held: held,
            live: self.clone(),
        };
        Some((tunnel_end, relay))
    }

    /// The rule every session connection applies to the addresses a host resolves to.
    pub(super) const fn target_rule(&self) -> &TargetRule {
        &self.target_rule
    }

    /// The poisoned case is taken over: no critical section here can leave the maps half
    /// written, because none of them panics between two writes.
    fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Whether `shut_down` has run: from then on no ticket is issued or redeemed.
    pub(super) fn is_shutting_down(&self) -> bool {
        self.state().shutting_down
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
    /// `now == expires_at` included), presented from another address or on another tunnel's
    /// path, or the hub is shutting down. A refused ticket is kept, without its token, until
    /// the sweep ends its row. An accepted ticket becomes a registered tunnel in the same
    /// critical section, so `end_by` always finds a started session pending, spent or live.
    ///
    /// # Errors
    /// `Refused`, with no detail, in every refused case.
    pub fn redeem(
        &self,
        presented: Presented<'_>,
        now: DateTime<Utc>,
    ) -> Result<Redeemed, Refused> {
        let hash = hash_of(presented.token).ok_or(Refused)?;
        let mut state = self.state();
        let ticket = state.tickets.remove(&hash).ok_or(Refused)?;
        if !ticket.admits(&presented, now)
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
    ///
    /// Restored tickets are never dropped, so the held count may pass `MAX_PENDING`: tickets
    /// issued while these were out count against the cap, and these come back on top. One
    /// failed sweep or end therefore adds at most `MAX_PENDING` beyond the cap (`2 ×
    /// MAX_PENDING` held), and `issue` refuses every new ticket while the count is over it.
    /// It can grow further only if the database keeps failing across sweeps; the first sweep
    /// that commits removes every expired ticket and brings it back under the cap.
    pub(super) fn restore(&self, tickets: Vec<PendingTicket>) {
        self.state()
            .spent
            .extend(tickets.into_iter().map(|ticket| (ticket.session, ticket)));
    }

    /// Tells every open tunnel to end with `HUB_SHUTDOWN`, withdraws every unredeemed and
    /// spent ticket, and refuses new tickets and redemptions from now on (D-10), all under one
    /// lock, so no ticket becomes a tunnel in between. The returned `Closing` holds the open
    /// tunnels and files relays; a files relay ends with its session's tunnel. The caller
    /// ends the withdrawn tickets' rows and bounds the wait on `Closing` (`end::shut_down`).
    pub(super) fn shut_down(&self) -> (Closing, Vec<PendingTicket>) {
        let mut state = self.state();
        state.shutting_down = true;
        let ends: Vec<EndSignal> = state.tunnels.values().cloned().collect();
        let mut tickets: Vec<PendingTicket> =
            state.tickets.drain().map(|(_, ticket)| ticket).collect();
        tickets.extend(state.spent.drain().map(|(_, ticket)| ticket));
        let relays: Vec<EndSignal> = state.files_relays.values().cloned().collect();
        drop(state);
        for end in &ends {
            let cause = EndCause {
                reason: EndReason::HubShutdown,
                by: None,
            };
            signal(end, cause);
        }
        (Closing([ends, relays].concat()), tickets)
    }
}
