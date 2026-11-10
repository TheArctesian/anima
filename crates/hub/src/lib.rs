//! Hub library face: module tree, HTTP router and the serving steps around it, so integration
//! tests can reach them.

pub mod access;
pub mod api_errors;
pub mod builds;
pub mod checks;
pub mod clock;
pub mod config;
pub mod credentials;
pub mod devices;
pub mod events;
pub mod fingerprint;
pub mod ids;
pub mod maintenance;
pub mod metrics;
pub mod notifications;
pub mod paging;
pub mod retention;
pub mod scalars;
pub mod schema;
pub mod sessions;
pub mod ssh;
pub mod status;
pub mod targets;
pub mod text;
pub mod trust;
pub mod web_client;
pub mod web_interfaces;

use std::{future::Future, path::Path, sync::Arc, time::Duration};

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
    /// `ANIMA_PUBLIC_ORIGIN`; without it no request is marked same-origin, so no browser
    /// cookie is read and `signInBrowser` is refused (D-3, D-4).
    origin: Option<access::PublicOrigin>,
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
/// Without a public origin it reads no browser cookie; [`router_at`] serves browsers too.
pub fn router(schema: schema::HubSchema, db: PgPool, clock: Arc<dyn clock::Clock>) -> Router {
    routes(GraphqlRoute {
        schema,
        db,
        clock,
        origin: None,
    })
}

/// The routes of [`router`] for clients at `origin`, `ANIMA_PUBLIC_ORIGIN`: a request from that
/// origin may also sign in with the browser cookie and call `signInBrowser` (D-3, D-4).
pub fn router_at(
    schema: schema::HubSchema,
    db: PgPool,
    clock: Arc<dyn clock::Clock>,
    origin: access::PublicOrigin,
) -> Router {
    routes(GraphqlRoute {
        schema,
        db,
        clock,
        origin: Some(origin),
    })
}

fn routes(route: GraphqlRoute) -> Router {
    logged(
        Router::new()
            .route("/graphql", post(graphql).get(subscriptions))
            .route("/health", get(health))
            .layer(DefaultBodyLimit::max(BODY_LIMIT))
            .layer(client_origins())
            .with_state(route),
    )
}

/// Runs every route of `routes` in an `http_request` span that records the route template,
/// never a path value, so `/push/{token}` is logged as `/push/[redacted]` (LOG-007, R-028).
/// Apply it to a router after its last route is added; [`all_routes`] wraps each router it
/// merges in, such as `/tunnel` and `/push`.
pub fn logged(routes: Router) -> Router {
    routes.layer(middleware::from_fn(events::process_log::http_request))
}

/// The web client as the hub's fallback when `root` is set (D-2): any other `GET` gets a file
/// or `index.html`, in a logged span like every route. Merge it beside the explicit routes,
/// which win, and before the NET-002 layer, which then covers it. Without a root it adds
/// nothing, so an unknown path gets 404.
pub fn web_fallback(root: Option<&Path>, origin: &access::PublicOrigin) -> Router {
    root.map_or_else(Router::new, |root| {
        logged(web_client::web_client(root, origin))
    })
}

/// The routes devices push to, logged by their templates only, never their tokens (LOG-007,
/// R-028): `/push/{token}` for push checks, and the agent's enrolment and metrics batches (K8).
pub fn pushes(dependencies: &schema::Dependencies) -> Router {
    let schema::Dependencies {
        clock,
        pool,
        bus,
        metrics_feed,
        ..
    } = dependencies;
    let agent = metrics::agent_router(
        pool.clone(),
        metrics_feed.clone(),
        bus.clone(),
        clock.clone(),
    );
    logged(checks::push_router(pool.clone(), bus.clone(), clock.clone()).merge(agent))
}

/// Every route the hub serves, as `main.rs` serves it: `/graphql` and `/health` at the public
/// origin of `dependencies.config`, then the routes [`beside_graphql`] adds, all behind the
/// permitted-networks layer of `edge` (K3, NET-002). The schema takes `dependencies`.
pub fn all_routes(dependencies: schema::Dependencies, edge: access::Edge) -> Router {
    let origin = access::PublicOrigin::new(dependencies.config.public_origin.clone());
    let beside = beside_graphql(&dependencies, &origin);
    let (pool, clock) = (dependencies.pool.clone(), dependencies.clock.clone());
    router_at(schema::build_schema(dependencies), pool, clock, origin)
        .merge(beside)
        .layer(access::permitted_networks_layer(edge))
}

