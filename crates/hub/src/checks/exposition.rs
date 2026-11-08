//! `GET /metrics`: the checks and devices in the Prometheus text format 0.0.4 (INT-001,
//! INT-002, D-24).
//!
//! The route does no authentication: the caller mounts it behind the bearer layer with READ
//! scope, inside `crate::logged` and the permitted-networks layer. One build reads a fixed
//! number of statements whatever the number of checks, and its text is served for ten seconds
//! to every scrape, so a token holder cannot load the database. A failed build answers 500
//! with a fixed body, is logged once here and is not kept.

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

/// `GET /metrics`, cached for ten seconds with one shared build at a time (D-24).
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
    let build = || async {
        match snapshot::take(&state.db, now).await {
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
