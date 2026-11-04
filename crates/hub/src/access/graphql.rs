//! K1 access fields: `me`, `login` and `logout` (SEC-002, API-003, LOG-005).

use std::{any::Any, sync::Arc};

use async_graphql::{Context, ID, InputObject, Object, SimpleObject};
use chrono::{DateTime, Utc};
use sqlx::PgPool;

use super::{
    Authenticated, ClientAddr, Error, Identity,
    login::{self, Attempt, SignedIn},
    password::Password,
    store::{self, Account},
};
use crate::{clock::Clock, events::Bus};

#[derive(Default)]
pub struct Query;

#[derive(Default)]
pub struct Mutation;

/// K1 `Operator`: the signed-in account as the client sees it. Named apart from the Rust
/// `access::Operator` that other domains read from the request.
#[derive(SimpleObject)]
#[graphql(name = "Operator")]
pub struct OperatorProfile {
    id: ID,
    username: String,
    totp_enabled: bool,
}

impl From<&Account> for OperatorProfile {
    fn from(account: &Account) -> Self {
        Self {
            id: ID(account.id.to_string()),
            username: account.username.clone(),
            totp_enabled: account.totp_enabled,
        }
    }
}

/// K1 `AuthPayload`. No `Debug`: it holds the token.
pub struct AuthPayload(SignedIn);

#[Object]
impl AuthPayload {
    /// Bearer token. The client keeps it in the OS key store (SEC-002).
    async fn token(&self) -> &str {
        self.0.token.expose()
    }

    async fn expires_at(&self) -> DateTime<Utc> {
        self.0.expires_at
    }

    async fn operator(&self) -> OperatorProfile {
        (&self.0.account).into()
    }
}

/// K1 `LoginInput`. No `Debug`: it holds the password.
#[derive(InputObject)]
pub struct LoginInput {
    username: String,
    password: String,
    totp_code: Option<String>,
}

/// Merged into the root `Query`; the name only keeps it apart from the other parts.
#[Object(name = "AccessQuery")]
impl Query {
    #[graphql(guard = "Authenticated")]
    async fn me(&self, ctx: &Context<'_>) -> async_graphql::Result<OperatorProfile> {
        let identity = need::<Identity>(ctx)?;
        let account = store::account_by_id(need::<PgPool>(ctx)?, identity.operator().id)
            .await
            .map_err(|error| Error::from(error).into_graphql())?
            .ok_or_else(|| Error::Unauthenticated.into_graphql())?;
        Ok((&account).into())
    }
}

/// Merged into the root `Mutation`.
#[Object(name = "AccessMutation")]
impl Mutation {
    /// The only operation that needs no token (API-003).
    async fn login(
        &self,
        ctx: &Context<'_>,
        input: LoginInput,
    ) -> async_graphql::Result<AuthPayload> {
        let attempt = Attempt {
            username: input.username,
            password: Password::new(input.password),
            totp_code: input.totp_code,
            client: *need::<ClientAddr>(ctx)?,
            at: need::<Arc<dyn Clock>>(ctx)?.now(),
        };
        login::login(need::<PgPool>(ctx)?, need::<Bus>(ctx)?, attempt)
            .await
            .map(AuthPayload)
            .map_err(Error::into_graphql)
    }

    /// Deletes the token of this request.
    #[graphql(guard = "Authenticated")]
    async fn logout(&self, ctx: &Context<'_>) -> async_graphql::Result<bool> {
        let now = need::<Arc<dyn Clock>>(ctx)?.now();
        let client = *need::<ClientAddr>(ctx)?;
        login::logout(need::<PgPool>(ctx)?, need::<Identity>(ctx)?, client, now)
            .await
            .map_err(Error::into_graphql)?;
        Ok(true)
    }
}

/// Context data the edge or the schema must provide; its absence is a wiring fault, shown
/// as `INTERNAL` without naming the type. A hub fault carries its cause, which the API
/// boundary (`events::process_log::InternalErrors`) writes to the process log (H-5).
fn need<'a, T: Any + Send + Sync>(ctx: &Context<'a>) -> async_graphql::Result<&'a T> {
    ctx.data_opt::<T>()
        .ok_or_else(|| Error::Unavailable.into_graphql())
}
