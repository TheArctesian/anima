//! Who is making a request: the bearer token resolved once at the HTTP or WebSocket edge into
//! an [`Identity`] that only `access` can build (D-6, API-003).

use std::net::SocketAddr;

use async_graphql::{Data, Request};
use axum::{
    extract::ConnectInfo,
    http::{Extensions, HeaderMap, header::AUTHORIZATION},
};
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::{
    ClientAddr, Error, Operator, store,
    tokens::{self, Scope},
};

/// A signed-in request: the operator and the token it presented.
///
/// Fields are private and there is no public constructor, so request data from anywhere
/// else (for example a schema-level `.data(..)`) cannot pass the guard.
pub struct Identity {
    operator: Operator,
    token: Uuid,
    scope: Scope,
    expires_at: DateTime<Utc>,
}

impl Identity {
    pub(super) const fn new(
        operator: Operator,
        token: Uuid,
        scope: Scope,
        expires_at: DateTime<Utc>,
    ) -> Self {
        Self {
            operator,
            token,
            scope,
            expires_at,
        }
    }

    /// A write identity for `operator` that holds until `expires_at`, for other domains' tests.
    /// It exists only with the `test-support` feature, which only the hub's own dev-dependency
    /// turns on: it is in `cargo test` builds, not in a `cargo build --release --bin
    /// anima-hub`, so the shipped hub cannot make an identity without a token (D-6).
    #[cfg(feature = "test-support")]
    #[must_use]
    pub const fn for_tests(operator: Operator, expires_at: DateTime<Utc>) -> Self {
        Self::new(operator, Uuid::nil(), Scope::Write, expires_at)
    }

    #[must_use]
    pub const fn operator(&self) -> &Operator {
        &self.operator
    }

    #[must_use]
    pub const fn scope(&self) -> Scope {
        self.scope
    }

    /// When the token stops working. A long-lived connection must stop serving at this time.
    #[must_use]
    pub const fn expires_at(&self) -> DateTime<Utc> {
        self.expires_at
    }

    pub(super) const fn token(&self) -> Uuid {
        self.token
    }

    /// Whether the identity still holds at `now`.
    pub(super) fn valid_at(&self, now: DateTime<Utc>) -> bool {
        tokens::usable(self.expires_at, now)
    }

    /// Adds the identity and its `Operator` to one HTTP request's data.
    #[must_use]
    pub fn attach(self, request: Request) -> Request {
        request.data(self.operator.clone()).data(self)
    }

    /// The identity and its `Operator` as WebSocket connection data, for the value
    /// `on_connection_init` returns.
    #[must_use]
    pub fn into_connection_data(self) -> Data {
        let mut data = Data::default();
        data.insert(self.operator.clone());
        data.insert(self);
        data
    }
}

/// The identity of an HTTP request from its `Authorization: Bearer <token>` header.
///
/// `None` when there is no header, more than one, another scheme, or a token that is
/// malformed, unknown or expired; the request then runs without an identity.
///
/// # Errors
///
/// `Error::Database` if the token cannot be looked up.
pub async fn identify(
    db: &PgPool,
    headers: &HeaderMap,
    now: DateTime<Utc>,
) -> Result<Option<Identity>, Error> {
    match bearer(headers) {
        Some(token) => resolve(db, token, now).await,
        None => Ok(None),
    }
}

/// The identity of a WebSocket connection from its `connection_init` payload
/// `{ "authorization": "Bearer <token>" }` (K7 "Authentication transport", R-020), with the
/// same scheme rule as the HTTP header. `None` when the key is missing or not a string, for
/// another scheme, and for a token that is malformed, unknown or expired. A token is never
/// taken from a URL query.
///
/// # Errors
///
/// `Error::Database` if the token cannot be looked up.
pub async fn identify_connection(
    db: &PgPool,
    payload: &serde_json::Value,
    now: DateTime<Utc>,
) -> Result<Option<Identity>, Error> {
    let authorization = payload
        .get("authorization")
        .and_then(serde_json::Value::as_str);
    match authorization.and_then(bearer_token) {
        Some(token) => resolve(db, token, now).await,
        None => Ok(None),
    }
}

/// The token of the one `Authorization` header with the `Bearer` scheme.
pub(super) fn bearer(headers: &HeaderMap) -> Option<&str> {
    let mut values = headers.get_all(AUTHORIZATION).iter();
    let value = values.next()?;
    if values.next().is_some() {
        return None;
    }
    bearer_token(value.to_str().ok()?)
}

/// The token in an authorization value `Bearer <token>`; the scheme is case-insensitive.
pub(super) fn bearer_token(authorization: &str) -> Option<&str> {
    let (scheme, token) = authorization.split_once(' ')?;
    scheme.eq_ignore_ascii_case("bearer").then(|| token.trim())
}

/// The identity a presented token stands for at `now`, writing `last_used_at` at most once a
/// minute.
pub(super) async fn resolve(
    db: &PgPool,
    presented: &str,
    now: DateTime<Utc>,
) -> Result<Option<Identity>, Error> {
    let Some(hash) = tokens::hash_of(presented) else {
        return Ok(None);
    };
    let Some(row) = store::token_by_hash(db, hash).await? else {
        return Ok(None);
    };
    let Some(scope) = Scope::from_stored(&row.scope) else {
        return Ok(None);
    };
    if !tokens::usable(row.expires_at, now) {
        return Ok(None);
    }
    if tokens::touch_due(row.last_used_at, now) {
        store::touch_token(db, row.id, now).await?;
    }
    let operator = Operator {
        id: row.operator_id,
        name: row.username,
    };
    Ok(Some(Identity::new(operator, row.id, scope, row.expires_at)))
}

/// The client address of a request: the one the network layer (WP-05 T4) put in the request
/// extensions, else the TCP peer. An IPv4-mapped IPv6 peer is given as IPv4 (D-11).
#[must_use]
pub fn client_addr(extensions: &Extensions) -> Option<ClientAddr> {
    extensions.get::<ClientAddr>().copied().or_else(|| {
        extensions
            .get::<ConnectInfo<SocketAddr>>()
            .map(|ConnectInfo(peer)| ClientAddr(peer.ip().to_canonical()))
    })
}
