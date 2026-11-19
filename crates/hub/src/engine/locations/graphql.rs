//! The locations GraphQL part (K1; K-E4 Section 13): `Query.locationSamples`,
//! `Query.locationIngestTokens`, `Query.transferContext`, `Mutation.recordLocationSamples`,
//! `Mutation.createLocationIngestToken` and `Mutation.revokeLocationIngestToken`.
//!
//! A change of an ingest token writes its row, one audit entry and one event in one
//! transaction. A stored sample writes no event (R-208). A read of samples writes one audit
//! entry with the span and the count. No entry holds a position.

mod views;

use std::sync::Arc;

use async_graphql::{
    Context, Error as GraphQlError, ErrorExtensions as _, ID, Object, Result, Value,
};
use chrono::{DateTime, Utc};

use views::{IssuedTokenView, LocationSampleInput, SampleView, TokenView, TransferContextView};

use super::{
    Bad, Error, MAX_BATCH, MAX_READ, NewSample, Origin, Provider, Seal, create_token, revoke_token,
    samples, store_samples, tokens, trail, transfer_context,
};
use crate::{
    api_errors::{ErrorCode, begin, coded, commit, internal, pool, schema_dependency},
    clock::Clock,
    fleet::{
        access::{Authenticated, operator_only},
        events::{self, AuditAction, AuditTarget, ClientAddr, NewAudit, Operator, Trail},
    },
    ids::{EventId, IngestTokenId},
};

/// `VALIDATION` with each field to fix.
fn invalid_fields(bad: &[Bad]) -> GraphQlError {
    let listed: Vec<Value> = bad
        .iter()
        .map(|bad| async_graphql::value!({ "field": bad.field.clone(), "message": bad.message }))
        .collect();
    GraphQlError::new("Some input is not valid.").extend_with(|_, extensions| {
        extensions.set("code", ErrorCode::Validation.as_str());
        extensions.set("fields", Value::List(listed));
    })
}

/// The K7 code of each refusal; a fault of the hub is `INTERNAL` with no detail (API-010).
fn refused(error: Error) -> GraphQlError {
    match error {
        Error::Invalid(bad) => invalid_fields(&bad),
        Error::NotFound => coded("No ingest token has this ID.", ErrorCode::NotFound),
        Error::Key(crate::engine::sealing_keys::SealingKeyError::Unavailable) => coded(
            "This hub has no sealing keys for positions: set ANIMA_DATA_KEY_DIR.",
            ErrorCode::Unavailable,
        ),
        other => internal(other),
    }
}

#[derive(Default)]
pub struct Query;

#[Object(name = "LocationsQuery")]
impl Query {
    /// The location samples observed from `from` to `to`, oldest first, at most `first` (1 to
    /// 1000). A span is at most 31 days. Each read writes one audit entry with the span and
    /// the count.
    #[graphql(guard = "Authenticated")]
    #[expect(
        clippy::too_many_arguments,
        reason = "K1 fixes the argument list of Query.locationSamples"
    )]
    async fn location_samples(
        &self,
        ctx: &Context<'_>,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        first: Option<i32>,
    ) -> Result<Vec<SampleView>> {
        let seal = schema_dependency::<Seal>(ctx)?;
        let first = first.map_or(MAX_READ, i64::from);
        let found = samples(pool(ctx)?, seal, (from, to), first)
            .await
            .map_err(refused)?;
        let entry = NewAudit {
            operator: schema_dependency::<Operator>(ctx)?.clone(),
            action: AuditAction::Read,
            target: AuditTarget::LocationSamples,
            client: *schema_dependency::<ClientAddr>(ctx)?,
            change: trail::read_change((from, to), found.len()),
        };
        let now = schema_dependency::<Arc<dyn Clock>>(ctx)?.now();
        let mut tx = begin(ctx).await?;
        events::audit(&mut tx, now, entry).await.map_err(internal)?;
        commit(tx).await?;
        Ok(found.into_iter().map(Into::into).collect())
    }

    /// The ingest tokens of the OwnTracks path, newest first, without their values.
    #[graphql(guard = "Authenticated")]
    async fn location_ingest_tokens(&self, ctx: &Context<'_>) -> Result<Vec<TokenView>> {
        let found = tokens(pool(ctx)?).await.map_err(refused)?;
        Ok(found.into_iter().map(Into::into).collect())
    }

    /// The context of the transfer event `event`: the stored roles, and the evidence for the
    /// position of the operator, computed at the read. Null when the event is not a transfer.
    #[graphql(guard = "Authenticated")]
    async fn transfer_context(
        &self,
        ctx: &Context<'_>,
        event: ID,
    ) -> Result<Option<TransferContextView>> {
        let Ok(event) = event.parse::<EventId>() else {
            return Ok(None);
        };
        let seal = schema_dependency::<Seal>(ctx)?;
        let found = transfer_context(pool(ctx)?, seal, event)
            .await
            .map_err(refused)?;
        Ok(found.map(Into::into))
    }
}

