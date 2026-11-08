//! K1 access fields: `me`, `login`, `signInBrowser`, `logout` and the API key fields
//! `apiKeys`, `createApiKey` and `revokeApiKey` (SEC-002, API-003, INT-003, LOG-005, D-3, D-4,
//! D-23).

use std::{any::Any, sync::Arc};

use async_graphql::{Context, ID, InputObject, Object, SimpleObject};
use axum::http::header::SET_COOKIE;
use chrono::{DateTime, Utc};
use sqlx::PgPool;

use super::{
    Authenticated, ClientAddr, Error, Identity, SameOrigin, TokenKind, browser,
    keys::{self, ApiKey, ApiKeyCreated, ApiKeyInput},
    login::{self, Attempt, Issue, SignedIn},
    password::Password,
    store::{self, Account},
};
use crate::{clock::Clock, credentials::Vault, events::Bus};

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
            totp_enabled: account.totp_secret_enc.is_some(),
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

/// K1 `BrowserSignIn`: what `signInBrowser` answers. The token goes only into the cookie,
/// never into the body (D-3).
#[derive(SimpleObject)]
pub struct BrowserSignIn {
    operator: OperatorProfile,
    expires_at: DateTime<Utc>,
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

    /// The operator's API keys; never their secrets (D-23). Refused for an API key.
    #[graphql(guard = "Authenticated")]
    async fn api_keys(&self, ctx: &Context<'_>) -> async_graphql::Result<Vec<ApiKey>> {
        keys::api_keys(ctx).await
    }
}

/// Merged into the root `Mutation`.
#[Object(name = "AccessMutation")]
impl Mutation {
    /// Signs the desktop app in; needs no token (API-003).
    async fn login(
        &self,
        ctx: &Context<'_>,
        input: LoginInput,
    ) -> async_graphql::Result<AuthPayload> {
        sign_in(ctx, input, Issue::LOGIN).await.map(AuthPayload)
    }

    /// Signs a browser in; needs no token (D-4). Refused before any attempt is recorded unless
    /// the request comes from the public origin (A-4). Sets the `__Host-anima` cookie; once
    /// signed in, deletes the browser token the request's cookie held, which the new cookie
    /// overwrites (SEC-002).
    async fn sign_in_browser(
        &self,
        ctx: &Context<'_>,
        input: LoginInput,
    ) -> async_graphql::Result<BrowserSignIn> {
        if ctx.data_opt::<SameOrigin>().is_none() {
            return Err(Error::OtherOrigin.into_graphql());
        }
        let issue = Issue::browser(ctx.data_opt::<Identity>());
        let signed_in = sign_in(ctx, input, issue).await?;
        ctx.append_http_header(SET_COOKIE, browser::signed_in(&signed_in.token));
        Ok(BrowserSignIn {
            operator: (&signed_in.account).into(),
            expires_at: signed_in.expires_at,
        })
    }

    /// Deletes the token of this request; a browser also gets its cookie cleared (A-8).
    #[graphql(guard = "Authenticated")]
    async fn logout(&self, ctx: &Context<'_>) -> async_graphql::Result<bool> {
        let now = need::<Arc<dyn Clock>>(ctx)?.now();
        let client = *need::<ClientAddr>(ctx)?;
        let identity = need::<Identity>(ctx)?;
        login::logout(need::<PgPool>(ctx)?, identity, client, now)
            .await
            .map_err(Error::into_graphql)?;
        if identity.kind() == TokenKind::Browser {
            ctx.append_http_header(SET_COOKIE, browser::signed_out());
        }
        Ok(true)
    }

    /// Makes an API key and shows its secret once (INT-003, D-23). Login or browser token only.
    #[graphql(guard = "Authenticated")]
    async fn create_api_key(
        &self,
        ctx: &Context<'_>,
        input: ApiKeyInput,
    ) -> async_graphql::Result<ApiKeyCreated> {
        keys::create_api_key(ctx, input).await
    }

    /// Deletes an API key; the next request with it is signed out (D-23).
    #[graphql(guard = "Authenticated")]
    async fn revoke_api_key(&self, ctx: &Context<'_>, id: ID) -> async_graphql::Result<ID> {
        keys::revoke_api_key(ctx, id).await
    }
}

/// Runs one sign-in attempt from `input` for the token `issue` names.
async fn sign_in(
    ctx: &Context<'_>,
    input: LoginInput,
    issue: Issue,
) -> async_graphql::Result<SignedIn> {
    let attempt = Attempt {
        username: input.username,
        password: Password::new(input.password),
        totp_code: input.totp_code,
        client: *need::<ClientAddr>(ctx)?,
        at: need::<Arc<dyn Clock>>(ctx)?.now(),
        issue,
    };
    let (db, bus, vault) = (need::<PgPool>(ctx)?, need::<Bus>(ctx)?, need::<Vault>(ctx)?);
    login::login(db, bus, vault, attempt)
        .await
        .map_err(Error::into_graphql)
}

/// Context data the edge or the schema must provide; its absence is a wiring fault, shown
/// as `INTERNAL` without naming the type. A hub fault carries its cause, which the API
/// boundary (`events::process_log::InternalErrors`) writes to the process log (H-5).
fn need<'a, T: Any + Send + Sync>(ctx: &Context<'a>) -> async_graphql::Result<&'a T> {
    ctx.data_opt::<T>()
        .ok_or_else(|| Error::Unavailable.into_graphql())
}
