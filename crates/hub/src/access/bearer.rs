//! The bearer check of HTTP routes outside GraphQL, such as `GET /metrics` (K3, K7 "HTTP
//! surface", WP-43 A-9).

#[cfg(test)]
mod tests;

use std::{convert::Infallible, sync::Arc};

use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse as _, Response},
    routing::Route,
};
use sqlx::PgPool;
use tower::{Layer, Service};

use super::{Error, Scope, identity, tokens::Transport};
use crate::clock::Clock;

/// What [`bearer_layer`] checks each request against.
#[derive(Clone)]
struct Gate {
    db: PgPool,
    clock: Arc<dyn Clock>,
    scope: Scope,
}

/// axum middleware for routes outside GraphQL: a request passes only with one
/// `Authorization: Bearer` header holding a login token or an API key, unexpired, whose scope
/// covers `scope` (A-9). The token is looked up as [`identity::identify`] looks up a bearer,
/// so a browser token, an agent token and an enrolment code are refused (A-3, R-092). The
/// cookie is never read.
///
/// Any other request gets 401 with an empty body and no `WWW-Authenticate` header, and the
/// inner route does not run. When the token cannot be looked up, 503 or 500 with an empty body,
/// and the fault is logged by kind. Nothing else is logged, and never the token (LOG-007).
///
/// Add it with `Router::route_layer`: `Router::layer` also wraps the router's fallback, which a
/// merge can make the fallback of the whole hub.
pub fn bearer_layer(
    db: PgPool,
    clock: Arc<dyn Clock>,
    scope: Scope,
) -> impl Layer<
    Route,
    Service: Service<Request, Response = Response, Error = Infallible, Future: Send + 'static>
                 + Clone
                 + Send
                 + Sync
                 + 'static,
> + Clone
+ Send
+ Sync
+ 'static {
    middleware::from_fn_with_state(Gate { db, clock, scope }, admit)
}

async fn admit(State(gate): State<Gate>, request: Request, next: Next) -> Response {
    match gate.admits(request.headers()).await {
        Ok(true) => next.run(request).await,
        Ok(false) => StatusCode::UNAUTHORIZED.into_response(),
        Err(error) => {
            error.log();
            let status = match error {
                Error::Unreachable(_) => StatusCode::SERVICE_UNAVAILABLE,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            status.into_response()
        }
    }
}

impl Gate {
    /// Whether `headers` carry a bearer token that grants this gate's scope now.
    async fn admits(&self, headers: &HeaderMap) -> Result<bool, Error> {
        let Some(token) = identity::bearer(headers) else {
            return Ok(false);
        };
        let now = self.clock.now();
        let identity = identity::resolve(&self.db, token, Transport::Bearer, now).await?;
        Ok(identity.is_some_and(|identity| covers(identity.scope(), self.scope)))
    }
}

/// Whether a token of scope `granted` may do what `wanted` asks: a write token may also read.
/// The builder scope covers nothing here and nothing covers it: a builder token reaches only
/// the builder mutations (BLD-022, D-23).
const fn covers(granted: Scope, wanted: Scope) -> bool {
    matches!(
        (granted, wanted),
        (Scope::Write, Scope::Read | Scope::Write) | (Scope::Read, Scope::Read)
    )
}
