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
    actor::{self, Declaration, MadeFor, Signals},
    browser::{self, SameOrigin},
    store,
    tokens::{self, Scope, TokenKind, Transport},
};
use crate::ids::TokenId;

/// What a stored token grants: its row, its kind and scope, and until when; for an API key,
/// also its name and what it is made for (LOG-015).
#[derive(Clone)]
pub(super) struct Grant {
    pub token: Uuid,
    pub kind: TokenKind,
    pub scope: Scope,
    pub expires_at: DateTime<Utc>,
    pub made_for: Option<MadeFor>,
    pub key_name: Option<String>,
}

impl Grant {
    /// A grant of `kind` and `scope` with no key facts, for tests.
    #[cfg(any(test, feature = "test-support"))]
    pub(super) const fn plain(kind: TokenKind, scope: Scope, expires_at: DateTime<Utc>) -> Self {
        Self {
            token: Uuid::nil(),
            kind,
            scope,
            expires_at,
            made_for: None,
            key_name: None,
        }
    }
}

/// A signed-in request: the operator, the token it presented and whether it declared an AI
/// agent. The operator's actor is decided from those, here only (LOG-014).
///
/// Fields are private and there is no public constructor, so request data from anywhere
/// else (for example a schema-level `.data(..)`) cannot pass the guard.
pub struct Identity {
    operator: Operator,
    grant: Grant,
}

impl Identity {
    /// The identity of `operator` with `grant`, with no declaration; the operator's actor is
    /// decided from the grant, whatever it held before.
    pub(super) fn new(operator: Operator, grant: Grant) -> Self {
        Self { operator, grant }.declared(Declaration::default())
    }

    /// The same identity with the request's agent declaration, which can only raise its
    /// actor kind (LOG-016, LOG-017).
    #[must_use]
    pub(super) fn declared(mut self, declaration: Declaration) -> Self {
        let signals = Signals {
            token: self.grant.kind,
            made_for: self.grant.made_for,
            declared: declaration,
        };
        self.operator.actor = actor::actor(signals, self.grant.key_name.clone());
        self
    }

    /// A write login identity for `operator` that holds until `expires_at`, for other domains'
    /// tests. It exists only with the `test-support` feature, which only the hub's own
    /// dev-dependency turns on: it is in `cargo test` builds, not in a `cargo build --release
    /// --bin anima-hub`, so the shipped hub cannot make an identity without a token (D-6).
    #[cfg(feature = "test-support")]
    #[must_use]
    pub fn for_tests(operator: Operator, expires_at: DateTime<Utc>) -> Self {
        Self::new(
            operator,
            Grant::plain(TokenKind::Login, Scope::Write, expires_at),
        )
    }

    /// A write API key identity for `operator`, for other domains' tests of a resolver's own
    /// token kind check (D-23). Only with the `test-support` feature, as [`Self::for_tests`].
    #[cfg(feature = "test-support")]
    #[must_use]
    pub fn api_key_for_tests(operator: Operator, expires_at: DateTime<Utc>) -> Self {
        let grant = Grant {
            made_for: Some(MadeFor::User),
            key_name: Some("tests".to_owned()),
            ..Grant::plain(TokenKind::ApiKey, Scope::Write, expires_at)
        };
        Self::new(operator, grant)
    }

    /// The operator the token stands for; `None` for a builder token, which stands for no
    /// operator, not even the one who enrolled it (BLD-022, R-107).
    #[must_use]
    pub fn operator(&self) -> Option<&Operator> {
        (self.grant.scope != Scope::Builder).then_some(&self.operator)
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

    /// The token of a builder identity, which names its builder record (BLD-024); `None` for
    /// an operator token.
    #[must_use]
    pub fn builder_token(&self) -> Option<TokenId> {
        (self.grant.scope == Scope::Builder).then(|| TokenId::from(self.grant.token))
    }

    /// The `Operator` resolvers may act as: none for a builder token, so an operator resolver
    /// that forgets its guard still finds no operator (BLD-022).
    fn acting_operator(&self) -> Option<Operator> {
        self.operator().cloned()
    }

    /// Whether the identity still holds at `now`.
    pub(super) fn valid_at(&self, now: DateTime<Utc>) -> bool {
        tokens::usable(self.grant.expires_at, now)
    }

    /// Adds the identity and, for an operator token, its `Operator` to one HTTP request's data.
    #[must_use]
    pub fn attach(self, request: Request) -> Request {
        match self.acting_operator() {
            Some(operator) => request.data(operator).data(self),
            None => request.data(self),
        }
    }

    /// The identity and, for an operator token, its `Operator` as WebSocket connection data,
    /// for the value `on_connection_init` returns.
    #[must_use]
    pub fn into_connection_data(self) -> Data {
        let mut data = Data::default();
        if let Some(operator) = self.acting_operator() {
            data.insert(operator);
        }
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
/// unknown, expired or of the wrong kind; the request then runs without an identity. The
/// identity carries the request's `Anima-Actor` declaration (LOG-016).
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
    let identity = match presented {
        Some((token, transport)) => resolve(db, token, transport, now).await?,
        None => None,
    };
    let declaration = Declaration::of_headers(headers);
    Ok(identity.map(|identity| identity.declared(declaration)))
}

/// The identity of a WebSocket connection from its `connection_init` payload
/// `{ "authorization": "Bearer <token>" }` (K7 "Authentication transport", R-020), with the
/// same scheme rule as the HTTP header. `None` when the key is missing or not a string, for
/// another scheme, and for a token that is malformed, unknown, expired or a browser token. A
/// token is never taken from a URL query. The identity carries the payload's
/// `"actor": "agent"` declaration (LOG-016).
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
    let identity = match authorization.and_then(bearer_token) {
        Some(token) => resolve(db, token, Transport::Bearer, now).await?,
        None => None,
    };
    let declaration = Declaration::of_payload(payload);
    Ok(identity.map(|identity| identity.declared(declaration)))
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
    if !tokens::consistent(kind, scope)
        || !tokens::admitted(kind, spelling, transport)
        || !tokens::usable(row.expires_at, now)
    {
        return Ok(None);
    }
    if tokens::touch_due(row.last_used_at, now) {
        store::touch_token(db, row.id, now).await?;
    }
    let made_for = MadeFor::from_stored(row.made_for.as_deref());
    if (kind == TokenKind::ApiKey) != made_for.is_some() {
        return Ok(None);
    }
    let operator = Operator {
        id: row.operator_id,
        name: row.username,
        actor: crate::events::Actor::hub(),
    };
    let grant = Grant {
        token: row.id,
        kind,
        scope,
        expires_at: row.expires_at,
        made_for,
        key_name: row.name,
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
    identity.operator().cloned().ok_or(Error::Unauthenticated)
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
