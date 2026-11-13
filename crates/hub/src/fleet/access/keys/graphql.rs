//! The K1 API key types and the work behind `apiKeys`, `createApiKey` and `revokeApiKey`. The
//! root fields themselves are in `access::graphql`. Only a login or browser identity may use
//! them: the backstop refuses an API key before any resolver, and each resolver asks again
//! (D-23, WP-43 A-6).
#![expect(
    clippy::unused_async_trait_impl,
    reason = "the async-graphql Enum derive generates async trait impls"
)]

use async_graphql::{
    Context, Enum, Error as GraphQlError, ErrorExtensions as _, ID, InputObject, Object, Result,
    value,
};
use chrono::{DateTime, Utc};

use super::{ApiKey, Created, Error, Wanted, create, list, revoke};
use crate::{
    api_errors::{ErrorCode, FieldError, begin, coded, commit, internal, invalid, pool},
    fleet::{
        access::{Error as AccessError, MadeFor, Scope, guard::operator_only, login},
        events::Trail,
    },
    ids::ApiKeyId,
};

/// K1 `TokenScope`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
pub enum TokenScope {
    Read,
    Write,
}

impl From<TokenScope> for Scope {
    fn from(scope: TokenScope) -> Self {
        match scope {
            TokenScope::Read => Self::Read,
            TokenScope::Write => Self::Write,
        }
    }
}

/// The K1 scope of an API key: `read` or `write`, never `builder`. The builder scope has no
/// `TokenScope` and is never shown as `write` (D-23, R-107); `ApiKey::from_row` refuses it too.
fn token_scope(scope: Scope) -> Result<TokenScope> {
    match scope {
        Scope::Read => Ok(TokenScope::Read),
        Scope::Write => Ok(TokenScope::Write),
        Scope::Builder => Err(to_graphql(Error::Unreadable)),
    }
}

#[Object]
impl ApiKey {
    async fn id(&self) -> ID {
        ID(self.id.to_string())
    }

    /// 1..64 characters, unique.
    async fn name(&self) -> &str {
        &self.name
    }

    #[expect(
        clippy::unused_async,
        reason = "async-graphql resolvers must be async; this one can fail"
    )]
    async fn scope(&self) -> Result<TokenScope> {
        token_scope(self.scope)
    }

    /// Who the key is for: a script the operator runs, or an AI agent. It never changes, and
    /// the hub records each request with an agent key as Agent (LOG-015, LOG-017).
    async fn made_for(&self) -> MadeFor {
        self.made_for
    }

    /// The first 12 characters of the secret, so the operator can tell keys apart.
    async fn prefix(&self) -> &str {
        &self.prefix
    }

    async fn expires_at(&self) -> DateTime<Utc> {
        self.expires_at
    }

    async fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    async fn last_used_at(&self) -> Option<DateTime<Utc>> {
        self.last_used_at
    }
}

/// K1 `ApiKeyCreated`. No `Debug`: it holds the secret.
pub struct ApiKeyCreated(Created);

#[Object]
impl ApiKeyCreated {
    async fn key(&self) -> &ApiKey {
        &self.0.key
    }

    /// Shown once. The hub keeps only its SHA-256 (INT-003, P3-004).
    async fn secret(&self) -> &str {
        &self.0.secret
    }
}

/// K1 `ApiKeyInput`.
#[derive(InputObject)]
pub struct ApiKeyInput {
    name: String,
    scope: TokenScope,
    /// Required; it cannot change later (LOG-015).
    made_for: MadeFor,
    /// Required; at most one year from now (D-23).
    expires_at: DateTime<Utc>,
}

/// `apiKeys`: the signed-in operator's keys.
pub(in crate::fleet::access) async fn api_keys(ctx: &Context<'_>) -> Result<Vec<ApiKey>> {
    let identity = operator_only(ctx, "apiKeys")?;
    let operator = identity
        .operator()
        .ok_or_else(|| AccessError::Unauthenticated.into_graphql())?;
    list(pool(ctx)?, operator.id).await.map_err(to_graphql)
}

/// `createApiKey`: stores the key with its audit entry and event, and shows its secret once.
pub(in crate::fleet::access) async fn create_api_key(
    ctx: &Context<'_>,
    input: ApiKeyInput,
) -> Result<ApiKeyCreated> {
    operator_only(ctx, "createApiKey")?;
    let wanted = Wanted {
        name: input.name,
        scope: input.scope.into(),
        made_for: input.made_for,
        expires_at: input.expires_at,
    };
    let trail = Trail::from_context(ctx)?;
    let bytes = login::random().map_err(AccessError::into_graphql)?;
    let mut tx = begin(ctx).await?;
    let created = create(&mut tx, trail, &wanted, bytes)
        .await
        .map_err(to_graphql)?;
    commit(tx).await?;
    Ok(ApiKeyCreated(created))
}

/// `revokeApiKey`: deletes the key with its audit entry and event; it stops at once.
pub(in crate::fleet::access) async fn revoke_api_key(ctx: &Context<'_>, id: ID) -> Result<ID> {
    operator_only(ctx, "revokeApiKey")?;
    let key: ApiKeyId = id
        .parse()
        .map_err(|_| invalid(&[FieldError::new("id", "That id is not valid.")]))?;
    let trail = Trail::from_context(ctx)?;
    let mut tx = begin(ctx).await?;
    revoke(&mut tx, trail, key).await.map_err(to_graphql)?;
    commit(tx).await?;
    Ok(id)
}

/// The GraphQL error with its `extensions.code` (K7). A taken name is a `CONFLICT` that names
/// the field; a hub failure shows no detail.
fn to_graphql(error: Error) -> GraphQlError {
    match error {
        Error::Invalid(fields) => invalid(&fields),
        Error::NameTaken => {
            let message = error.to_string();
            coded(message.clone(), ErrorCode::Conflict).extend_with(|_, extensions| {
                extensions.set("fields", value!([{ "field": "name", "message": message }]));
            })
        }
        Error::NotFound => coded(error.to_string(), ErrorCode::NotFound),
        Error::Unreadable | Error::Database(_) | Error::Trail(_) => internal(error),
    }
}
