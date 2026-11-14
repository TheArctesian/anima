//! The sources' GraphQL types: `Source`, `SourceAccount`, `SyncRun`, `SyncProblem` (K1).
//! An account shows that a key is stored, never the key (KEY-002, PRJ-018).

use std::collections::HashSet;

use async_graphql::{Context, ID, Object, Result, SimpleObject};
use chrono::{DateTime, TimeDelta, Utc};
use source_exchange::Declaration;

use crate::{
    api_errors::{FieldError, invalid, pool},
    engine::{
        extensions::Key,
        sources::{
            Error,
            health::{HealthFacts, health, paused_by},
            intake::Registry,
            model::{Account, AcquisitionMethod, Health, Outcome, PauseReason, Trigger},
            schedule,
            store::{self, runs},
        },
    },
};

/// What the views of one request share: the switches that are on and the time.
pub struct Seen<'a> {
    pub registry: &'a Registry,
    pub on: &'a HashSet<Key>,
    pub now: DateTime<Utc>,
}

impl Seen<'_> {
    /// The view of `account` as seen now.
    pub fn account(&self, account: Account) -> AccountView {
        let interval = schedule::interval(
            self.registry
                .declaration(&account.source)
                .map_or(0, |d| d.poll_interval_seconds),
        );
        let consumer_on = self.registry.consumer_on(&account.source, self.on);
        AccountView {
            account,
            consumer_on,
            interval,
            now: self.now,
        }
    }
}

/// One source the hub knows (SRC-001).
pub struct SourceView {
    pub declaration: Declaration,
    pub consumers: Vec<String>,
    pub accounts: Vec<AccountView>,
}

#[Object(name = "Source")]
impl SourceView {
    /// Stable identifier, for example github.
    async fn key(&self) -> &str {
        self.declaration.key
    }

    async fn name(&self) -> &str {
        self.declaration.name
    }

    async fn acquisition(&self) -> AcquisitionMethod {
        AcquisitionMethod::TokenPull
    }

    async fn scopes(&self) -> Vec<&str> {
        self.declaration.scopes.to_vec()
    }

    /// Null: the source asks for read-only scopes. Otherwise why it does not (SRC-010).
    async fn write_reason(&self) -> Option<&str> {
        self.declaration.write_reason
    }

    async fn gives(&self) -> Vec<&str> {
        self.declaration.gives.to_vec()
    }

    async fn poll_interval_seconds(&self) -> i32 {
        let seconds = schedule::interval(self.declaration.poll_interval_seconds).num_seconds();
        i32::try_from(seconds).unwrap_or(i32::MAX)
    }

    /// Keys of the extensions that use it.
    async fn consumers(&self) -> &[String] {
        &self.consumers
    }

    async fn accounts(&self) -> &[AccountView] {
        &self.accounts
    }
}

/// One connected account, as seen at one time.
pub struct AccountView {
    account: Account,
    consumer_on: bool,
    interval: TimeDelta,
    now: DateTime<Utc>,
}

impl AccountView {
    fn facts(&self) -> HealthFacts {
        HealthFacts {
            paused: self.account.paused_at.is_some(),
            consumer_on: self.consumer_on,
            last_error_kind: self.account.error_kind(),
            last_started_at: self.account.last_started_at,
            last_sync_at: self.account.last_sync_at,
            failure_count: self.account.failures(),
            interval: self.interval,
        }
    }
}

#[Object(name = "SourceAccount")]
impl AccountView {
    async fn id(&self) -> ID {
        ID(self.account.id.to_string())
    }

    async fn source_key(&self) -> &str {
        &self.account.source
    }

    async fn label(&self) -> &str {
        &self.account.label
    }

    /// For GitHub: <App ID>/<installation ID>.
    async fn instance_key(&self) -> &str {
        &self.account.instance_key
    }

    /// Never the value (KEY-002).
    async fn credential_stored(&self) -> bool {
        self.account.credential_id.is_some()
    }

    async fn health(&self) -> Health {
        health(&self.facts(), self.now)
    }

    /// Set when health is PAUSED.
    async fn paused_by(&self) -> Option<PauseReason> {
        paused_by(&self.facts())
    }

    /// The last successful sync.
    async fn last_sync_at(&self) -> Option<DateTime<Utc>> {
        self.account.last_sync_at
    }

    async fn last_error(&self) -> Option<&str> {
        self.account.last_error.as_deref()
    }

    async fn failure_count(&self) -> i32 {
        self.account.failure_count
    }

    /// Null while paused.
    async fn next_sync_at(&self) -> Option<DateTime<Utc>> {
        paused_by(&self.facts())
            .is_none()
            .then_some(self.account.next_sync_at)
    }

    async fn syncing(&self) -> bool {
        self.account.syncing(self.now)
    }

    /// Newest first, 1 to 20 (ITG-003).
    async fn runs(
        &self,
        ctx: &Context<'_>,
        #[graphql(default_with = "Some(20)")] first: Option<i32>,
    ) -> Result<Vec<RunView>> {
        let first = first.unwrap_or(20);
        if !(1..=20).contains(&first) {
            return Err(invalid(&[FieldError::new(
                "first",
                "Ask for 1 to 20 runs.",
            )]));
        }
        let rows = runs::newest(pool(ctx)?, self.account.id, i64::from(first))
            .await
            .map_err(|e| Error::from(e).into_graphql())?;
        Ok(rows.into_iter().map(RunView::of).collect())
    }

    async fn created_at(&self) -> DateTime<Utc> {
        self.account.created_at
    }
}

/// Something wrong a run saw.
#[derive(SimpleObject)]
#[graphql(name = "SyncProblem")]
pub struct ProblemView {
    code: String,
    subject: String,
    message: String,
}

/// One sync run (SRC-005).
#[derive(SimpleObject)]
#[graphql(name = "SyncRun")]
pub struct RunView {
    id: ID,
    trigger: Trigger,
    started_at: DateTime<Utc>,
    ended_at: Option<DateTime<Utc>>,
    /// Null while it runs.
    outcome: Option<Outcome>,
    inserted: i32,
    changed: i32,
    unchanged: i32,
    removed: i32,
    problems: Vec<ProblemView>,
    /// Fixed words. Never a token, a key or a provider reply (LOG-007).
    error: Option<String>,
}

impl RunView {
    fn of(run: runs::Run) -> Self {
        Self {
            id: ID(run.id.to_string()),
            trigger: Trigger::parse(&run.trigger).unwrap_or(Trigger::Schedule),
            started_at: run.started_at,
            ended_at: run.ended_at,
            outcome: run.outcome.as_deref().and_then(Outcome::parse),
            inserted: run.inserted,
            changed: run.changed,
            unchanged: run.unchanged,
            removed: run.removed,
            problems: run
                .problems
                .0
                .into_iter()
                .map(|p| ProblemView {
                    code: p.code,
                    subject: p.subject,
                    message: p.message,
                })
                .collect(),
            error: run.error,
        }
    }
}

/// Every account of `source` (all when `None`), seen now.
///
/// # Errors
/// `Database` when the read fails.
pub async fn accounts(
    db: &sqlx::PgPool,
    seen: &Seen<'_>,
    source: Option<&str>,
) -> Result<Vec<AccountView>, Error> {
    let rows = store::accounts(db, source).await?;
    Ok(rows.into_iter().map(|a| seen.account(a)).collect())
}
