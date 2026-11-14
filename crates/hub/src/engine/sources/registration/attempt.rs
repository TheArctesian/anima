//! One attempt to connect GitHub: its phases, and every step and refusal between them (K-E1
//! Section 7a, PRJ-039). Pure: the time is a parameter, and the values come in already minted.
//!
//! The App's private key lives only in the `AppCreated` and `Finishing` stages, so every step
//! out of them (cancel, failure, expiry, success) drops it. The attempt has no `Debug` and no
//! `Clone`; what may be shown leaves as a [`Snapshot`].
#![expect(
    clippy::unused_async_trait_impl,
    reason = "the async-graphql Enum derive generates async trait impls"
)]

use async_graphql::Enum;
use chrono::{DateTime, TimeDelta, Utc};
use source_exchange::Owner;
use uuid::Uuid;
use zeroize::Zeroizing;

use super::binding::Hash;
use crate::{
    fleet::events::{ActorKind, ClientAddr, Operator},
    ids::SourceAccountId,
};

/// How long an attempt lives from its start, in every phase (GitHub's code lives one hour).
pub const LIFETIME: TimeDelta = TimeDelta::minutes(55);
/// How long a ticket works after it is issued.
pub const TICKET_LIFETIME: TimeDelta = TimeDelta::minutes(5);
/// How long an ended attempt stays readable.
pub const READABLE_FOR: TimeDelta = TimeDelta::minutes(10);

/// The phase the API shows (K1 `GithubConnectPhase`). The internal `Finishing` shows as
/// `AppCreated`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
#[graphql(name = "GithubConnectPhase")]
pub enum Phase {
    WaitingForBrowser,
    AtGithub,
    AppCreated,
    Connected,
    Failed,
    Cancelled,
}

/// Why an attempt failed (K1 `GithubConnectFailure`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
#[graphql(name = "GithubConnectFailure")]
pub enum Failure {
    Expired,
    ExchangeRefused,
    GithubUnreachable,
    ReplyUnusable,
    KeyRefused,
}

/// Why a step was refused; the attempt is unchanged. Fixed words (LOG-007).
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Refusal {
    #[error("no attempt with this id is open")]
    NotFound,
    #[error("the attempt is not in a phase that allows this")]
    Conflict,
    #[error("an App is made and waits for Install")]
    AppWaiting,
    #[error("the link was already used or has ended")]
    Ticket,
    #[error("this browser is not the one bound to the attempt")]
    Unbound,
    #[error("the state does not match")]
    State,
}

/// The App GitHub made: its facts from the conversion reply, text from GitHub.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct App {
    pub id: u64,
    pub slug: String,
    pub owner: Owner,
}

/// Who started the attempt and from where, captured at the start for every audit entry and
/// for the store at the end.
#[derive(Clone)]
pub struct Starter {
    pub operator: Operator,
    pub client: ClientAddr,
}

/// What the operator asked for at the start.
pub struct Request {
    pub label: Option<String>,
    pub organisation: Option<String>,
    /// The App's name, `anima-` and ten characters.
    pub name: String,
    pub by: Starter,
}

enum Stage {
    WaitingForBrowser,
    AtGithub {
        exchanging: bool,
    },
    AppCreated {
        key: Zeroizing<String>,
    },
    Finishing {
        key: Zeroizing<String>,
    },
    Connected {
        account: SourceAccountId,
        consumer_on: bool,
    },
    Failed(Failure),
    Cancelled,
}

struct Ticket {
    hash: Hash,
    issued_at: DateTime<Utc>,
}

/// What a hit on `/created` may do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Exchange {
    /// The state matched and is now spent: convert the code.
    Fresh,
    /// The App is already made: convert nothing, send the browser to Install again.
    Made,
}

/// What the store at the end needs: taken under the lock with the stage `Finishing`.
pub struct Finish {
    pub label: String,
    pub app_id: u64,
    pub key: Zeroizing<String>,
    pub by: Starter,
}

/// How the store at the end went.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Finished {
    Connected {
        account: SourceAccountId,
        consumer_on: bool,
    },
    /// Unreachable or a database error: back to `AppCreated` for the next poll.
    Again,
    Failed(Failure),
}

