//! `POST /agent/v1/enrol`, `POST /agent/v1/push` and `POST /agent/v1/renew` (K8 Sections 1, 3
//! to 6); the push itself is answered in `push`.
//!
//! Signed out of GraphQL: the enrolment code in the body or the agent token in the
//! `Authorization` header is the only credential. A body is read only up to its limit, held in
//! a buffer wiped on drop, and never logged; a parse error is dropped unread, since its text
//! may quote the body (LOG-007). Every refusal is `{"error":"<CODE>"}`.

use std::{future::Future, net::IpAddr, sync::Arc, time::Duration};

use agent_core::wire::{
    AgentToken, EnrolRequest, Enrolled, EnrolmentCode, RefusalCode, RenewRequest, Renewed,
    TOKEN_BYTES,
};
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
    Error,
    admission::{Admitted, Stop, admit},
    credential, enrolment, push, random,
    refusals::{self, Recorder, Refused, Route},
    renewal,
    turns::{AGENT_REQUESTS, Turn, Turns},
};
use crate::{
    clock::Clock,
    events::{Bus, ClientAddr},
    metrics::Feed,
};

/// The enrol and renew bodies are small JSON objects (K8 Section 1).
const BODY_LIMIT: usize = 4 * 1024;

/// How long an agent request may take, in its parts.
#[derive(Clone, Copy)]
pub(in crate::metrics) struct Deadlines {
    /// The whole answer. Each request is a few small writes, so a slower one is a stuck
    /// database, and the agent is told to try again (as `/push`, NET-002).
    pub(in crate::metrics) answer: Duration,
    /// The body's arrival. A slower body is answered `UNAVAILABLE`, so a body that never ends
    /// holds no turn but its device's past this (K8 Section 1).
    pub(in crate::metrics) body: Duration,
    /// The wait for a database turn (`turns`, K8 Section 6).
    pub(in crate::metrics) request: Duration,
}

impl Deadlines {
    /// The hub's deadlines (K8 Sections 1 and 6).
    pub(in crate::metrics) const HUB: Self = Self {
        answer: Duration::from_secs(10),
        body: Duration::from_secs(5),
        request: Duration::from_secs(2),
    };
}

#[derive(Clone)]
pub(super) struct AgentState {
    pub(super) db: PgPool,
    pub(super) feed: Feed,
    pub(super) bus: Bus,
    pub(super) clock: Arc<dyn Clock>,
    pub(super) turns: Turns,
    recorder: Recorder,
    deadlines: Deadlines,
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
        turns: Turns::new(AGENT_REQUESTS, Deadlines::HUB.request),
        recorder,
        deadlines: Deadlines::HUB,
    })
}

/// The test seam of [`agent_router`]: the deadlines are a parameter, so a test of one need not
/// wait for the hub's.
#[cfg(test)]
pub(in crate::metrics) fn agent_router_within(
    db: PgPool,
    bus: Bus,
    clock: Arc<dyn Clock>,
    deadlines: Deadlines,
) -> Router {
    let recorder = Recorder::new(db.clone(), bus.clone());
    routes(AgentState {
        db,
        feed: Feed::new(),
        bus,
        clock,
        turns: Turns::new(AGENT_REQUESTS, deadlines.request),
        recorder,
        deadlines,
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
    let limit = state.deadlines.answer;
    within(limit, serve_enrol(state, request)).await
}

async fn push(State(state): State<AgentState>, request: Request) -> Response {
    let limit = state.deadlines.answer;
    within(limit, push::serve_push(state, request)).await
}

async fn renew(State(state): State<AgentState>, request: Request) -> Response {
    let limit = state.deadlines.answer;
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
    let Some(address) = state.turns.address(client.0) else {
        return busy();
    };
    let (parts, body) = request.into_parts();
    let code = match read_body(&state, &parts.headers, body, BODY_LIMIT).await {
        Ok(body) => serde_json::from_slice::<EnrolRequest>(&body).ok(),
        Err(code) => {
            drop(address);
            return state.refuse(refused(code), now).await;
        }
    };
    // One turn covers the redeem and the enrolment: a code used by the one must not meet a
    // busy answer before the other. A refusal is written after the turns end (K8 Section 6).
    let enrolled = match code {
        Some(EnrolRequest { code }) => {
            // Made before the code is redeemed, so a failed random source uses up no code.
            let token = match new_token() {
                Ok(token) => token,
                Err(error) => return unavailable(&error),
            };
            let Some(_request) = state.turns.request().await else {
                return busy();
            };
            redeem_and_enrol(&state, (&code, token), now).await
        }
        None => Ok(None),
    };
    drop(address);
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

/// The enrolment `code` gives at `now`, handing out `token`; `None` when the code is not usable
/// or its device cannot take it.
async fn redeem_and_enrol(
    state: &AgentState,
    (code, token): (&EnrolmentCode, AgentToken),
    now: DateTime<Utc>,
) -> Result<Option<Enrolled>, Error> {
    let Some(redeemed) = enrolment::redeem(&state.db, code, now).await? else {
        return Ok(None);
    };
    enrolment::enrol(&state.db, &state.bus, (redeemed, token), now).await
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
    let body = read_body(state, headers, body, BODY_LIMIT)
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
/// the state's body deadline (K8 Section 1).
pub(super) async fn read_body(
    state: &AgentState,
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
    let bytes = tokio::time::timeout(state.deadlines.body, to_bytes(body, limit))
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
