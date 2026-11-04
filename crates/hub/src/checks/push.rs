//! The push route: a device reports its own result to `/push/{token}` (CHK-003, R-028).
//!
//! The token in the path is the only authentication, so a request never needs a sign-in and
//! the route answers 404 with an empty body for every token that is not a live PUSH check's.
//! Neither the path nor the token reaches a log or an error message (LOG-007).

use std::{sync::Arc, time::Duration};

use axum::{
    Router,
    body::to_bytes,
    extract::{Path, Query, Request, State, rejection::QueryRejection},
    http::{Method, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use sqlx::PgPool;
use subtle::ConstantTimeEq as _;

use super::{
    Beat, Check, Error, Recorded, evaluate,
    probe::{ProbeResult, clean_message},
    record_beat, store,
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
    /// The message is cleaned here and again when the store writes it (defence in depth).
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
///
/// Defence in depth: the lookup already matched the token exactly in SQL, so the comparison
/// here cannot change an answer today. It keeps the route safe if the lookup ever becomes a
/// prefix or case-folded match, and it takes the same time for every near miss.
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

/// Evaluates the report against the check's settings and records the beat in one transaction.
/// `None` when the store skipped it (the check is paused or the beat is stale). This is the one
/// place that decides the state the beat is evaluated against.
async fn store_beat(
    db: &PgPool,
    check: &Check,
    probe: ProbeResult,
    now: DateTime<Utc>,
) -> Result<Option<Beat>, Error> {
    let evaluated = evaluate(&check.state, &probe, &check.settings, now);
    let beat = Beat::of(probe, evaluated, now);
    let mut tx = db.begin().await?;
    let recorded = record_beat(&mut tx, check.id, &beat).await?;
    tx.commit().await?;
    Ok((recorded == Recorded::Written).then_some(beat))
}

async fn record(
    state: &PushState,
    token: &str,
    query: Result<Query<PushQuery>, QueryRejection>,
) -> Result<Outcome, Error> {
    let Some(check) = store::by_push_token(&state.db, token).await? else {
        return Ok(Outcome::NotFound);
    };
    if !accepts(&check, token) {
        return Ok(Outcome::NotFound);
    }
    let Some(report) = query.ok().and_then(|Query(query)| query.report()) else {
        return Ok(Outcome::Refused);
    };
    let now = state.clock.now();
    let Some(beat) = store_beat(&state.db, &check, report.into_probe(), now).await? else {
        return Ok(Outcome::NotFound);
    };
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
    request: Request,
) -> Response {
    let (parts, body) = request.into_parts();
    // GET and POST only: axum routes HEAD to a GET handler, and a probe of the URL must not
    // count as the device reporting in. Uptime Kuma pushes with GET.
    if parts.method == Method::HEAD {
        return StatusCode::METHOD_NOT_ALLOWED.into_response();
    }
    if !is_token_shaped(&token) {
        return StatusCode::NOT_FOUND.into_response();
    }
    // The body is never used; reading it under the cap is what refuses a larger one.
    if to_bytes(body, BODY_LIMIT).await.is_err() {
        return StatusCode::PAYLOAD_TOO_LARGE.into_response();
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
