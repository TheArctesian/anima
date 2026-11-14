//! What the API may show of an attempt: its phase, its failure, the App GitHub made and the
//! snapshot that carries them (K1 `GithubConnectAttempt`). No key, ticket, cookie or state.
#![expect(
    clippy::unused_async_trait_impl,
    reason = "the async-graphql Enum derive generates async trait impls"
)]

use async_graphql::Enum;
use chrono::{DateTime, Utc};
use source_exchange::Owner;
use uuid::Uuid;

use super::{Attempt, Stage};
use crate::{fleet::events::ActorKind, ids::SourceAccountId};

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

/// The App GitHub made: its facts from the conversion reply, text from GitHub.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct App {
    pub id: u64,
    pub slug: String,
    pub owner: Owner,
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

impl Attempt {
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