/// What may be shown of an attempt: no key, ticket, cookie or state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub id: Uuid,
    pub phase: Phase,
    pub started_by: ActorKind,
    pub started_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub app: Option<App>,
    pub failure: Option<Failure>,
    pub account: Option<SourceAccountId>,
    pub consumer_on: bool,
}

/// One attempt. No `Debug` and no `Clone`: it holds the key between Create and Install.
pub struct Attempt {
    id: Uuid,
    request: Request,
    started_at: DateTime<Utc>,
    stage: Stage,
    ticket: Option<Ticket>,
    binding: Option<Hash>,
    state: Option<Hash>,
    app: Option<App>,
    ended_at: Option<DateTime<Utc>>,
}

impl Attempt {
    /// A new attempt `id`, waiting for the browser.
    #[must_use]
    pub const fn start(id: Uuid, request: Request, now: DateTime<Utc>) -> Self {
        Self {
            id,
            request,
            started_at: now,
            stage: Stage::WaitingForBrowser,
            ticket: None,
            binding: None,
            state: None,
            app: None,
            ended_at: None,
        }
    }

    #[must_use]
    pub const fn id(&self) -> Uuid {
        self.id
    }

    #[must_use]
    pub const fn request(&self) -> &Request {
        &self.request
    }

    #[must_use]
    pub fn expires_at(&self) -> DateTime<Utc> {
        self.started_at + LIFETIME
    }

    /// Whether an App is made and not yet installed: a new start must not replace it.
    #[must_use]
    pub const fn holds_app(&self) -> bool {
        matches!(
            self.stage,
            Stage::AppCreated { .. } | Stage::Finishing { .. }
        )
    }

    /// Whether the attempt has ended: connected, failed or cancelled.
    #[must_use]
    pub const fn ended(&self) -> bool {
        self.ended_at.is_some()
    }

    /// Whether the API may still show it at `now`: open, or ended under 10 minutes ago.
    #[must_use]
    pub fn readable(&self, now: DateTime<Utc>) -> bool {
        self.ended_at.is_none_or(|ended| now < ended + READABLE_FOR)
    }

    fn end(&mut self, stage: Stage, now: DateTime<Utc>) {
        self.stage = stage;
        self.ended_at = Some(now);
        self.ticket = None;
        self.state = None;
    }

    /// Fails the attempt with `EXPIRED` when its 55 minutes are over at `now`, in any open
    /// phase but the short store at the end, whose outcome decides. Returns whether it did.
    pub fn expire(&mut self, now: DateTime<Utc>) -> bool {
        let due = !self.ended() && now >= self.expires_at();
        if due && !matches!(self.stage, Stage::Finishing { .. }) {
            let at = self.expires_at();
            self.end(Stage::Failed(Failure::Expired), at);
            return true;
        }
        false
    }

    const fn waiting_for_browser(&self) -> bool {
        matches!(
            self.stage,
            Stage::WaitingForBrowser | Stage::AtGithub { exchanging: false }
        )
    }

    /// Issues a new ticket with `hash`; an earlier one stops working. Returns when it ends.
    ///
    /// # Errors
    /// `Conflict` outside `WAITING_FOR_BROWSER` and `AT_GITHUB`.
    pub fn issue_ticket(
        &mut self,
        hash: Hash,
        now: DateTime<Utc>,
    ) -> Result<DateTime<Utc>, Refusal> {
        if !self.waiting_for_browser() {
            return Err(Refusal::Conflict);
        }
        self.ticket = Some(Ticket {
            hash,
            issued_at: now,
        });
        Ok(now + TICKET_LIFETIME)
    }

    /// Binds the browser that presented the ticket `presented` to the attempt with the cookie
    /// hash `binding`, and spends the ticket.
    ///
    /// # Errors
    /// `Ticket` for a wrong, used or old ticket; `Conflict` outside the phases that wait for
    /// a browser.
    pub fn claim_ticket(
        &mut self,
        presented: &str,
        binding: Hash,
        now: DateTime<Utc>,
    ) -> Result<(), Refusal> {
        let valid = self.ticket.as_ref().is_some_and(|ticket| {
            ticket.hash.matches(presented) && now < ticket.issued_at + TICKET_LIFETIME
        });
        if !valid {
            return Err(Refusal::Ticket);
        }
        self.claim(binding)
    }

