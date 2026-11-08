//! `GET /metrics`: the checks and devices in the Prometheus text format 0.0.4 (INT-001,
//! INT-002, D-24).
//!
//! The route does no authentication: the caller mounts it behind the bearer layer with READ
//! scope, inside `crate::logged` and the permitted-networks layer. One build reads a fixed
//! number of statements whatever the number of checks. Its text is served to every scrape for
//! ten seconds; a scrape the client abandons does not stop or restart the build, so scrapes
//! cost at most one build per ten seconds. A failed build is logged once, here or in the cache
//! when it took longer than 30 s, and answers 500 with a fixed body for two seconds.

mod cache;
mod labels;
mod model;
mod render;
mod snapshot;
#[cfg(test)]
mod tests;

use std::sync::Arc;

use axum::{
    Router,
    extract::State,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use sqlx::PgPool;

use crate::clock::Clock;
use cache::Cache;

/// The media type of the text format 0.0.4.
const CONTENT_TYPE: &str = "text/plain; version=0.0.4; charset=utf-8";
/// The body of a failed build; it names no cause.
const FAILED: &str = "The metrics cannot be read now.";

#[derive(Clone)]
struct ExpositionState {
    db: PgPool,
    clock: Arc<dyn Clock>,
    cache: Arc<Cache>,
}

/// `GET /metrics`, cached for ten seconds with one detached build at a time (D-24).
pub fn exposition_router(db: PgPool, clock: Arc<dyn Clock>) -> Router {
    Router::new()
        .route("/metrics", get(scrape))
        .with_state(ExpositionState {
            db,
            clock,
            cache: Arc::default(),
        })
}

async fn scrape(State(state): State<ExpositionState>) -> Response {
    let now = state.clock.now();
    let db = state.db.clone();
    let build = move || async move {
        match snapshot::take(&db, now).await {
            Ok(snapshot) => Some(render::render(&snapshot)),
            Err(error) => {
                // checks::Error's text is fixed and holds no value (LOG-007).
                tracing::error!(%error, "the metrics exposition cannot be built");
                None
            }
        }
    };
    match state.cache.body(now, build).await {
        Some(body) => ([(header::CONTENT_TYPE, CONTENT_TYPE)], body).into_response(),
        None => (StatusCode::INTERNAL_SERVER_ERROR, FAILED).into_response(),
    }
}
