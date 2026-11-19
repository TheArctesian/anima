//! `POST /work/v1/claim`, `POST /work/v1/report` and `POST /work/v1/secret`: the work route
//! (K10, `contracts/work-route.md`, accepted by R-209). The transport, the credential, the
//! limits and the refusals are those of the daemon routes of K8: the daemon token is the
//! credential, and the checks run in the order of a renewal. The rules of a job are in
//! `work::jobs`; this file only reads a request and writes an answer.
//!
//! `/work/v1/secret` answers `SECRET_REFUSED` to each request that passes the credential and
//! the form: no purpose of K10 Section 8 has a source in the hub yet (tasks B2 and A2).

use std::net::IpAddr;

use axum::{
    body::Body,
    extract::Request,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse as _, Response},
};
use chrono::{DateTime, Utc};
use daemon_core::wire::{Claim, RefusalCode, SecretRequest, WorkReport};
use serde::de::DeserializeOwned;

use super::{
    Error,
    admission::{Admitted, Stop, admit},
    credential, refusals,
    refusals::Route,
    routes::{DaemonState, busy, client_of, read_body, unmounted},
    turns::Turn,
};
use crate::{
    fleet::events::ClientAddr,
    ids::DeviceId,
    work::jobs::{self, Claimed, Queue, Reporting},
};

/// The body limits of K10 Section 2.
const CLAIM_LIMIT: usize = 8 * 1024;
const REPORT_LIMIT: usize = 96 * 1024;
const SECRET_LIMIT: usize = 4 * 1024;

/// A request of a daemon whose token is usable, with its body read by the form `T`.
struct Asked<T> {
    device: DeviceId,
    name: String,
    body: T,
    /// The turn of the device, held until the answer is made (K8 Section 6).
    _turn: Turn<DeviceId>,
}

/// The bearer before the body, the size, then the JSON (as a renewal).
async fn asked<T: DeserializeOwned>(
    state: &DaemonState,
    (headers, body): (&HeaderMap, Body),
    address: Turn<IpAddr>,
    (limit, now): (usize, DateTime<Utc>),
) -> Result<Asked<T>, Stop> {
    let token =
        credential::bearer(headers).ok_or(Stop::Refused(RefusalCode::TokenRefused, None))?;
    let Admitted { device, name, turn } = admit(state, &token, address, now).await?;
    drop(token);
    let refused = |code| Stop::Refused(code, Some(device));
    let bytes = read_body(state, headers, body, limit)
        .await
        .map_err(refused)?;
    let body = serde_json::from_slice::<T>(&bytes).map_err(|_| refused(RefusalCode::Malformed))?;
    Ok(Asked {
        device,
        name,
        body,
        _turn: turn,
    })
}

/// What each of the three routes does before its own work, and how it answers a stop.
macro_rules! served {
    ($state:ident, $request:ident, $route:expr, $answer:ident) => {{
        let Some(client) = client_of(&$request) else {
            return unmounted();
        };
        let now = $state.clock.now();
        let Some(address) = $state.turns.address(client.0) else {
            return busy();
        };
        let (parts, body) = $request.into_parts();
        match $answer(&$state, (&parts.headers, body), (address, client), now).await {
            Ok(response) => response,
            Err(stop) => $state.stopped(stop, ($route, client), now).await,
        }
    }};
}

/// Answers a claim: `200` with a job, `204` with none, or the first refusal.
pub(super) async fn serve_claim(state: DaemonState, request: Request) -> Response {
    served!(state, request, Route::WorkClaim, claimed)
}

/// Answers a report: `200` with the new lease, or the first refusal.
pub(super) async fn serve_report(state: DaemonState, request: Request) -> Response {
    served!(state, request, Route::WorkReport, reported)
}

/// Answers a request for a secret of a job: a refusal (see the head of this file).
pub(super) async fn serve_secret(state: DaemonState, request: Request) -> Response {
    served!(state, request, Route::WorkSecret, secret)
}

fn failed(error: jobs::Error) -> Stop {
    Stop::Failed(Error::Jobs(error))
}

async fn claimed(
    state: &DaemonState,
    request: (&HeaderMap, Body),
    (address, client): (Turn<IpAddr>, ClientAddr),
    now: DateTime<Utc>,
) -> Result<Response, Stop> {
    let asked: Asked<Claim> = asked(state, request, address, (CLAIM_LIMIT, now)).await?;
    let device = asked.device;
    if DeviceId::from(asked.body.device) != device {
        return Err(Stop::Refused(RefusalCode::WrongDevice, Some(device)));
    }
    let _request = state.turns.request().await.ok_or(Stop::Busy)?;
    let queue = Queue {
        db: &state.db,
        bus: &state.bus,
    };
    match jobs::claim(queue, (device, &asked.name), &asked.body, (client, now))
        .await
        .map_err(failed)?
    {
        Claimed::Nothing => Ok(StatusCode::NO_CONTENT.into_response()),
        Claimed::Off => Err(Stop::Refused(RefusalCode::WorkOff, Some(device))),
        Claimed::Job(job) => Ok(refusals::json(StatusCode::OK, &*job)),
    }
}

async fn reported(
    state: &DaemonState,
    request: (&HeaderMap, Body),
    (address, client): (Turn<IpAddr>, ClientAddr),
    now: DateTime<Utc>,
) -> Result<Response, Stop> {
    let asked: Asked<WorkReport> = asked(state, request, address, (REPORT_LIMIT, now)).await?;
    let device = asked.device;
    let refused = |code| Stop::Refused(code, Some(device));
    let _request = state.turns.request().await.ok_or(Stop::Busy)?;
    let queue = Queue {
        db: &state.db,
        bus: &state.bus,
    };
    match jobs::report(queue, device, &asked.body, (client, now))
        .await
        .map_err(failed)?
    {
        Reporting::Reported(answer) => Ok(refusals::json(StatusCode::OK, &answer)),
        Reporting::NotHolder => Err(refused(RefusalCode::NotHolder)),
        Reporting::Malformed => Err(refused(RefusalCode::Malformed)),
        Reporting::Off => Err(refused(RefusalCode::WorkOff)),
    }
}

async fn secret(
    state: &DaemonState,
    request: (&HeaderMap, Body),
    (address, _client): (Turn<IpAddr>, ClientAddr),
    now: DateTime<Utc>,
) -> Result<Response, Stop> {
    let asked: Asked<SecretRequest> = asked(state, request, address, (SECRET_LIMIT, now)).await?;
    Err(Stop::Refused(
        RefusalCode::SecretRefused,
        Some(asked.device),
    ))
}
