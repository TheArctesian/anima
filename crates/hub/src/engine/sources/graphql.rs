//! The sources GraphQL part (K1; K-E1): the sources and accounts, connect, sync now, pause,
//! remove, and `sourceAccountChanged`.
//!
//! `connectGithubApp` and `removeSourceAccount` are closed to API keys by the access guard
//! (D-23). A key never reaches a log, an audit entry, an event or an error (PRJ-018).

mod changes;
mod trail;
mod views;

use std::sync::Arc;

use async_graphql::{Context, ID, InputObject, Object, Result};
use zeroize::Zeroizing;

pub use self::changes::Subscription;
use self::views::{AccountView, Seen, SourceView, accounts};
use super::{
    Error, Reach, Scheduling,
    connect::{self, Connection},
    controls,
    model::{Account, Trigger},
    run, store,
};
use crate::{
    api_errors::{FieldError, begin, commit, internal, invalid, pool, schema_dependency},
    clock::Clock,
    engine::extensions,
    fleet::{
        access::Authenticated,
        credentials::Vault,
        events::{Bus, Trail},
    },
    ids::SourceAccountId,
};

/// The source `connectGithubApp` connects.
const GITHUB: &str = "github";

/// The parts a run needs, from the schema data.
fn scheduling(ctx: &Context<'_>) -> Result<Scheduling> {
    let hub = (
        pool(ctx)?.clone(),
        schema_dependency::<Bus>(ctx)?.clone(),
        schema_dependency::<Arc<dyn Clock>>(ctx)?.clone(),
    );
    Ok(Scheduling::new(
        hub,
        schema_dependency::<Vault>(ctx)?.clone(),
        schema_dependency::<Reach>(ctx)?.clone(),
    ))
}

fn failed(error: impl Into<Error>) -> async_graphql::Error {
    error.into().into_graphql()
}

fn account_id(id: &ID) -> Result<SourceAccountId> {
    id.parse()
        .map_err(|_| invalid(&[FieldError::new("id", "That id is not valid.")]))
}

/// Starts a run of `id` in its own task and returns at once; the outcome reaches the account
/// row and `sourceAccountChanged`.
fn start(scheduling: Scheduling, id: SourceAccountId, trigger: Trigger) {
    tokio::spawn(async move {
        if let Err(error) = run::run(&scheduling, id, trigger).await {
            // Fixed words only (LOG-007).
            tracing::warn!(%error, "source sync could not be written");
        }
    });
}

/// The stored account, or `NOT_FOUND`.
async fn stored(scheduling: &Scheduling, id: SourceAccountId) -> Result<Account> {
    store::account(&scheduling.pool, id)
        .await
        .map_err(failed)?
        .ok_or_else(|| failed(Error::NotFound))
}

/// `account` as seen now, with whether a consumer of its source is on.
async fn seen(scheduling: &Scheduling, account: Account) -> Result<(AccountView, bool)> {
    let on = extensions::on_keys(&scheduling.pool)
        .await
        .map_err(failed)?;
    let seen = Seen {
        registry: &scheduling.registry,
        on: &on,
        now: scheduling.clock.now(),
    };
    let consumer_on = scheduling.registry.consumer_on(&account.source, &on);
    Ok((seen.account(account), consumer_on))
}

/// The account as seen now.
async fn view(scheduling: &Scheduling, id: SourceAccountId) -> Result<AccountView> {
    Ok(seen(scheduling, stored(scheduling, id).await?).await?.0)
}

/// A GitHub App installation to connect. The key is a secret.
#[derive(InputObject)]
#[graphql(name = "GithubAppInput")]
pub struct GithubAppInput {
    label: String,
    /// Digits only.
    app_id: String,
    /// Digits only.
    installation_id: String,
    /// SECRET. The App's private key as PEM text. Never returned, logged or put in the audit
    /// log (PRJ-018).
    private_key: String,
}

#[derive(Default)]
pub struct Query;

#[Object(name = "SourcesQuery")]
impl Query {
    #[graphql(guard = "Authenticated")]
    async fn sources(&self, ctx: &Context<'_>) -> Result<Vec<SourceView>> {
        let scheduling = scheduling(ctx)?;
        let registry = &scheduling.registry;
        let on = extensions::on_keys(&scheduling.pool)
            .await
            .map_err(failed)?;
        let seen = Seen {
            registry,
            on: &on,
            now: scheduling.clock.now(),
        };
        let mut all = Vec::new();
        for declaration in registry.declarations() {
            let consumers = registry.consumers(declaration.key);
            all.push(SourceView {
                declaration: *declaration,
                consumers: consumers.iter().map(|k| k.as_str().to_owned()).collect(),
                accounts: accounts(&scheduling.pool, &seen, Some(declaration.key))
                    .await
                    .map_err(failed)?,
            });
        }
        Ok(all)
    }

