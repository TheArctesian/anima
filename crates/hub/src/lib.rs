//! Hub library face: module tree, HTTP router and the serving steps around it, so integration
//! tests can reach them.

pub mod access;
pub mod checks;
pub mod clock;
pub mod config;
pub mod credentials;
pub mod devices;
pub mod events;
pub mod ids;
pub mod maintenance;
pub mod paging;
pub mod scalars;
pub mod schema;
pub mod sessions;
pub mod status;

use std::{future::Future, sync::Arc, time::Duration};

use async_graphql_axum::GraphQLResponse;
use axum::{
    Router,
    body::Bytes,
    extract::{
        DefaultBodyLimit, State, WebSocketUpgrade, ws::rejection::WebSocketUpgradeRejection,
    },
    http::{
        Extensions, HeaderMap, HeaderValue, Method, StatusCode,
        header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE},
    },
    middleware,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use sqlx::PgPool;
use tower_http::cors::{AllowOrigin, CorsLayer};

/// Largest `/graphql` request body. A bigger one gets 413 before it is parsed or any token
/// or password is checked (SEC-002).
pub const BODY_LIMIT: usize = 1024 * 1024;

/// What the `/graphql` handler needs besides the request.
#[derive(Clone)]
struct GraphqlRoute {
    schema: schema::HubSchema,
    db: PgPool,
    clock: Arc<dyn clock::Clock>,
}

/// The origins of the desktop app's web view: `tauri://localhost` on macOS and Linux,
/// `http://tauri.localhost` on Windows. The web view sends a CORS preflight before each
/// `POST /graphql`; only these origins are allowed, and every request still needs a token
/// (SEC-002).
pub const CLIENT_ORIGINS: [&str; 2] = ["tauri://localhost", "http://tauri.localhost"];

/// How long a web view may reuse a preflight answer.
const PREFLIGHT_MAX_AGE: Duration = Duration::from_secs(600);

/// The CORS rule for `/graphql`: the client origins may `POST` with the headers the client sends.
fn client_origins() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(AllowOrigin::list(
            CLIENT_ORIGINS.map(HeaderValue::from_static),
        ))
        .allow_methods([Method::POST])
        .allow_headers([CONTENT_TYPE, ACCEPT, AUTHORIZATION])
        .max_age(PREFLIGHT_MAX_AGE)
}

/// The hub's HTTP routes: `POST /graphql`, `GET /graphql` (subscriptions) and `GET /health`,
/// each request in a logged span. `/graphql` answers CORS preflights from the client origins.
pub fn router(schema: schema::HubSchema, db: PgPool, clock: Arc<dyn clock::Clock>) -> Router {
    logged(
        Router::new()
            .route("/graphql", post(graphql).get(subscriptions))
            .route("/health", get(health))
            .layer(DefaultBodyLimit::max(BODY_LIMIT))
            .layer(client_origins())
            .with_state(GraphqlRoute { schema, db, clock }),
    )
}

/// Runs every route of `routes` in an `http_request` span that records the route template,
/// never a path value, so `/push/{token}` is logged as `/push/[redacted]` (LOG-007, R-028).
/// Apply it to a router after its last route is added; `main.rs` wraps each router it merges
/// in, such as `/tunnel` and `/push`.
pub fn logged(routes: Router) -> Router {
    routes.layer(middleware::from_fn(events::process_log::http_request))
}

/// Liveness probe (R-026(4)): no sign-in, no database, body `ok`.
async fn health() -> &'static str {
    "ok"
}

/// The longest the drain may wait for open requests after the stop signal. A client can hold
/// a request open for ever (a partial body, say); past this the hub stops anyway, and its
/// sessions have already ended (R-040).
pub const DRAIN_LIMIT: Duration = Duration::from_secs(10);

/// What the hub ends when it stops, and how long it waits for open requests (R-040).
pub struct Shutdown {
    pub db: PgPool,
    /// The `Live` the schema and the tunnel serve with.
    pub live: sessions::Live,
    pub clock: Arc<dyn clock::Clock>,
    /// [`DRAIN_LIMIT`] in the hub.
    pub drain_limit: Duration,
}

/// Serves `app` through `entrance` (the NET-002 accept-time check, TLS when configured), with
/// each request's TCP peer as its connect info, until `signal` resolves. Then, in order: ends
/// every session `shutdown.live` holds with `HUB_SHUTDOWN`, because the drain does not end
/// upgraded connections such as tunnels; then stops accepting and drains the open requests for
/// at most `shutdown.drain_limit` (R-040).
pub async fn serve_until(
    entrance: access::listen::Entrance,
    app: Router,
    shutdown: Shutdown,
    signal: impl Future<Output = ()> + Send + 'static,
) {
    let Shutdown {
        db,
        live,
        clock,
        drain_limit,
    } = shutdown;
    let stop = async move {
        signal.await;
        if let Err(error) = sessions::shut_down(&db, &live, clock.now()).await {
            // The error's text is fixed and holds no value (sessions::Error, LOG-007).
            tracing::error!(%error, "sessions did not all end at shutdown");
        }
    };
    entrance.serve_until(app, stop, drain_limit).await;
}

/// Whether the media type is exactly `application/json`, in any case, parameters allowed.
fn is_json(headers: &HeaderMap) -> bool {
    headers
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .is_some_and(|media| media.trim().eq_ignore_ascii_case("application/json"))
}

/// Reads one JSON request of at most [`BODY_LIMIT`] bytes (no batch, no multipart), resolves
/// the bearer token and the client address once, then runs it (D-6).
async fn graphql(
    State(route): State<GraphqlRoute>,
    headers: HeaderMap,
    extensions: Extensions,
    body: Bytes,
) -> Response {
    if !is_json(&headers) {
        return (StatusCode::UNSUPPORTED_MEDIA_TYPE, "Send one JSON request.").into_response();
    }
    let Ok(mut request) = serde_json::from_slice::<async_graphql::Request>(&body) else {
        return (
            StatusCode::BAD_REQUEST,
            "The request is not a GraphQL request.",
        )
            .into_response();
    };
    if let Some(client) = access::client_addr(&extensions) {
        request = request.data(client);
    }
    let response = match access::identify(&route.db, &headers, route.clock.now()).await {
        Ok(Some(identity)) => route.schema.execute(identity.attach(request)).await,
        Ok(None) => route.schema.execute(request).await,
        // The schema's boundary never sees this error, so the route logs its cause chain (H-5).
        Err(error) => {
            let error = error.into_response_error();
            events::process_log::log_failure(&error);
            async_graphql::Response::from_errors(vec![error])
        }
    };
    GraphQLResponse::from(response).into_response()
}

/// Upgrades `GET /graphql` to a `graphql-transport-ws` connection. The token comes only from
/// the `connection_init` payload, never from the URL (R-020). A `GET` that is not a WebSocket
/// upgrade gets 405: queries are never read from a URL.
async fn subscriptions(
    State(route): State<GraphqlRoute>,
    extensions: Extensions,
    upgrade: Result<WebSocketUpgrade, WebSocketUpgradeRejection>,
) -> Response {
    let Ok(upgrade) = upgrade else {
        return StatusCode::METHOD_NOT_ALLOWED.into_response();
    };
    let access = access::SocketAccess::new(route.db, route.clock, access::client_addr(&extensions));
    access::accept_socket(upgrade, route.schema, access)
}