    /// Binds a browser signed in to Anima to the attempt with the cookie hash `binding`. Any
    /// ticket stops working: one browser holds the attempt.
    ///
    /// # Errors
    /// `Conflict` outside the phases that wait for a browser.
    pub fn claim(&mut self, binding: Hash) -> Result<(), Refusal> {
        if !self.waiting_for_browser() {
            return Err(Refusal::Conflict);
        }
        self.ticket = None;
        self.binding = Some(binding);
        self.state = None;
        self.stage = Stage::AtGithub { exchanging: false };
        Ok(())
    }

    /// Whether `cookie` is the bound browser's.
    #[must_use]
    pub fn bound(&self, cookie: Option<&str>) -> bool {
        cookie.is_some_and(|cookie| self.binding.is_some_and(|hash| hash.matches(cookie)))
    }

    /// Sets the state `s1` the form to GitHub carries, replacing an earlier one.
    ///
    /// # Errors
    /// `Unbound` for another browser; `Conflict` outside `AT_GITHUB` or during an exchange.
    pub fn send_to_github(&mut self, cookie: Option<&str>, s1: Hash) -> Result<(), Refusal> {
        if !self.bound(cookie) {
            return Err(Refusal::Unbound);
        }
        if !matches!(self.stage, Stage::AtGithub { exchanging: false }) {
            return Err(Refusal::Conflict);
        }
        self.state = Some(s1);
        Ok(())
    }

    /// A browser is back from "Create GitHub App" with `state`. A fresh match spends the state
    /// before the code is converted, so a replay converts nothing.
    ///
    /// # Errors
    /// `Unbound` for another browser; `State` for a state that does not match; `Conflict` in
    /// any other phase. Each leaves the attempt as it was.
    pub fn begin_exchange(
        &mut self,
        cookie: Option<&str>,
        state: Option<&str>,
    ) -> Result<Exchange, Refusal> {
        if !self.bound(cookie) {
            return Err(Refusal::Unbound);
        }
        match self.stage {
            Stage::AppCreated { .. } | Stage::Finishing { .. } => Ok(Exchange::Made),
            Stage::AtGithub { exchanging: false } => {
                let matched = (self.state.as_ref())
                    .zip(state)
                    .is_some_and(|(hash, state)| hash.matches(state));
                if !matched {
                    return Err(Refusal::State);
                }
                self.state = None;
                self.stage = Stage::AtGithub { exchanging: true };
                Ok(Exchange::Fresh)
            }
            _ => Err(Refusal::Conflict),
        }
    }

    /// The exchange made `app` with `key`: `APP_CREATED`, with `s2` for the install page.
    /// After a cancel or an expiry meanwhile, the App's facts are kept so the operator can
    /// delete it, and the key is dropped.
    ///
    /// # Errors
    /// `Conflict` when the attempt no longer waits for this exchange.
    pub fn created(&mut self, app: App, key: Zeroizing<String>, s2: Hash) -> Result<(), Refusal> {
        if !matches!(self.stage, Stage::AtGithub { exchanging: true }) {
            if self.ended() && self.app.is_none() {
                self.app = Some(app);
            }
            return Err(Refusal::Conflict);
        }
        self.app = Some(app);
        self.state = Some(s2);
        self.stage = Stage::AppCreated { key };
        Ok(())
    }

    /// The exchange failed with `failure`; `app` when GitHub's reply named it.
    pub fn exchange_failed(&mut self, failure: Failure, app: Option<App>, now: DateTime<Utc>) {
        if matches!(self.stage, Stage::AtGithub { exchanging: true }) {
            self.app = app;
            self.end(Stage::Failed(failure), now);
        }
    }

    /// Sets a new state `s2` for the bound browser sent to Install again.
    ///
    /// # Errors
    /// `Unbound` for another browser; `Conflict` outside `APP_CREATED`.
    pub fn send_to_install(&mut self, cookie: Option<&str>, s2: Hash) -> Result<(), Refusal> {
        if !self.bound(cookie) {
            return Err(Refusal::Unbound);
        }
        if !matches!(self.stage, Stage::AppCreated { .. }) {
            return Err(Refusal::Conflict);
        }
        self.state = Some(s2);
        Ok(())
    }