/// The routes beside `/graphql`, each logged by its template only, never its token (K3,
/// LOG-007, R-028, R-051): the session tunnels, the push routes ([`pushes`]), `/metrics` behind a READ bearer
/// (D-24) and, last, the web client at `config.web_root` as the fallback (D-2).
fn beside_graphql(dependencies: &schema::Dependencies, origin: &access::PublicOrigin) -> Router {
    let (pool, clock) = (&dependencies.pool, &dependencies.clock);
    let web_root = dependencies.config.web_root.as_deref();
    tunnels(dependencies)
        .merge(pushes(dependencies))
        .merge(exposition(pool.clone(), clock.clone()))
        .merge(web_fallback(web_root, origin))
}

/// `GET /metrics` for scrapers (INT-001, D-24): the checks' exposition behind a bearer token of
/// READ scope or more, in a logged span (K3, WP-43 A-9). A refused scrape gets 401 and starts
/// no build. The layer wraps the route only, never the fallback, so a merge cannot put the
/// bearer check on unknown paths. Merge it before the NET-002 layer, which then covers it.
pub fn exposition(db: PgPool, clock: Arc<dyn clock::Clock>) -> Router {
    let scrapes = checks::exposition_router(db.clone(), clock.clone());
    logged(scrapes.route_layer(access::bearer_layer(db, clock, access::Scope::Read)))
}

/// The session tunnels, each logged by its template only (K4 Sections 8, 9): `/terminal` for
/// SSH, `/tunnel` to guacd for RDP and VNC, which refuses SSH tickets so none reaches guacd
/// (R-062), and `/files`, which connects through the link and so gets the probes' deny list.
///
/// `main.rs` builds one address policy for every outbound connection (R-030, R-032): `/files`,
/// the probes and the collector take it as `policy`; the terminal, the Guacamole tunnel and the
/// connection tests take it through `live`'s target rule ([`sessions::Live::denying`]).
pub fn tunnels(dependencies: &schema::Dependencies) -> Router {
    let schema::Dependencies {
        config,
        clock,
        pool,
        bus,
        vault,
        live,
        policy,
        ..
    } = dependencies;
    let serving = sessions::Serving {
        db: pool.clone(),
        vault: vault.clone(),
        bus: bus.clone(),
        clock: clock.clone(),
        live: live.clone(),
        guacd: config.guacd.clone(),
        drive_root: config.guacd_drive_root.to_string_lossy().into_owned(),
    };
    let terminal = sessions::terminal_router(&serving);
    let tunnel = sessions::tunnel_router(&serving);
    let files = sessions::files_router(
        pool.clone(),
        vault.clone(),
        bus.clone(),
        clock.clone(),
        live.clone(),
        policy.clone(),
    );
    logged(terminal).merge(logged(tunnel)).merge(logged(files))
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
/// the client address, the Origin mark and the bearer token or browser cookie once, then runs
/// it (D-3, D-6).
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
    let from = (route.origin.as_ref()).and_then(|origin| access::SameOrigin::of(&headers, origin));
    let identified = access::identify(&route.db, &headers, from.as_ref(), route.clock.now()).await;
    if let Some(from) = from {
        request = request.data(from);
    }
    let response = match identified {
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

/// Upgrades `GET /graphql` to a `graphql-transport-ws` connection. The token comes from the
/// `connection_init` payload, or from the browser cookie of an upgrade from the public origin;
/// never from the URL (R-020, D-3). A `GET` that is not a WebSocket upgrade gets 405: queries
/// are never read from a URL.
async fn subscriptions(
    State(route): State<GraphqlRoute>,
    headers: HeaderMap,
    extensions: Extensions,
    upgrade: Result<WebSocketUpgrade, WebSocketUpgradeRejection>,
) -> Response {
    let Ok(upgrade) = upgrade else {
        return StatusCode::METHOD_NOT_ALLOWED.into_response();
    };
    let mut access =
        access::SocketAccess::new(route.db, route.clock, access::client_addr(&extensions));
    if let Some(origin) = &route.origin {
        access = access.with_upgrade(&headers, origin);
    }
    access::accept_socket(upgrade, route.schema, access)
}
