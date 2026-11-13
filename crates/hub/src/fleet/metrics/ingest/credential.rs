//! The agent token a request presents, and the device it speaks for (K8 Sections 2, 4 and 5,
//! MET-013). Renewal and push both authenticate here, and both ask whether the device still
//! takes its metrics from the agent (CR-18).

use agent_core::wire::AgentToken;
use axum::http::{HeaderMap, header::AUTHORIZATION};
use chrono::{DateTime, Utc};
use sqlx::PgPool;

use super::{Error, store};
use crate::{
    fleet::devices::{self, MetricsSource},
    ids::DeviceId,
};

/// The agent token of the one `Authorization` header with the `Bearer` scheme (any case);
/// `None` when the header is missing or repeated, for another scheme, and for text that is not
/// a canonical agent token, so no lookup runs for junk.
pub(super) fn bearer(headers: &HeaderMap) -> Option<AgentToken> {
    let mut values = headers.get_all(AUTHORIZATION).iter();
    let value = values.next()?;
    if values.next().is_some() {
        return None;
    }
    let (scheme, token) = value.to_str().ok()?.split_once(' ')?;
    if !scheme.eq_ignore_ascii_case("bearer") {
        return None;
    }
    AgentToken::parse(token.trim())
}

/// The device `presented` is the usable token of at `now`: the current token before its expiry,
/// or the renewed one before its expiry, whose first use here promotes it and ends the current
/// one (MET-013). `None` for a token that is unknown, expired, replaced or revoked.
///
/// # Errors
/// `Database` when the lookup fails.
pub(super) async fn authenticate(
    db: &PgPool,
    presented: &AgentToken,
    now: DateTime<Utc>,
) -> Result<Option<DeviceId>, Error> {
    Ok(store::holder(db, presented.hash(), now).await?)
}

/// The device's name while it takes its metrics from the agent; `None` once the operator
/// switched it away or deleted it, when its token speaks for nothing (CR-18, K8 Section 6).
///
/// # Errors
/// `Devices` when the device cannot be read.
pub(super) async fn agent_name(db: &PgPool, device: DeviceId) -> Result<Option<String>, Error> {
    match devices::get(db, device).await {
        Ok(found) if found.metrics_source == MetricsSource::Agent => Ok(Some(found.name)),
        Ok(_) | Err(devices::Error::NotFound(_)) => Ok(None),
        Err(other) => Err(Error::Devices(other)),
    }
}
