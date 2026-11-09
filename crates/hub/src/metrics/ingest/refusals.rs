//! How the agent routes answer a refusal and write it to the event log (K8 Section 6, CR-15).
//!
//! The body is `{"error":"<CODE>"}` and nothing else; the event names the code, the client
//! address and, when the credential was valid, the device. Neither ever holds a value from the
//! request: no code, token or body text (LOG-007).

use std::sync::{Arc, Mutex, PoisonError};

use agent_core::wire::{Refusal, RefusalCode};
use axum::{
    http::{StatusCode, header},
    response::{IntoResponse as _, Response},
};
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;

use super::{Error, throttle::Throttle};
use crate::{
    devices,
    events::{self, Bus, ClientAddr, NewEvent, Severity, Source},
    ids::DeviceId,
};

/// The route a refusal answered, as the event names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Route {
    Enrol,
    Renew,
}

impl Route {
    const fn noun(self) -> &'static str {
        match self {
            Self::Enrol => "an agent enrolment",
            Self::Renew => "an agent token renewal",
        }
    }
}

/// One refusal to answer and record.
#[derive(Clone, Copy, Debug)]
pub(super) struct Refused {
    pub(super) route: Route,
    pub(super) code: RefusalCode,
    pub(super) client: ClientAddr,
    /// The device of a valid credential; `None` when the request had none.
    pub(super) device: Option<DeviceId>,
}

/// A JSON answer that no cache keeps, since `Enrolled` and `Renewed` hold a token.
pub(super) fn json(status: StatusCode, body: &impl Serialize) -> Response {
    let Ok(bytes) = serde_json::to_vec(body) else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };
    let headers = [
        (header::CONTENT_TYPE, "application/json"),
        (header::CACHE_CONTROL, "no-store"),
    ];
    (status, headers, bytes).into_response()
}

/// `{"error":"<CODE>"}` with the code's status.
pub(super) fn answer(code: RefusalCode) -> Response {
    let status = StatusCode::from_u16(code.status()).unwrap_or(StatusCode::BAD_REQUEST);
    json(status, &Refusal::from(code))
}

/// The event's sentence: what was refused, from where, and why by code.
pub(super) fn sentence(refused: &Refused) -> String {
    format!(
        "Refused {} from {}: {}.",
        refused.route.noun(),
        refused.client.0,
        refused.code.as_str()
    )
}

/// The sentence of the refusals the throttle left out.
pub(super) fn unrecorded_sentence(count: usize) -> String {
    format!("Left out {count} more agent refusals from other addresses in the last minute.")
}

/// The refusal events of the agent routes, written at most once a minute per client address
/// when the request had no valid credential.
#[derive(Clone)]
pub(super) struct Recorder {
    db: PgPool,
    bus: Bus,
    throttle: Arc<Mutex<Throttle>>,
}

impl Recorder {
    pub(super) fn new(db: PgPool, bus: Bus) -> Self {
        Self {
            db,
            bus,
            throttle: Arc::default(),
        }
    }

    /// The events `refused` at `now` writes: its own unless the throttle holds it back, and the
    /// count of refusals left out once a minute has passed since the first of them.
    fn due(&self, refused: &Refused, now: DateTime<Utc>) -> (bool, Option<usize>) {
        let mut throttle = self.throttle.lock().unwrap_or_else(PoisonError::into_inner);
        let own = refused.device.is_some() || throttle.admit(refused.client.0, now);
        (own, throttle.unrecorded(now).filter(|count| *count > 0))
    }

    /// Writes the refusal's events, as warnings from metrics.
    ///
    /// # Errors
    /// `Database`, `Devices` or `Events` when the device or a write fails.
    pub(super) async fn record(&self, refused: &Refused, now: DateTime<Utc>) -> Result<(), Error> {
        let (own, unrecorded) = self.due(refused, now);
        if own {
            let device = match refused.device {
                Some(id) => self.named(id).await?,
                None => None,
            };
            self.write(device, sentence(refused), now).await?;
        }
        if let Some(count) = unrecorded {
            self.write(None, unrecorded_sentence(count), now).await?;
        }
        Ok(())
    }

    /// The device with its name; `None` when it was deleted meanwhile.
    async fn named(&self, id: DeviceId) -> Result<Option<(DeviceId, String)>, Error> {
        match devices::get(&self.db, id).await {
            Ok(device) => Ok(Some((id, device.name))),
            Err(devices::Error::NotFound(_)) => Ok(None),
            Err(other) => Err(other.into()),
        }
    }

    async fn write(
        &self,
        device: Option<(DeviceId, String)>,
        message: String,
        now: DateTime<Utc>,
    ) -> Result<(), Error> {
        let event = NewEvent {
            severity: Severity::Warning,
            source: Source::Metrics,
            device,
            message,
        };
        let mut conn = self.db.acquire().await?;
        events::record(&mut conn, &self.bus, now, event).await?;
        Ok(())
    }
}