    #[graphql(guard = "Authenticated")]
    async fn source_accounts(
        &self,
        ctx: &Context<'_>,
        source_key: Option<String>,
    ) -> Result<Vec<AccountView>> {
        let scheduling = scheduling(ctx)?;
        let on = extensions::on_keys(&scheduling.pool)
            .await
            .map_err(failed)?;
        let seen = Seen {
            registry: &scheduling.registry,
            on: &on,
            now: scheduling.clock.now(),
        };
        accounts(&scheduling.pool, &seen, source_key.as_deref())
            .await
            .map_err(failed)
    }
}

#[derive(Default)]
pub struct Mutation;

#[Object(name = "SourcesMutation")]
impl Mutation {
    /// Checks the key with GitHub before it stores it (SRC-003, PRJ-018).
    #[graphql(guard = "Authenticated")]
    async fn connect_github_app(
        &self,
        ctx: &Context<'_>,
        input: GithubAppInput,
    ) -> Result<AccountView> {
        let s = scheduling(ctx)?;
        let input = Connection {
            label: input.label,
            app_id: input.app_id,
            installation_id: input.installation_id,
            private_key: Zeroizing::new(input.private_key),
        };
        let checked = connect::check(&input).map_err(failed)?;
        let declaration = *s
            .registry
            .declaration(GITHUB)
            .ok_or_else(|| failed(Error::UnknownSource))?;
        let ticket = s.transport.tokens().ticket();
        let minted = connect::prove(&s.transport, &declaration, &checked.key, s.clock.now())
            .await
            .map_err(failed)?;
        let trail = Trail::from_context(ctx)?;
        let mut tx = begin(ctx).await?;
        let (id, created) = connect::store(
            &mut tx,
            &s.vault,
            (&declaration, &checked, &input),
            trail.now,
        )
        .await
        .map_err(failed)?;
        let instance_key = checked.key.instance_key();
        let named = (
            declaration.key,
            checked.label.as_str(),
            instance_key.as_str(),
        );
        trail
            .write(&mut tx, trail::connected(id, named, created))
            .await
            .map_err(internal)?;
        commit(tx).await?;
        drop(input);
        s.feed.publish(id);
        let (shown, consumer_on) = seen(&s, stored(&s, id).await?).await?;
        // The proving token is kept only for the run that uses it now; otherwise it is
        // dropped here and the first run mints its own (PRJ-018).
        if consumer_on {
            drop(s.transport.tokens().keep(id, minted, ticket));
            start(s, id, Trigger::Connect);
        } else {
            drop(minted);
        }
        Ok(shown)
    }

    /// Starts a sync now and returns at once. `CONFLICT` while paused or while no consumer is
    /// on; a run that already holds the account is not started twice.
    #[graphql(guard = "Authenticated")]
    async fn sync_source_account(&self, ctx: &Context<'_>, id: ID) -> Result<AccountView> {
        let s = scheduling(ctx)?;
        let id = account_id(&id)?;
        let account = stored(&s, id).await?;
        let now = s.clock.now();
        let (shown, consumer_on) = seen(&s, account.clone()).await?;
        if controls::sync_now(&account, consumer_on, now).map_err(failed)? {
            start(s, id, Trigger::Manual);
        }
        Ok(shown)
    }

    #[graphql(guard = "Authenticated")]
    async fn set_source_account_paused(
        &self,
        ctx: &Context<'_>,
        id: ID,
        paused: bool,
    ) -> Result<AccountView> {
        let s = scheduling(ctx)?;
        let id = account_id(&id)?;
        let trail = Trail::from_context(ctx)?;
        let mut tx = begin(ctx).await?;
        let (before, after) = controls::set_paused(&mut tx, id, (paused, trail.now))
            .await
            .map_err(failed)?;
        if before.paused_at.is_some() == paused {
            drop(trail);
        } else {
            trail
                .write(&mut tx, trail::paused(&after, paused))
                .await
                .map_err(internal)?;
        }
        commit(tx).await?;
        s.feed.publish(id);
        view(&s, id).await
    }

    /// Deletes the account and its stored key; `deleteRecords` false keeps what it imported
    /// (ITG-004). Returns the id.
    #[graphql(guard = "Authenticated")]
    async fn remove_source_account(
        &self,
        ctx: &Context<'_>,
        id: ID,
        delete_records: bool,
    ) -> Result<ID> {
        let s = scheduling(ctx)?;
        let account = account_id(&id)?;
        let trail = Trail::from_context(ctx)?;
        let mut tx = begin(ctx).await?;
        let (removed, deleted) = controls::remove(&mut tx, &s.registry, account, delete_records)
            .await
            .map_err(failed)?;
        trail
            .write(&mut tx, trail::removed(&removed, delete_records, deleted))
            .await
            .map_err(internal)?;
        commit(tx).await?;
        s.transport.tokens().forget(account);
        s.feed.publish(account);
        Ok(id)
    }
}
