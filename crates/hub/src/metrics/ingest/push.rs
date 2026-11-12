//! `POST /agent/v1/push` (K8 Sections 4 and 6; MET-015, MET-017, MET-018).
//!
//! The checks run in K8's order and stop at the first refusal: the bearer, before the body is
//! read; the size; the JSON; the device; the shape; the times. The bearer check includes the
//! device's metrics source: once the operator switches it away from `agent`, the agent's role
//! is over and its pushes are `TOKEN_REFUSED` (package ruling, WP-54 T3b). The body is held in a
//! buffer wiped on drop and never logged; a parse error is dropped unread, since its text may
//! quote the body (LOG-007). The token is looked up by its hash only and never reaches an
//! event, a log or an answer.

mod batch;
mod samples;

use std::net::IpAddr;

use agent_core::{
    readings::Drive,
    wire::{Batch, RefusalCode},
};
use async_graphql::ID;
use axum::{
    body::Body,
    extract::Request,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse as _, Response},
};
use chrono::{DateTime, Utc};

use super::{
    admission::{Admitted, Stop, admit},
    credential,
    refusals::Route,
    routes::{AgentState, busy, client_of, read_body, unmounted},
    turns::Turn,
};
use crate::{
    ids::DeviceId,
    metrics::{self, Gpu, MetricSample, MetricsAdded, Readings},
};
use batch::{Push, Written};
use samples::Rows;

/// The push body limit (K8 Section 1, MET-017).
const BODY_LIMIT: usize = 512 * 1024;

/// Takes a batch from the agent of the bearer token: `204` once it is stored, or the first
/// refusal of K8 Section 6, written to the event log without the token.
pub(super) async fn serve_push(state: AgentState, request: Request) -> Response {
    let Some(client) = client_of(&request) else {
        return unmounted();
    };
    let now = state.clock.now();
    let Some(address) = state.turns.address(client.0) else {
        return busy();
    };
    let (parts, body) = request.into_parts();
    match push(&state, (&parts.headers, body), address, now).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(stop) => state.stopped(stop, (Route::Push, client), now).await,
    }
}

async fn push(
    state: &AgentState,
    (headers, body): (&HeaderMap, Body),
    address: Turn<IpAddr>,
    now: DateTime<Utc>,
) -> Result<(), Stop> {
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
    let gpus: Option<Vec<Gpu>> = accepted
        .gpus
        .map(|gpus| gpus.into_iter().map(Gpu::from).collect());
    let drives: Option<Vec<Drive>> = accepted
        .drives
        .map(|drives| drives.into_iter().map(Drive::from).collect());
    let pushed = Push {
        device,
        name: &name,
        token: token.hash(),
        gpus: gpus.as_deref(),
        drives: drives.as_deref(),
    };
    drop(token);
    let _request = state.turns.request().await.ok_or(Stop::Busy)?;
    match batch::write(&state.db, &state.bus, (pushed, &rows), now).await {
        Ok(Written::Kept(kept)) => {
            if kept > 0 {
                publish(state, device, &rows).await;
            }
            Ok(())
        }
        Ok(Written::Refused(code)) => Err(refused(code)),
        Err(error) => Err(Stop::Failed(error)),
    }
}

/// Publishes the newest point of the batch on the live feed with the device's readings as
/// stored (MET-005). A failed read is logged; the batch is stored and stays accepted.
async fn publish(state: &AgentState, device: DeviceId, rows: &Rows) {
    let Some(at) = rows.values.iter().map(|value| value.at).max() else {
        return;
    };
    let readings = match metrics::readings(&state.db, &[device], at).await {
        Ok(mut found) => found.remove(&device).unwrap_or_else(|| Readings::empty(at)),
        Err(error) => {
            tracing::error!(
                fault = error.fault(),
                "an agent batch could not be published"
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
