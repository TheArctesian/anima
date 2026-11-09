//! `POST /agent/v1/enrol`, `POST /agent/v1/push` and `POST /agent/v1/renew` (K8 Sections 1, 3
//! to 6); the push itself is answered in `push`.
//!
//! Signed out of GraphQL: the enrolment code in the body or the agent token in the
//! `Authorization` header is the only credential. A body is read only up to its limit, held in
//! a buffer wiped on drop, and never logged; a parse error is dropped unread, since its text
//! may quote the body (LOG-007). Every refusal is `{"error":"<CODE>"}`.

use std::{future::Future, sync::Arc, time::Duration};

use agent_core::wire::{AgentToken, EnrolRequest, RefusalCode, RenewRequest, TOKEN_BYTES};
use axum::{
    Router,
    body::{Body, to_bytes},
    extract::{Request, State},
    http::{HeaderMap, StatusCode, header::CONTENT_LENGTH},
    response::Response,
    routing::post,
};
use sqlx::PgPool;
use zeroize::Zeroizing;

use super::{
    Error, credential, enrolment, push, random,
    refusals::{self, Recorder, Refused, Route},
    renewal,
};
use crate::{
    clock::Clock,
    events::{Bus, ClientAddr},
    metrics::Feed,
};

/// The enrol and renew bodies are small JSON objects (K8 Section 1).
const BODY_LIMIT: usize = 4 * 1024;

/// The longest an answer may take. Each request is a few small writes, so a slower one is a
/// stuck database, and the agent is told to try again (as `/push`, NET-002).
const ANSWER_TIME: Duration = Duration::from_secs(10);

#[derive(Clone)]
pub(super) struct AgentState {
    pub(super) db: PgPool,
    pub(super) feed: Feed,
    pub(super) bus: Bus,
    pub(super) clock: Arc<dyn Clock>,
    recorder: Recorder,
    answer_time: Duration,
}

/// `POST /agent/v1/enrol`, `POST /agent/v1/push` and `POST /agent/v1/renew`, each answering
/// within ten seconds (K8).
/// The caller mounts it inside `crate::logged` and behind the permitted-networks layer, whose
/// client address every refusal event names (NET-002); without that layer each request is
/// refused as `UNAVAILABLE`.
pub fn agent_router(db: PgPool, feed: Feed, bus: Bus, clock: Arc<dyn Clock>) -> Router {
    let recorder = Recorder::new(db.clone(), bus.clone());
    routes(AgentState {
        db,
        feed,
        bus,
        clock,
        recorder,
        answer_time: ANSWER_TIME,
    })
}

/// The test seam of [`agent_router`]: the answer time is a parameter, so a test of the deadline
/// need not wait ten seconds.
#[cfg(test)]
pub(in crate::metrics) fn agent_router_within(
    db: PgPool,
    bus: Bus,
    clock: Arc<dyn Clock>,
    answer_time: Duration,
) -> Router {
    let recorder = Recorder::new(db.clone(), bus.clone());
    routes(AgentState {
        db,
        feed: Feed::new(),
        bus,
        clock,
        recorder,
        answer_time,
    })
}

fn routes(state: AgentState) -> Router {
    Router::new()
        .route("/agent/v1/enrol", post(enrol))
        .route("/agent/v1/push", post(push))
        .route("/agent/v1/renew", post(renew))
        .with_state(state)
}

async fn enrol(State(state): State<AgentState>, request: Request) -> Response {
    let limit = state.answer_time;
    within(limit, serve_enrol(state, request)).await
}

async fn push(State(state): State<AgentState>, request: Request) -> Response {
    let limit = state.answer_time;
    within(limit, push::serve_push(state, request)).await
}

async fn renew(State(state): State<AgentState>, request: Request) -> Response {
    let limit = state.answer_time;
    within(limit, serve_renew(state, request)).await
}

/// The answer, or `UNAVAILABLE` once `limit` has passed. Dropping the late answer rolls back
/// its open transaction; a code it already redeemed stays used (MET-012).
async fn within(limit: Duration, answer: impl Future<Output = Response>) -> Response {
    tokio::time::timeout(limit, answer)
        .await
        .unwrap_or_else(|_| {
            tracing::warn!("an agent request was not answered in time");
            refusals::answer(RefusalCode::Unavailable)
        })
}

