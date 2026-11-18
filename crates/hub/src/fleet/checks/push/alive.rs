//! The check that the hub reports for itself (HMD-045): a PUSH check on the hub's own device,
//! which the hub reports as Up while it runs. It is a PUSH check as any other: when the hub
//! stops reporting, the scheduler of the next hub that runs records the missed push.

use std::time::Duration;

use chrono::{DateTime, Utc};
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use super::{Report, report_in};
use crate::{
    fleet::{
        checks::{Error, config::CheckConfig, fresh_push_token, store},
        devices,
        events::Bus,
    },
    ids::{CheckId, DeviceId},
};

/// The name of the check. The check is found by it.
const NAME: &str = "Hub running";

/// Makes sure that `device` has the PUSH check `Hub running`, in the caller's transaction:
/// the one that exists, else a new one with the stored defaults and a new token.
///
/// # Errors
/// `Validation` (`deviceId`) when there is no such device; `Random` when no token can be made.
pub async fn ensure_alive_check(
    conn: &mut PgConnection,
    device: DeviceId,
) -> Result<CheckId, Error> {
    if let Some(id) = store::push_check_named(&mut *conn, device, NAME).await? {
        return Ok(id);
    }
    let settings = store::read_defaults(&mut *conn).await?.settings();
    let token = fresh_push_token()?;
    let row = store::Insert {
        device,
        parent: None,
        name: NAME,
        config: &CheckConfig::Push,
        settings,
        push_token: Some(&token),
        sealed: None,
    };
    let id = CheckId::from(Uuid::now_v7());
    Ok(store::insert(conn, id, row).await?.id)
}

/// Reports the check as Up at `now`, as a push of its device does, and returns the interval
/// in which the check waits for the next report. `None` when the check or its device is
/// gone: nothing is written, and the next start of the hub adds them again. A paused check
/// records nothing and keeps its interval.
///
/// # Errors
/// `Database`, `Devices` or `Maintenance` when a read or the write fails.
pub async fn beat_alive(
    db: &PgPool,
    bus: &Bus,
    check: CheckId,
    now: DateTime<Utc>,
) -> Result<Option<Duration>, Error> {
    let check = match store::fetch(db, check).await {
        Ok(check) => check,
        Err(Error::NotFound(_)) => return Ok(None),
        Err(error) => return Err(error),
    };
    let up = Report {
        up: true,
        message: None,
        ping_ms: None,
    };
    match report_in((db, bus), &check, up, now).await {
        Ok(()) => {}
        Err(Error::NotFound(_) | Error::Devices(devices::Error::NotFound(_))) => return Ok(None),
        Err(error) => return Err(error),
    }
    let interval = u64::from(check.settings.interval_seconds);
    Ok(Some(Duration::from_secs(interval)))
}
