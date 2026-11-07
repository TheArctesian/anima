//! The push route: a device reports its own result to `/push/{token}` (CHK-003).
//!
//! The token in the path is the only authentication, so a request never needs a sign-in and
//! the route answers 404 with an empty body for every token that is not a live PUSH check's.
//! Neither the path nor the token reaches a log or an error message (LOG-007).

use std::{sync::Arc, time::Duration};

use axum::{
    Router,
    body::to_bytes,
    extract::{Path, Query, Request, State, rejection::QueryRejection},
    http::{Method, StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use sqlx::PgPool;
use subtle::ConstantTimeEq as _;

use super::{
    Attempt, Beat, Check, Error,
    probe::{Judgement, ProbeResult, clean_message},
    scheduler::in_maintenance,
    store,
};
use crate::{
    clock::Clock,
    devices,
    events::{Bus, HeartbeatSummary, Signal},
};

/// A push request carries nothing the hub reads from its body.
const BODY_LIMIT: usize = 1024;
/// 32 bytes in unpadded base64url.
const TOKEN_LENGTH: usize = 43;
/// The longest response time a device may report, in milliseconds.
const PING_MAX_MS: u32 = 600_000;
/// The longest a push may take to answer. A push is one small write, so a slower one is a
/// stuck database, and the device is told to try again rather than held for the edge's
/// one-minute limit (NET-002).
const ANSWER_TIME: Duration = Duration::from_secs(10);

#[derive(Clone)]
struct PushState {
    db: PgPool,
    bus: Bus,
    clock: Arc<dyn Clock>,
    answer_time: Duration,
}

/// `GET|POST /push/{token}`, answering within ten seconds. The caller mounts it behind the
/// permitted-networks layer (NET-002) and inside `crate::logged`, which logs the path as
/// `/push/[redacted]` (LOG-007).
pub fn push_router(db: PgPool, bus: Bus, clock: Arc<dyn Clock>) -> Router {
    push_router_within(db, bus, clock, ANSWER_TIME)
}

/// [`push_router`] with its answer time as a parameter, so a test need not wait ten seconds.
pub(in crate::checks) fn push_router_within(
    db: PgPool,
    bus: Bus,
    clock: Arc<dyn Clock>,
    answer_time: Duration,
) -> Router {
    Router::new()
        .route("/push/{token}", get(accept).post(accept))
        .with_state(PushState {
            db,
            bus,
            clock,
            answer_time,
        })
}

/// The push's answer, or 503 once `answer_time` has passed. Dropping the late answer rolls
/// back its transaction, so a timed-out push records nothing.
async fn accept(
    State(state): State<PushState>,
    Path(token): Path<String>,
    query: Result<Query<PushQuery>, QueryRejection>,
    request: Request,
) -> Response {
    let limit = state.answer_time;
    let answer = answer(state, token, query, request);
    tokio::time::timeout(limit, answer)
        .await
        .unwrap_or_else(|_| StatusCode::SERVICE_UNAVAILABLE.into_response())
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
            judgement: Judgement::Answered,
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

/// Records the report as an attempt: the store evaluates it against the check's state and
/// settings under the row lock, so a push and a scheduled attempt never count from a stale
/// state. `None` when nothing was written (the check is paused or has a beat at least as
/// new). Maintenance covers a push as it covers a scheduled attempt, so a pushed `down`
/// under maintenance is recorded as Maintenance (CHK-006).
async fn store_beat(
    db: &PgPool,
    check: &Check,
    probe: &ProbeResult,
    now: DateTime<Utc>,
) -> Result<Option<Beat>, Error> {
    let device = devices::get(db, check.device)
        .await
        .map_err(Error::Devices)?;
    let in_maintenance = in_maintenance(db, &device, (check.id, check.parent), now)
        .await
        .map_err(Error::Maintenance)?;
    let attempt = Attempt {
        result: probe,
        in_maintenance,
        at: now,
    };
    let mut tx = db.begin().await?;
    let beat = store::record_attempt(&mut tx, check.id, &attempt).await?;
    tx.commit().await?;
    Ok(beat)
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
    let Some(beat) = store_beat(&state.db, &check, &report.into_probe(), now).await? else {
        // Skipped: a push at the instant of the last beat has nothing new to record. The
        // device did report in, so it is answered as a success, and nothing is published.
        return Ok(Outcome::Recorded);
    };
    state.bus.publish(Signal::Heartbeat {
        check: check.id,
        device: check.device,
        beat: HeartbeatSummary {
            at: beat.at,
            status: beat.status,
            response_time_ms: beat.response_time_ms,
            message: beat.message.as_deref().map(clean_message),
        },
    });
    Ok(Outcome::Recorded)
}

async fn answer(
    state: PushState,
    token: String,
    query: Result<Query<PushQuery>, QueryRejection>,
    request: Request,
) -> Response {
    let (parts, body) = request.into_parts();
    // GET and POST only: axum routes HEAD to a GET handler, and a probe of the URL must not
    // count as the device reporting in. Uptime Kuma pushes with GET.
    if parts.method == Method::HEAD {
        let allow = [(header::ALLOW, "GET, POST")];
        return (StatusCode::METHOD_NOT_ALLOWED, allow).into_response();
    }
    if !is_token_shaped(&token) {
        return StatusCode::NOT_FOUND.into_response();
    }
    // A declared length over the cap is refused at once, before any wait for the body.
    let declared = parts
        .headers
        .get(header::CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<usize>().ok());
    if declared.is_some_and(|length| length > BODY_LIMIT) {
        return StatusCode::PAYLOAD_TOO_LARGE.into_response();
    }
    // The body is never used; reading it under the cap is what refuses a larger one.
    if to_bytes(body, BODY_LIMIT).await.is_err() {
        return StatusCode::PAYLOAD_TOO_LARGE.into_response();
    }
    match record(&state, &token, query).await {
        Ok(Outcome::Recorded) => (StatusCode::OK, "ok").into_response(),
        Ok(Outcome::Refused) => StatusCode::BAD_REQUEST.into_response(),
        // Not there: unknown, paused, or deleted while the push was in flight.
        Ok(Outcome::NotFound) | Err(Error::NotFound(_)) => StatusCode::NOT_FOUND.into_response(),
        Err(error) => {
            // The error's message holds no path, token or SQL (LOG-007).
            tracing::error!(%error, "a push result could not be recorded");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
