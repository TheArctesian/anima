//! Starting a session (K4 §1, K3 `start`): checks the device and the display, holds the
//! device to its session limit (`limit`), wakes it when it is Down (`waking`), writes the row
//! and its audit entry, and issues the single-use ticket.

mod limit;
mod waking;

use std::{net::IpAddr, sync::Arc};

use chrono::{DateTime, Utc};
use serde_json::json;
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use super::{
    Conflict, Error,
    end::{Actor, end_row, sweep},
    find_device,
    live::Live,
    model::{EndReason, FieldError, Protocol, Session, SessionEnd},
    store::{self, NewSession},
    tickets::{LIFETIME, PendingTicket, TOKEN_BYTES, Ticket, TicketToken},
};
use crate::{
    clock::Clock,
    devices::{Device, RdpSecurity, RdpSettings},
    events::{self, AuditAction, AuditTarget, ClientAddr, NewAudit, Operator},
    ids::{DeviceId, SessionId},
    targets::AddressPolicy,
    trust,
};

pub use waking::Waking;

/// What the operator asks for: a device, a protocol and the view the client will show, and
/// what waking the device needs (`startSession` gives it; `None` sends no packet).
#[derive(Clone, Debug)]
pub struct StartRequest {
    pub device: DeviceId,
    pub protocol: Protocol,
    pub display: DisplayRequest,
    /// What a wake needs; `None` never wakes the device, whatever its status.
    pub wake: Option<Waking>,
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

/// Longest `DisplayInput.timezone` a ticket holds, in bytes: the longest IANA zone name has
/// 32, and `guac_protocol::is_zone_name` allows 64.
const MAX_TIMEZONE_BYTES: usize = 64;

/// The time zone a ticket holds: the requested one when it is a zone name of at most
/// `MAX_TIMEZONE_BYTES`, else none, so the tunnel uses UTC (K4 §3, R-032 E-6). Never a
/// refusal, and a ticket never holds arbitrary client text.
pub(super) fn held_timezone(requested: Option<String>) -> Option<String> {
    requested.filter(|zone| zone.len() <= MAX_TIMEZONE_BYTES && guac_protocol::is_zone_name(zone))
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

/// The host as an IP literal `policy` refuses, as a field error. Host names are checked when
/// the tunnel connects, because they resolve only then.
fn host_problem(host: &str, policy: &AddressPolicy) -> Option<FieldError> {
    let ip = host.parse::<IpAddr>().ok()?.to_canonical();
    (!policy.permits(ip)).then(|| {
        FieldError::new(
            "host",
            "The device's host is a loopback, link-local, multicast or unspecified address, \
             or one in a network the hub refuses (ANIMA_PROBE_DENY). Enter the address the \
             device has on its network.",
        )
    })
}

/// Why `protocol` is off for `device`, if it is (K4 Section 1). The tunnel checks it again
/// when it opens, because the operator may turn the protocol off in between.
pub(super) const fn protocol_off(device: &Device, protocol: Protocol) -> Option<Conflict> {
    match protocol {
        Protocol::Ssh if !device.ssh.enabled => Some(Conflict::SshDisabled),
        Protocol::Rdp if !device.rdp.enabled => Some(Conflict::RdpDisabled),
        Protocol::Vnc if !device.vnc.enabled => Some(Conflict::VncDisabled),
        Protocol::Ssh | Protocol::Rdp | Protocol::Vnc => None,
    }
}

/// Whether an RDP session needs an accepted server certificate: always, unless the device
/// uses standard RDP security (no TLS, so no certificate) or the operator chose to ignore
/// the certificate (K4 Section 1, SEC-007).
pub(super) fn certificate_required(rdp: &RdpSettings) -> bool {
    rdp.security != RdpSecurity::Rdp && !rdp.ignore_certificate
}

/// What the device must have accepted before `protocol` may start: an SSH host key, or an
/// RDP server certificate when [`certificate_required`]; VNC pins nothing (K4 Sections 1
/// and 6).
async fn trust_missing(
    db: &PgPool,
    device: &Device,
    protocol: Protocol,
) -> Result<Option<Conflict>, Error> {
    let missing = match protocol {
        Protocol::Ssh => trust::host_key(db, device.id)
            .await?
            .is_none()
            .then_some(Conflict::NoHostKey),
        Protocol::Rdp if certificate_required(&device.rdp) => {
            let accepted = trust::server_certificates(db, &[device.id]).await?;
            accepted.is_empty().then_some(Conflict::NoServerCertificate)
        }
        Protocol::Rdp | Protocol::Vnc => None,
    };
    Ok(missing)
}

/// The device's state allows a session: display and host acceptable, the protocol on, and
/// what it pins accepted.
async fn admit(
    db: &PgPool,
    live: &Live,
    device: &Device,
    req: &StartRequest,
) -> Result<Display, Error> {
    let display = &req.display;
    let host = host_problem(&device.host, live.policy());
    let display = match (checked_display(display), host) {
        (Ok(display), None) => display,
        (display, host) => {
            let mut fields = display.err().unwrap_or_default();
            fields.extend(host);
            return Err(Error::Validation(fields));
        }
    };
    if let Some(conflict) = protocol_off(device, req.protocol) {
        return Err(Error::Conflict(conflict));
    }
    if let Some(conflict) = trust_missing(db, device, req.protocol).await? {
        return Err(Error::Conflict(conflict));
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
/// K4 §4's bounds or a host that is an IP literal the target rule or the hub's deny list
/// refuses (`Validation`), and the protocol turned off, an SSH session without an accepted
/// host key, an RDP session without an accepted server certificate when it needs one, or a
/// device with as many live sessions as its limit allows (`Conflict`, RA-036). A time zone
/// that is not a zone name is dropped, so the tunnel uses UTC.
/// When `req.wake` says the device is asleep, it has a MAC address and its site has a wake
/// relay, a Wake-on-LAN packet goes first and the ticket holds `wake_sent` when it was sent;
/// a wake that fails leaves `wake_sent` false and the start goes on. Then, in one
/// transaction, it counts the device's live sessions again under the device's start lock, so
/// two starts cannot both pass the limit, writes the live session row and the audit entry
/// `session.start`, and holds a ticket that the protocol's tunnel redeems once within 30 s
/// from `client`. Tickets that expired unredeemed are swept first, and a start the ticket
/// table or the limit would refuse is refused before any packet is sent. Once the row
/// commits, its start is published on `live`'s feed of changes (RA-034); a refused or
/// rolled-back start publishes nothing.
///
/// If the hub shut down while the commit was in flight, `shut_down` took the ticket but could
/// not yet see the row; the start is then not published, the row ends here with
/// `HUB_SHUTDOWN` and its audit entry (that end is published), and the start is
/// `Unavailable`.
///
/// # Errors
/// As above; `Unavailable` when too many tickets are pending or the hub is shutting down;
/// `Database`, `Devices`, `Events` or `Random` when a dependency fails.
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
    let display = admit(db, live, &device, &req).await?;
    sweep(db, live, clock.now()).await?;
    // No packet for a start `issue` would refuse anyway.
    if live.is_full() {
        return Err(Error::Unavailable);
    }
    // Nor for a start the limit refuses; the count that holds is the locked one below.
    limit::check(&mut *db.acquire().await?, device.id).await?;
    let wake_sent = match &req.wake {
        Some(wake) => waking::sent(db, &device, wake, clock.now()).await?,
        None => false,
    };
    // Read after the wake, so the ticket still has its whole lifetime.
    let now = clock.now();
    let token = random_token()?;
    let pending = PendingTicket {
        operator: op.clone(),
        session: SessionId::from(Uuid::now_v7()),
        client,
        device: device.id,
        protocol: req.protocol,
        display,
        timezone: held_timezone(req.display.timezone),
        expires_at: now + LIFETIME,
        wake_sent,
    };
    let mut tx = db.begin().await?;
    limit::check_locked(&mut tx, device.id).await?;
    let started = write_start(&mut tx, &device, &pending, now).await?;
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
    if !live.publish_start(started) {
        end_shut_down(db, live, &pending, now).await?;
        return Err(Error::Unavailable);
    }
    let mut ticket = Ticket::new(pending.session, pending.protocol, token, pending.expires_at);
    ticket.wake_sent = pending.wake_sent;
    Ok(ticket)
}

/// Ends the committed row of a ticket `shut_down` withdrew before the row was visible to it,
/// with `HUB_SHUTDOWN` and its audit entry naming the ticket's operator and client, then
/// publishes that end. A row `shut_down` did end is left as it is.
async fn end_shut_down(
    db: &PgPool,
    live: &Live,
    pending: &PendingTicket,
    now: DateTime<Utc>,
) -> Result<(), Error> {
    let end = SessionEnd {
        at: now,
        reason: EndReason::HubShutdown,
    };
    let by = Actor {
        operator: pending.operator.clone(),
        client: pending.client,
        at: now,
    };
    let mut tx = db.begin().await?;
    let ended = end_row(&mut tx, pending.session, end, &by).await?;
    tx.commit().await?;
    if let Some(row) = ended {
        live.publish_end(row);
    }
    Ok(())
}

/// The live row, returned as written, and the audit entry `session.start`; the change never
/// holds the token.
async fn write_start(
    conn: &mut PgConnection,
    device: &Device,
    pending: &PendingTicket,
    now: DateTime<Utc>,
) -> Result<Session, Error> {
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
    let started = store::insert(&mut *conn, &new).await?;
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
    Ok(started)
}