#[derive(Default)]
pub struct Mutation;

#[Object(name = "LocationsMutation")]
impl Mutation {
    /// Stores samples of an Anima phone app, at most 500 in one call, and returns how many
    /// were new. The same sample again is not stored twice. A batch that stored a sample
    /// writes one audit entry and no event.
    #[graphql(guard = "Authenticated")]
    async fn record_location_samples(
        &self,
        ctx: &Context<'_>,
        samples: Vec<LocationSampleInput>,
    ) -> Result<i32> {
        if samples.len() > MAX_BATCH {
            let bad = Bad::new("samples", "Give at most 500 samples in one call.");
            return Err(invalid_fields(&[bad]));
        }
        let given: Vec<NewSample> = samples.into_iter().map(Into::into).collect();
        let seal = schema_dependency::<Seal>(ctx)?;
        let operator = schema_dependency::<Operator>(ctx)?;
        let origin = Origin {
            provider: Provider::Anima,
            account: operator.id.to_string(),
        };
        let at = schema_dependency::<Arc<dyn Clock>>(ctx)?.now();
        let mut tx = begin(ctx).await?;
        let stored = store_samples(&mut tx, seal, (&origin, &given), at)
            .await
            .map_err(refused)?;
        if stored.added > 0 {
            // One audit entry for the batch, and no event: a stored fix writes none (R-208).
            let entry = NewAudit {
                operator: operator.clone(),
                action: AuditAction::Create,
                target: AuditTarget::LocationSamples,
                client: *schema_dependency::<ClientAddr>(ctx)?,
                change: trail::recorded_change(&origin, &operator.name, stored),
            };
            events::audit(&mut tx, at, entry).await.map_err(internal)?;
        }
        commit(tx).await?;
        Ok(i32::try_from(stored.added).unwrap_or(i32::MAX))
    }

    /// Makes an ingest token for the OwnTracks path and shows its value this one time. An API
    /// key cannot make one.
    #[graphql(guard = "Authenticated")]
    async fn create_location_ingest_token(
        &self,
        ctx: &Context<'_>,
        name: String,
    ) -> Result<IssuedTokenView> {
        operator_only(ctx, "createLocationIngestToken")?;
        let change = Trail::from_context(ctx)?;
        let mut tx = begin(ctx).await?;
        let issued = create_token(&mut tx, &name, change.now)
            .await
            .map_err(refused)?;
        change
            .write(&mut tx, trail::token_created(&issued.token))
            .await
            .map_err(internal)?;
        commit(tx).await?;
        Ok(issued.into())
    }

    /// Revokes an ingest token: the OwnTracks path refuses it from now on. `NOT_FOUND` when
    /// no token has this ID.
    #[graphql(guard = "Authenticated")]
    async fn revoke_location_ingest_token(&self, ctx: &Context<'_>, id: ID) -> Result<TokenView> {
        let id: IngestTokenId = id.parse().map_err(|_| refused(Error::NotFound))?;
        let change = Trail::from_context(ctx)?;
        let mut tx = begin(ctx).await?;
        let token = revoke_token(&mut tx, id, change.now)
            .await
            .map_err(refused)?;
        change
            .write(&mut tx, trail::token_revoked(&token))
            .await
            .map_err(internal)?;
        commit(tx).await?;
        Ok(token.into())
    }
}
