//! `POST /agent/v1/enrol`, `POST /agent/v1/push` and `POST /agent/v1/renew` (K8 Sections 1, 3
//! to 6); the push itself is answered in `push`.
//!
//! Signed out of GraphQL: the enrolment code in the body or the agent token in the
//! `Authorization` header is the only credential. A body is read only up to its limit, held in
//! a buffer wiped on drop, and never logged; a parse error is dropped unread, since its text
//! may quote the body (LOG-007). Every refusal is `{"error":"<CODE>"}`.

use std::{future::Future, net::IpAddr, sync::Arc, time::Duration};

use agent_core::wire::{AgentToken, EnrolRequest, RefusalCode, RenewRequest, Renewed, TOKEN_BYTES};
use axum::{
    Router,
    body::{Body, to_bytes},
    extract::{Request, State},
    http::{HeaderMap, StatusCode, header::CONTENT_LENGTH},
    response::Response,
    routing::post,
};
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use zeroize::Zeroizing;

use super::{
    Error, credential, enrolment, push, random,
    refusals::{self, Recorder, Refused, Route},
    renewal,
    turns::{AGENT_REQUESTS, Turn, Turns},
};
use crate::{
    clock::Clock,
    events::{Bus, ClientAddr},
    ids::DeviceId,
    metrics::Feed,
};

/// The enrol and renew bodies are small JSON objects (K8 Section 1).
const BODY_LIMIT: usize = 4 * 1024;

/// The longest a body may take to arrive. A slower one is answered `UNAVAILABLE`, so a body
/// that never ends holds no turn but its device's past this (K8 Section 1).
const BODY_TIME: Duration = Duration::from_secs(5);

/// The longest an answer may take. Each request is a few small writes, so a slower one is a
/// stuck database, and the agent is told to try again (as `/push`, NET-002).
const ANSWER_TIME: Duration = Duration::from_secs(10);

