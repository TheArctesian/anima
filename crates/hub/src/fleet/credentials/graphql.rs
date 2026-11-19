//! The credentials GraphQL part: the operational keys (K1; K-E5 Section 2.2, R-147).
//!
//! The queries give the metadata, the versions, the due state and the consumers; no field holds
//! a value. Each change needs a login or browser token (D-23): the backstop refuses an API key
//! before any resolver, and each resolver asks again with `operator_only`. A grant is a trust
//! decision. Each change writes its rows, one audit entry and one event in one transaction.

mod views;

use std::sync::Arc;

use async_graphql::{Context, Error as GraphQlError, ID, Object, Result};

use super::{
    Secret, Vault,
    operational::{self, Error, Grant, NewKey},
};
use crate::{
    api_errors::{
        ErrorCode, FieldError, begin, coded, commit, internal, invalid, pool, schema_dependency,
    },
    clock::Clock,
    fleet::{
        access::{Authenticated, operator_only},
        events::Trail,
    },
    ids::DeviceId,
};
use views::{CreateInput, EndingView, KeyView, UpdateInput};

const NO_KEY: &str = "No operational key has this name.";

/// `VALIDATION`, `CONFLICT` and `NOT_FOUND` for what the operator can fix; anything else is
/// `INTERNAL` (API-010). No message holds a value.
fn refused(error: Error) -> GraphQlError {
    match error {
        Error::Invalid(problems) => invalid(
            &problems
                .iter()
                .map(|problem| FieldError::new(problem.field, problem.message))
                .collect::<Vec<_>>(),
        ),
        Error::NameTaken => coded(
            "An operational key has this name already. A removed key keeps its name.",
            ErrorCode::Conflict,
        ),
        Error::UnknownKey => coded(NO_KEY, ErrorCode::NotFound),
        Error::UnknownDevice => coded("No device has this ID.", ErrorCode::NotFound),
        Error::NoGrant => coded(
            "The key is not granted to this device for this workload.",
            ErrorCode::NotFound,
        ),
        Error::Expired => coded(
            "This key passed its expiry. Set a later expiry, then give the new value.",
            ErrorCode::Conflict,
        ),
        Error::NoActiveVersion => coded(
            "This key has no active version to revoke.",
            ErrorCode::Conflict,
        ),
        Error::Database(_) | Error::Trail(_) | Error::Sealing(_) => internal(error),
    }
}

fn device_of(id: &ID) -> Result<DeviceId> {
    id.parse()
        .map_err(|_| coded("No device has this ID.", ErrorCode::NotFound))
}

#[derive(Default)]
pub struct Query;

#[Object(name = "CredentialsQuery")]
impl Query {
    /// Each key that is not removed, by name. No field holds a value.
    #[graphql(guard = "Authenticated")]
    async fn operational_keys(&self, ctx: &Context<'_>) -> Result<Vec<KeyView>> {
        let now = schema_dependency::<Arc<dyn Clock>>(ctx)?.now();
        let keys = operational::keys(pool(ctx)?, now).await.map_err(refused)?;
        Ok(keys.into_iter().map(Into::into).collect())
    }

    /// The key named `name`; null when no key that is not removed has this name.
    #[graphql(guard = "Authenticated")]
    async fn operational_key(&self, ctx: &Context<'_>, name: String) -> Result<Option<KeyView>> {
        let now = schema_dependency::<Arc<dyn Clock>>(ctx)?.now();
        let key = operational::key(pool(ctx)?, &name, now)
            .await
            .map_err(refused)?;
        Ok(key.map(Into::into))
    }
}

#[derive(Default)]
pub struct Mutation;

#[Object(name = "CredentialsMutation")]
impl Mutation {
    /// Makes a key with its first value. Login or browser token only.
    #[graphql(guard = "Authenticated")]
    async fn create_operational_key(
        &self,
        ctx: &Context<'_>,
        mut input: CreateInput,
    ) -> Result<KeyView> {
        operator_only(ctx, "createOperationalKey")?;
        let new = NewKey {
            metadata: input.metadata(),
            value: Secret::from(std::mem::take(&mut input.value)),
            name: input.name,
        };
        let (vault, trail) = (schema_dependency::<Vault>(ctx)?, Trail::from_context(ctx)?);
        let mut tx = begin(ctx).await?;
        let key = operational::create(&mut tx, vault, new, trail)
            .await
            .map_err(refused)?;
        commit(tx).await?;
        Ok(key.into())
    }

