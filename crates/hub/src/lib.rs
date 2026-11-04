//! Hub library face: module tree, HTTP router and the serving steps around it, so integration
//! tests can reach them.

pub mod access;
pub mod clock;
pub mod config;
pub mod credentials;
pub mod devices;
pub mod events;
pub mod ids;
pub mod paging;
pub mod scalars;
pub mod schema;
pub mod sessions;
pub mod status;

use std::{future::Future, net::SocketAddr, sync::Arc};

use async_graphql_axum::GraphQLResponse;
use axum::{
    Router,
    body::Bytes,
    extract::{
        DefaultBodyLimit, State, WebSocketUpgrade, ws::rejection::WebSocketUpgradeRejection,
    },
    http::{Extensions, HeaderMap, StatusCode, header::CONTENT_TYPE},
    response::{IntoResponse, Response},
    routing::post,
};
use sqlx::PgPool;
use tokio::net::TcpListener;

/// Largest `/graphql` request body. A bigger one gets 413 before it is parsed or any token
/// or password is checked (SEC-002).
pub const BODY_LIMIT: usize = 1024 * 1024;

/// What the `/graphql` handler needs besides the request.
#[derive(Clone)]
struct Edge {
    schema: schema::HubSchema,
    db: PgPool,
    clock: Arc<dyn clock::Clock>,
}

pub fn router(schema: schema::HubSchema, db: PgPool, clock: Arc<dyn clock::Clock>) -> Router {
    Router::new()
        .route("/graphql", post(graphql).get(subscriptions))
        .layer(DefaultBodyLimit::max(BODY_LIMIT))
        .with_state(Edge { schema, db, clock })
}

/// The sessions this hub process holds, with what ends them when it stops (R-040).
pub struct HeldSessions {
    db: PgPool,
    live: sessions::Live,
    clock: Arc<dyn clock::Clock>,
}

impl HeldSessions {
    /// A clone of the process's one `Live`, for the schema and the tunnel router (R-032).
    #[must_use]
    pub fn live(&self) -> sessions::Live {
        self.live.clone()
    }
}

/// Ends, with `HUB_SHUTDOWN`, the sessions a previous hub process left open and logs how many
/// (a number only), then holds a new `Live` (R-040, D-13). Call once, before binding: only
/// its `Live` issues tickets, so the stale rows end before any request is served.
///
/// # Errors
/// When the stale rows cannot be ended; then none is.
pub async fn hold_sessions(
    db: PgPool,
    clock: Arc<dyn clock::Clock>,
) -> Result<HeldSessions, sessions::Error> {
    let ended = sessions::close_stale(&db, clock.now()).await?;
    tracing::info!(ended, "stale sessions ended");
    Ok(HeldSessions {
        db,
        live: sessions::Live::default(),
        clock,
    })
}

/// Serves `app`, with each request's TCP peer as its connect info, until `signal` resolves.
/// Then, in order: ends every held session with `HUB_SHUTDOWN`, because the drain does not
/// end upgraded connections such as tunnels; then drains the open requests (R-040).
///
/// # Errors
/// When the server fails to serve.
pub async fn serve_until(
    listener: TcpListener,
    app: Router,
    held: HeldSessions,
    signal: impl Future<Output = ()> + Send + 'static,
) -> std::io::Result<()> {
    let stop = async move {
        signal.await;
        let HeldSessions { db, live, clock } = held;
        if let Err(error) = sessions::shut_down(&db, &live, clock.now()).await {
            // The error's text is fixed and holds no value (sessions::Error, LOG-007).
            tracing::error!(%error, "sessions did not all end at shutdown");
        }
    };
    let app = app.into_make_service_with_connect_info::<SocketAddr>();
    axum::serve(listener, app)
        .with_graceful_shutdown(stop)
        .await
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
    State(edge): State<Edge>,
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
    let response = match access::identify(&edge.db, &headers, edge.clock.now()).await {
        Ok(Some(identity)) => edge.schema.execute(identity.attach(request)).await,
        Ok(None) => edge.schema.execute(request).await,
        Err(error) => {
            error.log();
            async_graphql::Response::from_errors(vec![error.to_server_error()])
        }
    };
    GraphQLResponse::from(response).into_response()
}

/// Upgrades `GET /graphql` to a `graphql-transport-ws` connection. The token comes only from
/// the `connection_init` payload, never from the URL (R-020). A `GET` that is not a WebSocket
/// upgrade gets 405: queries are never read from a URL.
async fn subscriptions(
    State(edge): State<Edge>,
    extensions: Extensions,
    upgrade: Result<WebSocketUpgrade, WebSocketUpgradeRejection>,
) -> Response {
    let Ok(upgrade) = upgrade else {
        return StatusCode::METHOD_NOT_ALLOWED.into_response();
    };
    let access = access::SocketAccess::new(edge.db, edge.clock, access::client_addr(&extensions));
    access::accept_socket(upgrade, edge.schema, access)
}