    /// Whether a hit on `/installed` comes from the bound browser: its cookie, and the state
    /// `s2` when GitHub sent one.
    #[must_use]
    pub fn back_from_install(&self, cookie: Option<&str>, state: Option<&str>) -> bool {
        let state_ok = state.is_none_or(|state| self.state.is_some_and(|hash| hash.matches(state)));
        self.bound(cookie) && state_ok
    }

    /// The App and a copy of its key, while it waits for Install.
    #[must_use]
    pub fn waiting_app(&self) -> Option<(App, Zeroizing<String>)> {
        match (&self.stage, &self.app) {
            (Stage::AppCreated { key }, Some(app)) => Some((app.clone(), key.clone())),
            _ => None,
        }
    }

    /// Takes the store at the end: `APP_CREATED` becomes `Finishing`, so only one caller
    /// stores.
    ///
    /// # Errors
    /// `Conflict` outside `APP_CREATED`, including while another caller stores.
    pub fn begin_finish(&mut self) -> Result<Finish, Refusal> {
        let (Stage::AppCreated { key }, Some(app)) = (&self.stage, &self.app) else {
            return Err(Refusal::Conflict);
        };
        let finish = Finish {
            label: (self.request.label.clone()).unwrap_or_else(|| app.owner.login.clone()),
            app_id: app.id,
            key: key.clone(),
            by: self.request.by.clone(),
        };
        let stage = std::mem::replace(&mut self.stage, Stage::Cancelled);
        if let Stage::AppCreated { key } = stage {
            self.stage = Stage::Finishing { key };
        }
        Ok(finish)
    }

    /// Ends the store taken with [`Attempt::begin_finish`].
    pub fn finished(&mut self, outcome: Finished, now: DateTime<Utc>) {
        if !matches!(self.stage, Stage::Finishing { .. }) {
            return;
        }
        match outcome {
            Finished::Connected {
                account,
                consumer_on,
            } => self.end(
                Stage::Connected {
                    account,
                    consumer_on,
                },
                now,
            ),
            Finished::Failed(failure) => self.end(Stage::Failed(failure), now),
            Finished::Again => {
                let stage = std::mem::replace(&mut self.stage, Stage::Cancelled);
                if let Stage::Finishing { key } = stage {
                    self.stage = Stage::AppCreated { key };
                }
                self.expire(now);
            }
        }
    }

    /// Cancels the attempt and drops any key it holds. An attempt that already failed or was
    /// cancelled stays as it is.
    ///
    /// # Errors
    /// `Conflict` once connected, or while the connection is being stored.
    pub fn cancel(&mut self, now: DateTime<Utc>) -> Result<(), Refusal> {
        match self.stage {
            Stage::Connected { .. } | Stage::Finishing { .. } => Err(Refusal::Conflict),
            Stage::Failed(_) | Stage::Cancelled => Ok(()),
            _ => {
                self.end(Stage::Cancelled, now);
                Ok(())
            }
        }
    }

    /// What may be shown.
    #[must_use]
    pub fn snapshot(&self) -> Snapshot {
        let (phase, failure, account, consumer_on) = match self.stage {
            Stage::WaitingForBrowser => (Phase::WaitingForBrowser, None, None, false),
            Stage::AtGithub { .. } => (Phase::AtGithub, None, None, false),
            Stage::AppCreated { .. } | Stage::Finishing { .. } => {
                (Phase::AppCreated, None, None, false)
            }
            Stage::Connected {
                account,
                consumer_on,
            } => (Phase::Connected, None, Some(account), consumer_on),
            Stage::Failed(failure) => (Phase::Failed, Some(failure), None, false),
            Stage::Cancelled => (Phase::Cancelled, None, None, false),
        };
        Snapshot {
            id: self.id,
            phase,
            started_by: self.request.by.operator.actor.kind(),
            started_at: self.started_at,
            expires_at: self.expires_at(),
            app: self.app.clone(),
            failure,
            account,
            consumer_on,
        }
    }
}