    /// Replaces the metadata of the key. Login or browser token only.
    #[graphql(guard = "Authenticated")]
    async fn update_operational_key(
        &self,
        ctx: &Context<'_>,
        name: String,
        input: UpdateInput,
    ) -> Result<KeyView> {
        operator_only(ctx, "updateOperationalKey")?;
        let trail = Trail::from_context(ctx)?;
        let mut tx = begin(ctx).await?;
        let key = operational::update(&mut tx, &name, input.into(), trail)
            .await
            .map_err(refused)?;
        commit(tx).await?;
        Ok(key.into())
    }

    /// Ends the active version and the retrieval of the key. Login or browser token only.
    #[graphql(guard = "Authenticated")]
    async fn remove_operational_key(&self, ctx: &Context<'_>, name: String) -> Result<EndingView> {
        operator_only(ctx, "removeOperationalKey")?;
        let trail = Trail::from_context(ctx)?;
        let mut tx = begin(ctx).await?;
        let ending = operational::remove(&mut tx, &name, trail)
            .await
            .map_err(refused)?;
        commit(tx).await?;
        Ok(ending.into())
    }

    /// Makes `value` the active version and supersedes the old one at once. Login or browser
    /// token only.
    #[graphql(guard = "Authenticated")]
    #[expect(
        clippy::too_many_arguments,
        reason = "K1 fixes the argument list of Mutation.rotateOperationalKey"
    )]
    async fn rotate_operational_key(
        &self,
        ctx: &Context<'_>,
        name: String,
        value: String,
        rotated_at_provider: bool,
    ) -> Result<EndingView> {
        operator_only(ctx, "rotateOperationalKey")?;
        let value = Secret::from(value);
        let (vault, trail) = (schema_dependency::<Vault>(ctx)?, Trail::from_context(ctx)?);
        let mut tx = begin(ctx).await?;
        let asked = (name.as_str(), value, rotated_at_provider);
        let ending = operational::rotate(&mut tx, vault, asked, trail)
            .await
            .map_err(refused)?;
        commit(tx).await?;
        Ok(ending.into())
    }

    /// Ends the active version with no new one. Login or browser token only.
    #[graphql(guard = "Authenticated")]
    async fn revoke_operational_key(&self, ctx: &Context<'_>, name: String) -> Result<EndingView> {
        operator_only(ctx, "revokeOperationalKey")?;
        let trail = Trail::from_context(ctx)?;
        let mut tx = begin(ctx).await?;
        let ending = operational::revoke(&mut tx, &name, trail)
            .await
            .map_err(refused)?;
        commit(tx).await?;
        Ok(ending.into())
    }

    /// Lets the daemon of `device` fetch the key for `workload`: a trust decision. Login or
    /// browser token only.
    #[graphql(guard = "Authenticated")]
    #[expect(
        clippy::too_many_arguments,
        reason = "K1 fixes the argument list of Mutation.grantOperationalKey"
    )]
    async fn grant_operational_key(
        &self,
        ctx: &Context<'_>,
        name: String,
        device: ID,
        workload: String,
    ) -> Result<KeyView> {
        operator_only(ctx, "grantOperationalKey")?;
        let grant = Grant {
            name,
            device: device_of(&device)?,
            workload,
        };
        let trail = Trail::from_context(ctx)?;
        let mut tx = begin(ctx).await?;
        let key = operational::grant(&mut tx, grant, trail)
            .await
            .map_err(refused)?;
        commit(tx).await?;
        Ok(key.into())
    }

    /// Ends a grant. Login or browser token only.
    #[graphql(guard = "Authenticated")]
    #[expect(
        clippy::too_many_arguments,
        reason = "K1 fixes the argument list of Mutation.ungrantOperationalKey"
    )]
    async fn ungrant_operational_key(
        &self,
        ctx: &Context<'_>,
        name: String,
        device: ID,
        workload: String,
    ) -> Result<KeyView> {
        operator_only(ctx, "ungrantOperationalKey")?;
        let grant = Grant {
            name,
            device: device_of(&device)?,
            workload,
        };
        let trail = Trail::from_context(ctx)?;
        let mut tx = begin(ctx).await?;
        let key = operational::ungrant(&mut tx, grant, trail)
            .await
            .map_err(refused)?;
        commit(tx).await?;
        Ok(key.into())
    }
}
