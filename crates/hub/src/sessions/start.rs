//! Starting a session (K4 §1, K3 `start`): checks the device and the display, writes the row
//! and its audit entry, and issues the single-use ticket.

use std::{net::IpAddr, sync::Arc};

use chrono::{DateTime, Utc};
use serde_json::json;
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use super::{
    Conflict, Error,
    end::sweep,
    find_device, host_key,
    live::Live,
    model::{FieldError, Protocol},
    store::{self, NewSession},
    target::is_refused_target,
    tickets::{LIFETIME, PendingTicket, TOKEN_BYTES, Ticket, TicketToken},
};
use crate::{
    clock::Clock,
    devices::Device,
    events::{self, AuditAction, AuditTarget, ClientAddr, NewAudit, Operator},
    ids::{DeviceId, SessionId},
};

/// What the operator asks for: a device, a protocol and the view the client will show.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StartRequest {
    pub device: DeviceId,
    pub protocol: Protocol,
    pub display: DisplayRequest,
}

/// The client's view as it reports it (K1 `DisplayInput`); `start` checks the bounds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DisplayRequest {
    pub width_px: i64,
    pub height_px: i64,
    pub dpi: i64,
    pub timezone: Option<String>,
}

/// K1's default for `DisplayInput.dpi`.
pub const DEFAULT_DPI: i32 = 96;

type Display = guac_protocol::Display;

/// One side or the DPI as `u32` when within `min..=max`, else the field's error.
fn bounded(value: i64, (min, max): (u32, u32), field: &'static str) -> Result<u32, FieldError> {
    u32::try_from(value)
        .ok()
        .filter(|value| (min..=max).contains(value))
        .ok_or_else(|| FieldError::new(field, format!("Enter a value from {min} to {max}.")))
}

/// The display within K4 §4's bounds, else an error for each field outside them.
pub(super) fn checked_display(display: &DisplayRequest) -> Result<Display, Vec<FieldError>> {
    let side = (Display::MIN_SIDE_PX, Display::MAX_SIDE_PX);
    let width = bounded(display.width_px, side, "display.widthPx");
    let height = bounded(display.height_px, side, "display.heightPx");
    let dpi = bounded(
        display.dpi,
        (Display::MIN_DPI, Display::MAX_DPI),
        "display.dpi",
    );
    match (width, height, dpi) {
        (Ok(width), Ok(height), Ok(dpi)) => Ok(Display { width, height, dpi }),
        (width, height, dpi) => Err([width.err(), height.err(), dpi.err()]
            .into_iter()
            .flatten()
            .collect()),
    }
}

/// The host as an IP literal the gateway must not reach, as a field error. Host names are
/// checked when the tunnel connects (T5), because they resolve only then.
fn host_problem(host: &str) -> Option<FieldError> {
    let ip = host.parse::<IpAddr>().ok()?;
    is_refused_target(ip).then(|| {
        FieldError::new(
            "host",
            "The device's host is a loopback, link-local, multicast or unspecified address. \
             Enter the address the device has on its network.",
        )
    })
}

/// The device's state allows a session: display and host acceptable, SSH on, a key accepted.
async fn admit(db: &PgPool, device: &Device, display: &DisplayRequest) -> Result<Display, Error> {
    let host = host_problem(&device.host);
    let display = match (checked_display(display), host) {
        (Ok(display), None) => display,
        (display, host) => {
            let mut fields = display.err().unwrap_or_default();
            fields.extend(host);
            return Err(Error::Validation(fields));
        }
    };
    if !device.ssh.enabled {
        return Err(Error::Conflict(Conflict::SshDisabled));
    }
    if host_key(db, device.id).await?.is_none() {
        return Err(Error::Conflict(Conflict::NoHostKey));
    }
    Ok(display)
}

/// Random bytes from the operating system.
fn random_token() -> Result<TicketToken, Error> {
    let mut bytes = [0; TOKEN_BYTES];
    getrandom::fill(&mut bytes).map_err(Error::Random)?;
    Ok(TicketToken::from_random(bytes))
}

/// Starts a session (K3, K4 §1): refuses a missing device (`NotFound`), a display outside
/// K4 §4's bounds or a host that is a loopback, link-local, multicast or unspecified IP
/// literal (`Validation`), and SSH turned off or no accepted host key (`Conflict`). Then, in
/// one transaction, it writes the live session row and the audit entry `session.start`, and
/// holds a ticket that `/tunnel` redeems once within 30 s from `client`. Tickets that expired
/// unredeemed are swept first.
///
/// # Errors
/// As above; `Unavailable` when too many tickets are pending or the hub is shutting down;
/// `Database`, `Devices` or `Random` when a dependency fails.
#[expect(
    clippy::too_many_arguments,
    reason = "K3 fixes start's parameters (R-032 E-1)"
)]
pub async fn start(
    db: &PgPool,
    clock: &Arc<dyn Clock>,
    live: &Live,
    op: &Operator,
    client: ClientAddr,
    req: StartRequest,
) -> Result<Ticket, Error> {
    let device = find_device(db, req.device).await?;
    let display = admit(db, &device, &req.display).await?;
    let now = clock.now();
    sweep(db, live, now).await?;
    let token = random_token()?;
    let pending = PendingTicket {
        operator: op.clone(),
        session: SessionId::from(Uuid::now_v7()),
        client,
        device: device.id,
        protocol: req.protocol,
        display,
        timezone: req.display.timezone,
        expires_at: now + LIFETIME,
    };
    let mut tx = db.begin().await?;
    write_start(&mut tx, &device, &pending, now).await?;
    live.issue(token.hash(), pending.clone())
        .map_err(|_| Error::Unavailable)?;
    if let Err(error) = tx.commit().await {
        // The commit may have reached the database before it failed, leaving a live row.
        // The ticket goes back without its token: never redeemable, and the sweep ends any
        // such row `TIMEOUT` at expiry (a missing row is a no-op for `end_row`).
        if let Some(ticket) = live.withdraw(pending.session) {
            live.restore(vec![ticket]);
        }
        return Err(error.into());
    }
    Ok(Ticket::new(pending.session, token, pending.expires_at))
}

/// The live row and the audit entry `session.start`; the change never holds the token.
async fn write_start(
    conn: &mut PgConnection,
    device: &Device,
    pending: &PendingTicket,
    now: DateTime<Utc>,
) -> Result<(), Error> {
    let protocol = pending.protocol;
    let new = NewSession {
        id: pending.session,
        device: device.id,
        device_name: &device.name,
        protocol,
        operator: &pending.operator,
        client: pending.client.0,
        started_at: now,
    };
    store::insert(&mut *conn, &new).await?;
    let change = json!({
        "deviceId": device.id.to_string(),
        "deviceName": device.name,
        "protocol": protocol.as_str(),
    });
    let audit = NewAudit {
        operator: pending.operator.clone(),
        action: AuditAction::Start,
        target: AuditTarget::Session(pending.session),
        client: pending.client,
        change,
    };
    events::audit(conn, now, audit).await?;
    Ok(())
}
