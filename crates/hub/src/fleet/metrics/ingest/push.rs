//! `POST /daemon/v1/push` (K8 Sections 4 and 6; MET-015, MET-017, MET-018).
//!
//! The checks run in K8's order and stop at the first refusal: the bearer, before the body is
//! read; the size; the JSON; the device; the shape; the times. The bearer check includes the
//! device's metrics source: once the operator switches it away from `daemon`, the daemon's role
//! is over and its pushes are `TOKEN_REFUSED` (package ruling, WP-54 T3b). The body is held in a
//! buffer wiped on drop and never logged; a parse error is dropped unread, since its text may
//! quote the body (LOG-007). The token is looked up by its hash only and never reaches an
//! event, a log or an answer.

mod batch;
mod samples;

use std::net::IpAddr;

use async_graphql::ID;
use axum::{
    body::Body,
    extract::Request,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse as _, Response},
};
use base64::Engine as _;
use chrono::{DateTime, Utc};
use daemon_core::{
    readings::Drive,
    wire::{Batch, KeyVersion, PushAccepted, RefusalCode},
};

use super::{
    admission::{Admitted, Stop, admit},
    credential, keys,
    refusals::{self, Route},
    routes::{DaemonState, busy, client_of, read_body, unmounted},
    turns::Turn,
};
use crate::{
    fleet::agent_usage,
    fleet::metrics::{self, Battery, Gpu, MetricSample, MetricsAdded, Readings},
    ids::DeviceId,
};
use batch::{Push, Written};
use samples::Rows;

/// The push body limit (K8 Section 1, MET-017).
const BODY_LIMIT: usize = 512 * 1024;

/// Takes a batch from the daemon of the bearer token: `204` once it is stored, `200` with
/// `Accepted` when the batch had a `keys` member (K-E5 Section 2.1), or the first refusal of K8
/// Section 6, written to the event log without the token.
pub(super) async fn serve_push(state: DaemonState, request: Request) -> Response {
    let Some(client) = client_of(&request) else {
        return unmounted();
    };
    let now = state.clock.now();
    let Some(address) = state.turns.address(client.0) else {
        return busy();
    };
    let (parts, body) = request.into_parts();
    match push(&state, (&parts.headers, body), address, now).await {
        Ok(None) => StatusCode::NO_CONTENT.into_response(),
        Ok(Some(accepted)) => refusals::json(StatusCode::OK, &accepted),
        Err(stop) => state.stopped(stop, (Route::Push, client), now).await,
    }
}

/// A list of the batch as the rows of the store; `None` when the batch did not bring it.
fn converted<T, U: From<T>>(list: Option<Vec<T>>) -> Option<Vec<U>> {
    list.map(|list| list.into_iter().map(U::from).collect())
}

async fn push(
    state: &DaemonState,
    (headers, body): (&HeaderMap, Body),
    address: Turn<IpAddr>,
    now: DateTime<Utc>,
) -> Result<Option<PushAccepted>, Stop> {
    let token =
        credential::bearer(headers).ok_or(Stop::Refused(RefusalCode::TokenRefused, None))?;
    // One push per device at a time: a second one would wait for the device row holding a
    // connection (MET-017). The body arrives under this turn only.
    let Admitted {
        device,
        name,
        turn: _turn,
    } = admit(state, &token, address, now).await?;
    let refused = |code| Stop::Refused(code, Some(device));
    let accepted = {
        let body = read_body(state, headers, body, BODY_LIMIT)
            .await
            .map_err(refused)?;
        let batch =
            serde_json::from_slice::<Batch>(&body).map_err(|_| refused(RefusalCode::Malformed))?;
        batch.check(device.into(), now).map_err(refused)?
    };
    let rows = samples::rows(&accepted.points);
    let daemon_version = accepted.daemon_version;
    let held = accepted.keys;
    let gpus: Option<Vec<Gpu>> = converted(accepted.gpus);
    let drives: Option<Vec<Drive>> = converted(accepted.drives);
    let batteries: Option<Vec<Battery>> = converted(accepted.batteries);
    let wants_salt = accepted
        .agent_usage
        .as_ref()
        .is_some_and(|usage| usage.accounts.is_some());
    let pushed = Push {
        device,
        name: &name,
        token: token.hash(),
        gpus: gpus.as_deref(),
        drives: drives.as_deref(),
        batteries: batteries.as_deref(),
        inventory: accepted.inventory.as_ref(),
        hardware: accepted.hardware.as_ref(),
        daemon_version: daemon_version.as_deref(),
        agent_usage: accepted.agent_usage.as_ref(),
    };
    drop(token);
    let _request = state.turns.request().await.ok_or(Stop::Busy)?;
    match batch::write(&state.db, &state.bus, (pushed, &rows), now).await {
        Ok(Written::Kept(kept)) => {
            if kept > 0 {
                publish(state, device, &rows).await;
            }
            answer(state, device, (held, wants_salt), now).await
        }
        Ok(Written::Refused(code)) => Err(refused(code)),
        Err(error) => Err(Stop::Failed(error)),
    }
}

/// The `200` answer of a stored batch: the keys the daemon holds when it listed them, and the
/// account salt when it listed its accounts; `None` (`204`) when it listed neither.
async fn answer(
    state: &DaemonState,
    device: DeviceId,
    (held, wants_salt): (Option<Vec<KeyVersion>>, bool),
    now: DateTime<Utc>,
) -> Result<Option<PushAccepted>, Stop> {
    let told = match held {
        Some(held) => keys::current(state, device, &held, now).await,
        None => None,
    };
    let account_salt = if wants_salt {
        Some(account_salt(state).await?)
    } else {
        None
    };
    Ok(
        (told.is_some() || account_salt.is_some()).then_some(PushAccepted {
            keys: told,
            update: None,
            account_salt,
        }),
    )
}

/// The account salt of the hub in standard base64, for the daemon to make the keys of its
/// accounts (K8 Section 8.2; AIS-022, R-313).
async fn account_salt(state: &DaemonState) -> Result<String, Stop> {
    let salt = agent_usage::accounts::salt(&state.db)
        .await
        .map_err(|error| Stop::Failed(error.into()))?;
    Ok(base64::engine::general_purpose::STANDARD.encode(salt))
}

/// Publishes the newest point of the batch on the live feed with the device's readings as
/// stored (MET-005). A failed read is logged; the batch is stored and stays accepted.
async fn publish(state: &DaemonState, device: DeviceId, rows: &Rows) {
    let Some(at) = rows.values.iter().map(|value| value.at).max() else {
        return;
    };
    let readings = match metrics::readings(&state.db, &[device], at).await {
        Ok(mut found) => found.remove(&device).unwrap_or_else(|| Readings::empty(at)),
        Err(error) => {
            tracing::error!(
                fault = error.fault(),
                "a daemon batch could not be published"
            );
            return;
        }
    };
    let samples = rows
        .values
        .iter()
        .filter(|value| value.at == at)
        .filter_map(|value| MetricSample::served(&value.sample))
        .collect();
    state.feed.publish(MetricsAdded {
        device_id: ID(device.to_string()),
        at,
        samples,
        readings,
    });
}