/// Swaps an enrolment code for an agent token (K8 Section 3). Every bad code, every malformed
/// body and a code whose device was revoked or deleted meanwhile gets the same
/// `ENROLMENT_REFUSED`.
async fn serve_enrol(state: AgentState, request: Request) -> Response {
    let Some(client) = client_of(&request) else {
        return unmounted();
    };
    let now = state.clock.now();
    let refused = |code| Refused {
        route: Route::Enrol,
        code,
        client,
        device: None,
    };
    let (parts, body) = request.into_parts();
    let code = match read_body(&parts.headers, body, BODY_LIMIT).await {
        Ok(body) => serde_json::from_slice::<EnrolRequest>(&body).ok(),
        Err(code) => return state.refuse(refused(code), now).await,
    };
    let Some(EnrolRequest { code }) = code else {
        return state
            .refuse(refused(RefusalCode::EnrolmentRefused), now)
            .await;
    };
    let redeemed = match enrolment::redeem(&state.db, &code, now).await {
        Ok(Some(redeemed)) => redeemed,
        Ok(None) => {
            return state
                .refuse(refused(RefusalCode::EnrolmentRefused), now)
                .await;
        }
        Err(error) => return unavailable(&error),
    };
    drop(code);
    let enrolled = match new_token() {
        Ok(token) => enrolment::enrol(&state.db, &state.bus, (redeemed, token), now).await,
        Err(error) => Err(error),
    };
    match enrolled {
        Ok(Some(enrolled)) => refusals::json(StatusCode::OK, &enrolled),
        Ok(None) | Err(Error::NoDevice) => {
            state
                .refuse(refused(RefusalCode::EnrolmentRefused), now)
                .await
        }
        Err(error) => unavailable(&error),
    }
}

/// Stores a renewed token for the holder of the bearer token (K8 Section 5). The bearer is
/// checked before the body is read.
async fn serve_renew(state: AgentState, request: Request) -> Response {
    let Some(client) = client_of(&request) else {
        return unmounted();
    };
    let now = state.clock.now();
    let refused = |code, device| Refused {
        route: Route::Renew,
        code,
        client,
        device,
    };
    let (parts, body) = request.into_parts();
    let Some(current) = credential::bearer(&parts.headers) else {
        return state
            .refuse(refused(RefusalCode::TokenRefused, None), now)
            .await;
    };
    let device = match credential::authenticate(&state.db, &current, now).await {
        Ok(Some(device)) => device,
        Ok(None) => {
            return state
                .refuse(refused(RefusalCode::TokenRefused, None), now)
                .await;
        }
        Err(error) => return unavailable(&error),
    };
    let well_formed = match read_body(&parts.headers, body, BODY_LIMIT).await {
        Ok(body) => is_renew_request(&body),
        Err(code) => return state.refuse(refused(code, Some(device)), now).await,
    };
    if !well_formed {
        return state
            .refuse(refused(RefusalCode::Malformed, Some(device)), now)
            .await;
    }
    let renewed = match new_token() {
        Ok(next) => renewal::renew(&state.db, &current, next, now).await,
        Err(error) => Err(error),
    };
    match renewed {
        Ok(Some(renewed)) => refusals::json(StatusCode::OK, &renewed),
        Ok(None) => {
            state
                .refuse(refused(RefusalCode::TokenRefused, None), now)
                .await
        }
        Err(error) => unavailable(&error),
    }
}

impl AgentState {
    /// Answers the refusal after writing its event; a failed write is logged and the answer
    /// stays the refusal.
    pub(super) async fn refuse(
        &self,
        refused: Refused,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Response {
        if let Err(error) = self.recorder.record(&refused, now).await {
            tracing::error!(%error, "an agent refusal could not be written to the event log");
        }
        refusals::answer(refused.code)
    }
}

/// The client address the permitted-networks layer put on the request (NET-002).
pub(super) fn client_of(request: &Request) -> Option<ClientAddr> {
    request.extensions().get::<ClientAddr>().copied()
}

/// Without the permitted-networks layer a request has no checked client address; it is refused
/// rather than served (fail closed).
pub(super) fn unmounted() -> Response {
    tracing::error!("an agent route is mounted without the permitted-networks layer");
    refusals::answer(RefusalCode::Unavailable)
}

/// The hub cannot answer now; the error's text is fixed and holds no value (LOG-007).
pub(super) fn unavailable(error: &Error) -> Response {
    tracing::error!(%error, "an agent request could not be answered");
    refusals::answer(RefusalCode::Unavailable)
}

/// A new token from the OS random source.
fn new_token() -> Result<AgentToken, Error> {
    let bytes = random::<TOKEN_BYTES>()?;
    Ok(AgentToken::from_random(*bytes))
}

/// The body, read up to `limit` bytes into a buffer wiped on drop; `TOO_LARGE` past it, or at
/// once when the declared length is past it (K8 Section 1).
pub(super) async fn read_body(
    headers: &HeaderMap,
    body: Body,
    limit: usize,
) -> Result<Zeroizing<Vec<u8>>, RefusalCode> {
    let declared = headers
        .get(CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<usize>().ok());
    if declared.is_some_and(|length| length > limit) {
        return Err(RefusalCode::TooLarge);
    }
    let bytes = to_bytes(body, limit)
        .await
        .map_err(|_| RefusalCode::TooLarge)?;
    Ok(Zeroizing::new(Vec::from(bytes)))
}

/// Whether the body is a JSON object, as `{}` with members the hub ignores (K8 Section 5).
fn is_renew_request(body: &[u8]) -> bool {
    body.trim_ascii_start().starts_with(b"{")
        && serde_json::from_slice::<RenewRequest>(body).is_ok()
}
