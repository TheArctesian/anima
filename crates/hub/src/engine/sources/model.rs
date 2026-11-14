//! What a source account is made of: its stored row, the kinds of failure, and the words the
//! API shows for its health and its runs (K-E1 Sections 5 and 9).
#![expect(
    clippy::unused_async_trait_impl,
    reason = "the async-graphql Enum derive generates async trait impls"
)]

use async_graphql::Enum;
use chrono::{DateTime, Utc};

use crate::ids::{SourceAccountId, SourceSecretId};

/// One connected account, as stored, without its sync state.
#[derive(Clone, Debug, PartialEq, Eq, sqlx::FromRow)]
pub struct Account {
    pub id: SourceAccountId,
    pub source: String,
    pub label: String,
    pub instance_key: String,
    pub credential_id: Option<SourceSecretId>,
    pub last_started_at: Option<DateTime<Utc>>,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub last_error_kind: Option<String>,
    pub last_error: Option<String>,
    pub failure_count: i32,
    pub next_sync_at: DateTime<Utc>,
    pub paused_at: Option<DateTime<Utc>>,
    pub run_generation: i64,
    pub run_until: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl Account {
    /// The stored error kind, if it is one this version knows.
    #[must_use]
    pub fn error_kind(&self) -> Option<ErrorKind> {
        self.last_error_kind.as_deref().and_then(ErrorKind::parse)
    }

    /// Whether a run holds the account at `now`.
    #[must_use]
    pub fn syncing(&self, now: DateTime<Utc>) -> bool {
        self.run_until.is_some_and(|until| until > now)
    }

    /// The failure count as the pure rules read it; a negative count never happens.
    #[must_use]
    pub fn failures(&self) -> u32 {
        u32::try_from(self.failure_count).unwrap_or(0)
    }
}

/// Why the last run failed (K-E1 Section 5, `last_error_kind`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorKind {
    /// The source refused the key or the token: the operator must connect again.
    Auth,
    RateLimit,
    /// The source answered something the hub could not use.
    Provider,
    Network,
    /// The hub could not store what it read.
    Store,
}

impl ErrorKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Auth => "auth",
            Self::RateLimit => "rate_limit",
            Self::Provider => "provider",
            Self::Network => "network",
            Self::Store => "store",
        }
    }

    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        [
            Self::Auth,
            Self::RateLimit,
            Self::Provider,
            Self::Network,
            Self::Store,
        ]
        .into_iter()
        .find(|kind| kind.as_str() == text)
    }
}

/// How a source gets its token (SRC-002); only the App-key token pull is built.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
pub enum AcquisitionMethod {
    TokenPull,
}

/// The health of an account (SRC-009).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
#[graphql(name = "SourceHealth")]
pub enum Health {
    Ok,
    Starting,
    Stalled,
    NeverSynced,
    Paused,
    NeedsReconnect,
}

/// Who paused an account.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
pub enum PauseReason {
    Operator,
    ExtensionOff,
}

/// What started a run (SRC-005).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
#[graphql(name = "SyncTrigger")]
pub enum Trigger {
    Connect,
    Schedule,
    Manual,
}

impl Trigger {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Connect => "connect",
            Self::Schedule => "schedule",
            Self::Manual => "manual",
        }
    }

    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        [Self::Connect, Self::Schedule, Self::Manual]
            .into_iter()
            .find(|trigger| trigger.as_str() == text)
    }
}

/// How a run ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
#[graphql(name = "SyncOutcome")]
pub enum Outcome {
    Ok,
    Failed,
    /// The consumer was set off, the account was paused, or the run reached its request cap;
    /// the next run resumes where it stopped.
    Stopped,
}

impl Outcome {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Failed => "failed",
            Self::Stopped => "stopped",
        }
    }

    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        [Self::Ok, Self::Failed, Self::Stopped]
            .into_iter()
            .find(|outcome| outcome.as_str() == text)
    }
}