#[derive(Clone)]
pub(super) struct AgentState {
    pub(super) db: PgPool,
    pub(super) feed: Feed,
    pub(super) bus: Bus,
    pub(super) clock: Arc<dyn Clock>,
    pub(super) turns: Turns,
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
        turns: Turns::new(AGENT_REQUESTS),
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
        turns: Turns::new(AGENT_REQUESTS),
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
    let Some(_address) = state.turns.address(client.0) else {
        return busy();
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
    let Some(_request) = state.turns.request().await else {
        return busy();
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
    let Some(address) = state.turns.address(client.0) else {
        return busy();
    };
    let (parts, body) = request.into_parts();
    match renewed(&state, (&parts.headers, body), address, now).await {
        Ok(renewed) => refusals::json(StatusCode::OK, &renewed),
        Err(stop) => state.stopped(stop, (Route::Renew, client), now).await,
    }
}

async fn renewed(
    state: &AgentState,
    (headers, body): (&HeaderMap, Body),
    address: Turn<IpAddr>,
    now: DateTime<Utc>,
) -> Result<Renewed, Stop> {
    let current =
        credential::bearer(headers).ok_or(Stop::Refused(RefusalCode::TokenRefused, None))?;
    // A device switched away from the agent renews nothing (CR-18), as it pushes nothing.
    let Admitted {
        device,
        turn: _turn,
        ..
    } = admit(state, &current, address, now).await?;
    let refused = |code| Stop::Refused(code, Some(device));
    let body = read_body(headers, body, BODY_LIMIT)
        .await
        .map_err(refused)?;
    if !is_renew_request(&body) {
        return Err(refused(RefusalCode::Malformed));
    }
    let next = new_token().map_err(Stop::Failed)?;
    let _request = state.turns.request().await.ok_or(Stop::Busy)?;
    match renewal::renew(&state.db, &current, next, now).await {
        Ok(Some(renewed)) => Ok(renewed),
        Ok(None) => Err(refused(RefusalCode::TokenRefused)),
        Err(error) => Err(Stop::Failed(error)),
    }
}

/// Why an agent push or renewal stopped short of its answer.
pub(super) enum Stop {
    /// Refused by K8 Section 6, for the device of a valid token when there was one.
    Refused(RefusalCode, Option<DeviceId>),
    /// The hub cannot answer now (`UNAVAILABLE`).
    Failed(Error),
    /// Another request has the turn this one needs (`UNAVAILABLE`, nothing recorded).
    Busy,
}

/// A device whose agent may go on: its token is usable and its metrics come from the agent.
pub(super) struct Admitted {
    pub(super) device: DeviceId,
    pub(super) name: String,
    /// The device's one turn (MET-017), held until the answer.
    pub(super) turn: Turn<DeviceId>,
}

/// The device whose usable token `token` is at `now`, while it takes its metrics from the agent,
/// with its turn. The lookups run under a request turn and under the client `address`'s turn;
/// both end before this returns, so the body that follows arrives holding neither.
pub(super) async fn admit(
    state: &AgentState,
    token: &AgentToken,
    address: Turn<IpAddr>,
    now: DateTime<Utc>,
) -> Result<Admitted, Stop> {
    let (device, name) = {
        let _request = state.turns.request().await.ok_or(Stop::Busy)?;
        let device = holder(state, token, now).await?;
        (device, agent_name(state, device).await?)
    };
    drop(address);
    let turn = state.turns.device(device).ok_or(Stop::Busy)?;
    Ok(Admitted { device, name, turn })
}

/// The device whose usable token `token` is at `now`.
async fn holder(
    state: &AgentState,
    token: &AgentToken,
    now: DateTime<Utc>,
) -> Result<DeviceId, Stop> {
    match credential::authenticate(&state.db, token, now).await {
        Ok(Some(device)) => Ok(device),
        Ok(None) => Err(Stop::Refused(RefusalCode::TokenRefused, None)),
        Err(error) => Err(Stop::Failed(error)),
    }
}

/// The device's name while its metrics come from the agent; `TOKEN_REFUSED` once the operator
/// switched the source away or deleted the device.
async fn agent_name(state: &AgentState, device: DeviceId) -> Result<String, Stop> {
    match credential::agent_name(&state.db, device).await {
        Ok(Some(name)) => Ok(name),
        Ok(None) => Err(Stop::Refused(RefusalCode::TokenRefused, Some(device))),
        Err(error) => Err(Stop::Failed(error)),
    }
}

impl AgentState {
    /// The answer to a request that stopped short: a refusal is written to the event log
    /// first, `UNAVAILABLE` is not.
    pub(super) async fn stopped(
        &self,
        stop: Stop,
        (route, client): (Route, ClientAddr),
        now: DateTime<Utc>,
    ) -> Response {
        match stop {
            Stop::Refused(code, device) => {
                let refused = Refused {
                    route,
                    code,
                    client,
                    device,
                };
                self.refuse(refused, now).await
            }
            Stop::Failed(error) => unavailable(&error),
            Stop::Busy => busy(),
        }
    }

    /// Answers the refusal after writing its event; a failed write is logged and the answer
    /// stays the refusal.
    pub(super) async fn refuse(&self, refused: Refused, now: DateTime<Utc>) -> Response {
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

/// Another request has the turn this one needs (`turns`). Not an error and not an event: the
/// agent keeps its points and tries again (K8 Section 6).
pub(super) fn busy() -> Response {
    refusals::answer(RefusalCode::Unavailable)
}

/// A new token from the OS random source.
fn new_token() -> Result<AgentToken, Error> {
    let bytes = random::<TOKEN_BYTES>()?;
    Ok(AgentToken::from_random(*bytes))
}

/// The body, read up to `limit` bytes into a buffer wiped on drop; `TOO_LARGE` past it, or at
/// once when the declared length is past it, and `UNAVAILABLE` when it has not arrived within
/// [`BODY_TIME`] (K8 Section 1).
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
    let bytes = tokio::time::timeout(BODY_TIME, to_bytes(body, limit))
        .await
        .map_err(|_| RefusalCode::Unavailable)?
        .map_err(|_| RefusalCode::TooLarge)?;
    Ok(Zeroizing::new(Vec::from(bytes)))
}

/// Whether the body is a JSON object, as `{}` with members the hub ignores (K8 Section 5).
fn is_renew_request(body: &[u8]) -> bool {
    body.trim_ascii_start().starts_with(b"{")
        && serde_json::from_slice::<RenewRequest>(body).is_ok()
}
