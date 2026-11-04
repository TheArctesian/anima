//! The push route: a device reports its own result to `/push/{token}` (CHK-003, R-028).
//!
//! The token in the path is the only authentication, so a request never needs a sign-in and
//! the route answers 404 with an empty body for every token that is not a live PUSH check's.
//! Neither the path nor the token reaches a log or an error message (LOG-007).

use std::{sync::Arc, time::Duration};

use axum::{
    Router,
    body::Bytes,
    extract::{DefaultBodyLimit, Path, Query, State, rejection::QueryRejection},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use serde::Deserialize;
use sqlx::PgPool;
use subtle::ConstantTimeEq as _;

use super::{
    Beat, Check, Error, evaluate,
    probe::{ProbeResult, clean_message},
    push_check, record_beat,
};
use crate::{
    clock::Clock,
    events::{Bus, HeartbeatSummary, Signal},
};

/// A push request carries nothing the hub reads from its body.
const BODY_LIMIT: usize = 1024;
/// 32 bytes in unpadded base64url.
const TOKEN_LENGTH: usize = 43;
/// The longest response time a device may report, in milliseconds.
const PING_MAX_MS: u32 = 600_000;

#[derive(Clone)]
struct PushState {
    db: PgPool,
    bus: Bus,
    clock: Arc<dyn Clock>,
}

/// `GET|POST /push/{token}`. The caller mounts it behind the permitted-networks layer (NET-002).
pub fn push_router(db: PgPool, bus: Bus, clock: Arc<dyn Clock>) -> Router {
    Router::new()
        .route("/push/{token}", get(accept).post(accept))
        .layer(DefaultBodyLimit::max(BODY_LIMIT))
        .with_state(PushState { db, bus, clock })
}

#[derive(Deserialize)]
struct PushQuery {
    status: Option<String>,
    msg: Option<String>,
    ping: Option<String>,
}

/// What the device reported.
#[derive(Debug, PartialEq, Eq)]
struct Report {
    up: bool,
    message: Option<String>,
    ping_ms: Option<u32>,
}

impl PushQuery {
    /// `None` when `status` is not `up` or `down`, or `ping` is not 0 to 600000.
    fn report(self) -> Option<Report> {
        let up = match self.status.as_deref() {
            None | Some("up") => true,
            Some("down") => false,
            Some(_) => return None,
        };
        let ping_ms = match self.ping {
            None => None,
            Some(text) => Some(text.parse::<u32>().ok().filter(|ms| *ms <= PING_MAX_MS)?),
        };
        let message = self
            .msg
            .map(|text| clean_message(&text))
            .filter(|text| !text.is_empty());
        Some(Report {
            up,
            message,
            ping_ms,
        })
    }
}

impl Report {
    fn into_probe(self) -> ProbeResult {
        ProbeResult {
            ok: self.up,
            response_time: self.ping_ms.map(|ms| Duration::from_millis(u64::from(ms))),
            message: self.message,
            resolved_ip: None,
            cert_not_after: None,
        }
    }
}

fn is_token_shaped(text: &str) -> bool {
    text.len() == TOKEN_LENGTH
        && text
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
}

/// Whether the check is a live PUSH check holding exactly this token; the bytes are compared
/// in constant time.
fn accepts(check: &Check, token: &str) -> bool {
    let holds = check
        .push_token
        .as_ref()
        .is_some_and(|stored| bool::from(stored.expose().as_bytes().ct_eq(token.as_bytes())));
    holds && !check.paused
}

enum Outcome {
    Recorded,
    NotFound,
    Refused,
}

async fn record(
    state: &PushState,
    token: &str,
    query: Result<Query<PushQuery>, QueryRejection>,
) -> Result<Outcome, Error> {
    let Some(check) = push_check(&state.db, token).await? else {
        return Ok(Outcome::NotFound);
    };
    if !accepts(&check, token) {
        return Ok(Outcome::NotFound);
    }
    let Some(report) = query.ok().and_then(|Query(query)| query.report()) else {
        return Ok(Outcome::Refused);
    };
    let now = state.clock.now();
    let probe = report.into_probe();
    let evaluated = evaluate(&check.state, probe.clone(), &check.settings, now);
    let beat = Beat::of(probe, evaluated, now);
    let mut tx = state.db.begin().await?;
    record_beat(&mut tx, check.id, &beat).await?;
    tx.commit().await?;
    state.bus.publish(Signal::Heartbeat {
        check: check.id,
        device: check.device,
        beat: HeartbeatSummary {
            at: beat.at,
            status: beat.status,
            response_time_ms: beat.response_time_ms,
        },
    });
    Ok(Outcome::Recorded)
}

async fn accept(
    State(state): State<PushState>,
    Path(token): Path<String>,
    query: Result<Query<PushQuery>, QueryRejection>,
    // Reading the body is what makes `DefaultBodyLimit` refuse a larger one.
    _body: Bytes,
) -> Response {
    if !is_token_shaped(&token) {
        return StatusCode::NOT_FOUND.into_response();
    }
    match record(&state, &token, query).await {
        Ok(Outcome::Recorded) => (StatusCode::OK, "ok").into_response(),
        Ok(Outcome::NotFound) => StatusCode::NOT_FOUND.into_response(),
        Ok(Outcome::Refused) => StatusCode::BAD_REQUEST.into_response(),
        Err(error) => {
            // The error's message holds no path, token or SQL (LOG-007).
            tracing::error!(%error, "a push result could not be recorded");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
