//! An agent's enrolment: the operator makes a one-use code, the agent swaps it for a token, and
//! the operator may revoke that token (K8 Section 3, MET-011, MET-012, MET-013).

use agent_core::wire::{AgentToken, CodeHash, Enrolled, EnrolmentCode, is_device_name};
use async_graphql::Object;
use chrono::{DateTime, Utc};
use sqlx::{PgConnection, PgPool};

use super::{CODE_LIFETIME, Error, TOKEN_LIFETIME, store};
use crate::{
    devices,
    events::{self, Bus, NewEvent, Severity, Source},
    ids::DeviceId,
};

/// A new enrolment code, shown once (K1 `AgentEnrolment`). The code is wiped on drop and is
/// read only by the `code` field (LOG-007).
pub struct AgentEnrolment {
    code: EnrolmentCode,
    expires_at: DateTime<Utc>,
}

#[Object]
impl AgentEnrolment {
    async fn code(&self) -> &str {
        self.code.expose()
    }

    async fn expires_at(&self) -> DateTime<Utc> {
        self.expires_at
    }
}

impl AgentEnrolment {
    /// When the code stops working.
    pub(in crate::metrics) const fn expiry(&self) -> DateTime<Utc> {
        self.expires_at
    }
}

/// Stores the hash of `code`, made from the OS random source, as the device's one working code
/// until 15 minutes after `now`, in the caller's transaction; the device's earlier unused codes
/// stop working (MET-012, K8 Section 3).
///
/// # Errors
/// `NoDevice` when the device was deleted meanwhile; `Database` when a write fails.
pub(in crate::metrics) async fn make_code(
    conn: &mut PgConnection,
    device: DeviceId,
    code: EnrolmentCode,
    now: DateTime<Utc>,
) -> Result<AgentEnrolment, Error> {
    let expires_at = now + CODE_LIFETIME;
    // The device row first, the order a device delete takes the rows in; a second code for
    // the device waits here, then voids this one.
    devices::lock(conn, device).await?;
    store::void_unused_codes(conn, device).await?;
    store::insert_code(conn, code.hash(), device, expires_at).await?;
    Ok(AgentEnrolment { code, expires_at })
}

/// A code marked used, whose exchange for a token is still to come.
#[derive(Clone, Copy)]
pub(super) struct Redeemed {
    device: DeviceId,
    code: CodeHash,
}

/// Marks `code` used at `now`, committed before anything else happens; `None` for a code that
/// is unknown, used or expired. A second use fails even when the exchange that used it first
/// fails later (MET-012).
///
/// # Errors
/// `Database` when the mark fails; the code is then unchanged.
pub(super) async fn redeem(
    db: &PgPool,
    code: &EnrolmentCode,
    now: DateTime<Utc>,
) -> Result<Option<Redeemed>, Error> {
    let hash = code.hash();
    let device = store::use_code(db, hash, now).await?;
    Ok(device.map(|device| Redeemed { device, code: hash }))
}

/// Makes `token` the device's only token for 365 days from `now`, sets the device's metrics
/// source to `agent` and writes one event naming the device, together (MET-011, MET-013);
/// `None`, storing nothing, when the operator revoked the device's agent after the code was
/// redeemed, or when the device's stored name is not one the agent keeps (K8 Section 3).
///
/// # Errors
/// `NoDevice` when the device was deleted after its code was redeemed; `Database` or `Events`
/// when a write fails. Nothing is stored then.
pub(super) async fn enrol(
    db: &PgPool,
    bus: &Bus,
    (redeemed, token): (Redeemed, AgentToken),
    now: DateTime<Utc>,
) -> Result<Option<Enrolled>, Error> {
    let Redeemed { device, code } = redeemed;
    let expires_at = now + TOKEN_LIFETIME;
    let mut tx = db.begin().await?;
    // The device row first, the order a device delete and a revoke take the rows in; a revoke
    // that committed before this lock deleted the code.
    let name = devices::use_agent_metrics(&mut tx, device).await?;
    // A name the agent would not keep (a row older than the device rule) changes nothing: the
    // transaction rolls back with the source and token as they were (K8 Section 3).
    if !is_device_name(&name) || !store::code_kept(&mut tx, code, device).await? {
        return Ok(None);
    }
    store::replace_token(&mut tx, device, token.hash(), expires_at).await?;
    let event = NewEvent {
        severity: Severity::Info,
        source: Source::Metrics,
        device: Some((device, name.clone())),
        message: format!("The agent on {name} enrolled; its metrics now come from the agent."),
    };
    events::record(&mut tx, bus, now, event).await?;
    tx.commit().await?;
    Ok(Some(Enrolled {
        device: device.into(),
        device_name: name,
        token,
        expires_at,
    }))
}

/// Deletes the device's token and its codes in the caller's transaction; the next push or
/// renewal with that token is refused, and so is an exchange whose code was already redeemed.
/// The device row is locked first, so an exchange in flight either commits before the revoke,
/// which then deletes its token, or finds its code gone. The metrics source stays `agent`
/// until the operator changes it (MET-013, R-092 D-7). Whether the device had a token.
///
/// # Errors
/// `NoDevice` when the device was deleted meanwhile; `Database` when a delete fails.
pub(in crate::metrics) async fn revoke(
    conn: &mut PgConnection,
    device: DeviceId,
) -> Result<bool, Error> {
    devices::lock(conn, device).await?;
    Ok(store::revoke(conn, device).await?)
}
