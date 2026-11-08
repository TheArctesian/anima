//! Who is making a request: the bearer token, or a browser's cookie, resolved once at the HTTP
//! or WebSocket edge into an [`Identity`] that only `access` can build (D-3, D-6, API-003).

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
    ClientAddr, Error, Operator,
    browser::{self, SameOrigin},
    store,
    tokens::{self, Scope, TokenKind, Transport},
};

/// What a stored token grants: its row, its kind and scope, and until when.
#[derive(Clone, Copy)]
pub(super) struct Grant {
    pub token: Uuid,
    pub kind: TokenKind,
    pub scope: Scope,
    pub expires_at: DateTime<Utc>,
}

/// A signed-in request: the operator and the token it presented.
///
/// Fields are private and there is no public constructor, so request data from anywhere
/// else (for example a schema-level `.data(..)`) cannot pass the guard.
pub struct Identity {
    operator: Operator,
    grant: Grant,
}

impl Identity {
    pub(super) const fn new(operator: Operator, grant: Grant) -> Self {
        Self { operator, grant }
    }

    /// A write login identity for `operator` that holds until `expires_at`, for other domains'
    /// tests. It exists only with the `test-support` feature, which only the hub's own
    /// dev-dependency turns on: it is in `cargo test` builds, not in a `cargo build --release
    /// --bin anima-hub`, so the shipped hub cannot make an identity without a token (D-6).
    #[cfg(feature = "test-support")]
    #[must_use]
    pub const fn for_tests(operator: Operator, expires_at: DateTime<Utc>) -> Self {
        let grant = Grant {
            token: Uuid::nil(),
            kind: TokenKind::Login,
            scope: Scope::Write,
            expires_at,
        };
        Self::new(operator, grant)
    }

    #[must_use]
    pub const fn operator(&self) -> &Operator {
        &self.operator
    }

    #[must_use]
    pub const fn scope(&self) -> Scope {
        self.grant.scope
    }

    /// The kind of token the request signed in with.
    #[must_use]
    pub const fn kind(&self) -> TokenKind {
        self.grant.kind
    }

    /// When the token stops working. A long-lived connection must stop serving at this time.
    #[must_use]
    pub const fn expires_at(&self) -> DateTime<Utc> {
        self.grant.expires_at
    }

    pub(super) const fn token(&self) -> Uuid {
        self.grant.token
    }

    /// Whether the identity still holds at `now`.
    pub(super) fn valid_at(&self, now: DateTime<Utc>) -> bool {
        tokens::usable(self.grant.expires_at, now)
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

/// The identity of an HTTP request (K7 "Authentication transport", WP-43 A-3, A-4).
///
/// With any `Authorization` header, only that header counts: one `Bearer` header with a login
/// token or an API key. Without one, a request marked [`SameOrigin`] may sign in with the one
/// `__Host-anima` cookie, which must hold a browser token; a cookie from any other origin, or
/// with none, is ignored. `None` in every other case, and for a token that is malformed,
/// unknown, expired or of the wrong kind; the request then runs without an identity.
///
/// # Errors
///
/// `Error::Database` if the token cannot be looked up.
pub async fn identify(
    db: &PgPool,
    headers: &HeaderMap,
    from: Option<&SameOrigin>,
    now: DateTime<Utc>,
) -> Result<Option<Identity>, Error> {
    let presented = if headers.contains_key(AUTHORIZATION) {
        bearer(headers).map(|token| (token, Transport::Bearer))
    } else {
        from.and_then(|from| browser::cookie_token(headers, from))
            .map(|token| (token, Transport::Cookie))
    };
    match presented {
        Some((token, transport)) => resolve(db, token, transport, now).await,
        None => Ok(None),
    }
}

/// The identity of a WebSocket connection from its `connection_init` payload
/// `{ "authorization": "Bearer <token>" }` (K7 "Authentication transport", R-020), with the
/// same scheme rule as the HTTP header. `None` when the key is missing or not a string, for
/// another scheme, and for a token that is malformed, unknown, expired or a browser token. A
/// token is never taken from a URL query.
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
        Some(token) => resolve(db, token, Transport::Bearer, now).await,
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

/// The identity a presented token stands for at `now`, when its kind may come from
/// `transport` (A-3), writing `last_used_at` at most once a minute.
pub(super) async fn resolve(
    db: &PgPool,
    presented: &str,
    transport: Transport,
    now: DateTime<Utc>,
) -> Result<Option<Identity>, Error> {
    let Some((hash, spelling)) = tokens::read_presented(presented) else {
        return Ok(None);
    };
    let Some(row) = store::token_by_hash(db, hash).await? else {
        return Ok(None);
    };
    let (Some(kind), Some(scope)) = (
        TokenKind::from_stored(&row.kind),
        Scope::from_stored(&row.scope),
    ) else {
        return Ok(None);
    };
    if !tokens::admitted(kind, spelling, transport) || !tokens::usable(row.expires_at, now) {
        return Ok(None);
    }
    if tokens::touch_due(row.last_used_at, now) {
        store::touch_token(db, row.id, now).await?;
    }
    let operator = Operator {
        id: row.operator_id,
        name: row.username,
    };
    let grant = Grant {
        token: row.id,
        kind,
        scope,
        expires_at: row.expires_at,
    };
    Ok(Some(Identity::new(operator, grant)))
}

/// Used by the tunnel: resolve a bearer token to an operator. Only the desktop app's login token
/// counts: a browser token is no bearer (WP-43 A-3), and an API key, even a read one, is for
/// scripts, not for a session.
///
/// # Errors
///
/// `Error::Unauthenticated` for a malformed, unknown or expired token, a browser token or an API
/// key; `Error::Database` if it cannot be looked up.
pub async fn operator_for_token(
    db: &PgPool,
    token: &str,
    now: DateTime<Utc>,
) -> Result<Operator, Error> {
    let identity = resolve(db, token, Transport::Bearer, now)
        .await?
        .filter(|identity| identity.kind() == TokenKind::Login)
        .ok_or(Error::Unauthenticated)?;
    Ok(identity.operator().clone())
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
