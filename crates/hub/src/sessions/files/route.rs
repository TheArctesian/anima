//! `GET /files` (K4 Sections 8.1 and 9, API-009): every refusal happens before the WebSocket
//! upgrade, with an HTTP status and an empty body.
//!
//! 1. The token from the query and the client address, then the files ticket's single-use
//!    redemption (403). The token is spent before anything else is checked (K4 Section 1).
//! 2. The ticket's SSH session must still have its tunnel open, and the hub must not be
//!    shutting down (403). The relay then registers with `Live`, so hub shutdown waits for it
//!    to write the audit entry of a transfer it cuts short.
//! 3. The subprotocol `anima-files` (400).
//! 4. The device side: the device, a link to it (pinned key, stored credential, deny list,
//!    P2-003, P2-004), the `sftp` subsystem and the SFTP version exchange, unless the session
//!    ends first (503). A changed host key also writes an Error event (SEC-007).
//!
//! No session row belongs to the files tunnel, so this handler, not a task, does the opening:
//! the client leaving drops it and with it the link. Neither the URI, the query nor the token
//! reaches a log, a span or an error (LOG-007).

use std::{sync::Arc, time::Duration};

use axum::{
    Router,
    extract::{RawQuery, State, WebSocketUpgrade, ws::rejection::WebSocketUpgradeRejection},
    http::{Extensions, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
};
use sqlx::PgPool;
use tokio::time::{Instant, timeout};
use zeroize::Zeroizing;

use super::{
    device::{self, Broken},
    relay::{Relay, SessionEnd, after},
    tickets::FilesTicket,
    trail::{self, Trail},
};
use crate::{
    access,
    clock::Clock,
    credentials::Vault,
    devices::{self, Device},
    events::Bus,
    sessions::{
        Live, admission, end::CLIENT_IDLE, live::FilesRelay, quiet::Quiet, tickets::FILES_PATH,
    },
    ssh::{LinkError, SshLink, link},
    targets::AddressPolicy,
};

/// The WebSocket subprotocol the client asks for (K4 Section 9).
const SUBPROTOCOL: &str = "anima-files";
/// The device side's whole opening: the link's own 10 s and the subsystem with SFTP's start.
const OPENING: Duration = Duration::from_secs(15);
/// The hub sends `ping` when it has sent the client nothing for this long (K4 Section 8.4).
const HUB_SILENCE: Duration = Duration::from_secs(5);
/// The longest one request to the device may wait (K4 Section 8.5).
const DEVICE_WAIT: Duration = Duration::from_secs(15);
/// An idle tunnel sends the device a request this often, inside the link's 60 s idle end.
const KEEP_ALIVE: Duration = Duration::from_secs(30);
/// Most entries in one `entries` message (K4 Section 9); the rest of a directory is not read.
const MAX_ENTRIES: usize = 10_000;

/// The waits of one files tunnel. Tests shorten them; the hub uses `Timing::default()`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Timing {
    pub(super) opening: Duration,
    pub(super) hub_silence: Duration,
    /// Client silence that ends the tunnel, not counting time spent waiting on the device;
    /// also the longest one send to the client may take.
    pub(super) client_idle: Duration,
    pub(super) device: Duration,
    pub(super) keep_alive: Duration,
}

impl Default for Timing {
    fn default() -> Self {
        Self {
            opening: OPENING,
            hub_silence: HUB_SILENCE,
            client_idle: CLIENT_IDLE,
            device: DEVICE_WAIT,
            keep_alive: KEEP_ALIVE,
        }
    }
}

/// What the files route needs: the hub's stores, the live state, the deny list and the waits.
#[derive(Clone)]
pub(super) struct Files {
    pub(super) db: PgPool,
    pub(super) vault: Vault,
    pub(super) bus: Bus,
    pub(super) clock: Arc<dyn Clock>,
    pub(super) live: Live,
    pub(super) policy: AddressPolicy,
    pub(super) timing: Timing,
    pub(super) max_entries: usize,
}

/// The `/files` route (K3 with CR-1's `policy`, K4 Section 9). `main.rs` mounts it behind
/// `access::permitted_networks_layer` (NET-002) and serves it with `ConnectInfo<SocketAddr>`.
#[expect(
    clippy::too_many_arguments,
    reason = "K3 fixes files_router's parameters; CR-1 adds the deny list"
)]
pub fn files_router(
    db: PgPool,
    vault: Vault,
    bus: Bus,
    clock: Arc<dyn Clock>,
    live: Live,
    policy: AddressPolicy,
) -> Router {
    router(Files {
        db,
        vault,
        bus,
        clock,
        live,
        policy,
        timing: Timing::default(),
        max_entries: MAX_ENTRIES,
    })
}

/// The route over `files`' dependencies and waits.
pub(super) fn router(files: Files) -> Router {
    Router::new()
        .route(FILES_PATH, get(open))
        .with_state(Arc::new(files))
}

/// The device side of an opened tunnel. The link must outlive the SFTP session over it.
struct Opened {
    link: SshLink,
    remote: device::Remote,
    device_name: String,
}

/// Why the device side did not open. Holds no host, address or device text (LOG-007).
#[derive(Debug, thiserror::Error)]
enum NotOpened {
    #[error("the device is gone")]
    Gone,
    #[error("device store error")]
    Devices,
    #[error(transparent)]
    Link(LinkError),
    #[error(transparent)]
    Sftp(Broken),
    #[error("the device side did not open in time")]
    Timeout,
}

async fn open(
    State(files): State<Arc<Files>>,
    RawQuery(query): RawQuery,
    extensions: Extensions,
    upgrade: Result<WebSocketUpgrade, WebSocketUpgradeRejection>,
) -> Response {
    let query = Zeroizing::new(query.unwrap_or_default());
    let now = files.clock.now();
    let token = admission::token_in(&query);
    let (Some(token), Some(client)) = (token, access::client_addr(&extensions)) else {
        return StatusCode::FORBIDDEN.into_response();
    };
    let Ok(ticket) = files.live.files().redeem(token, client, now) else {
        return StatusCode::FORBIDDEN.into_response();
    };
    drop(query);
    let Some((signal, registration)) = files.live.files_relay(ticket.session) else {
        return StatusCode::FORBIDDEN.into_response();
    };
    let mut session = SessionEnd::new(signal);
    let Some(upgrade) = admission::offering(upgrade, SUBPROTOCOL) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let opened = tokio::select! {
        opened = timeout(files.timing.opening, opened(&files, &ticket)) => {
            opened.unwrap_or(Err(NotOpened::Timeout))
        }
        _ended = session.ended() => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
    };
    let opened = match opened {
        Ok(opened) => opened,
        Err(not_opened) => {
            let session = ticket.session;
            tracing::warn!(%session, error = %not_opened, "the files tunnel did not open");
            return StatusCode::SERVICE_UNAVAILABLE.into_response();
        }
    };
    let admitted = Admitted {
        ticket,
        session,
        registration,
    };
    upgrade.on_upgrade(move |client| relay(client, files, admitted, opened))
}

/// A link to `device`. A link refused because the device presented another host key, or
/// because its stored credential was entered for another host (SEC-011), also writes the
/// event that says so; no credential was sent either way.
async fn linked(files: &Files, device: &Device) -> Result<SshLink, NotOpened> {
    let now = files.clock.now();
    let error = match link(&files.db, &files.vault, &files.policy, device, now).await {
        Ok(link) => return Ok(link),
        Err(error) => error,
    };
    let written = match error {
        LinkError::HostKeyChanged => {
            trail::host_key_changed(&files.db, &files.bus, now, device).await
        }
        LinkError::CredentialForOtherHost => {
            trail::credential_moved(&files.db, &files.bus, now, device).await
        }
        _ => Ok(()),
    };
    if let Err(error) = written {
        tracing::error!(%error, "the event of a refused files tunnel could not be written");
    }
    Err(NotOpened::Link(error))
}

/// The device, a link to it, its `sftp` subsystem and the SFTP session over it.
async fn opened(files: &Files, ticket: &FilesTicket) -> Result<Opened, NotOpened> {
    let device = devices::get(&files.db, ticket.device)
        .await
        .map_err(|error| match error {
            devices::Error::NotFound(_) => NotOpened::Gone,
            _ => NotOpened::Devices,
        })?;
    let link = linked(files, &device).await?;
    let stream = link
        .sftp(files.timing.device)
        .await
        .map_err(NotOpened::Link)?;
    let remote = device::start(stream, files.timing.device)
        .await
        .map_err(NotOpened::Sftp)?;
    Ok(Opened {
        link,
        remote,
        device_name: device.name,
    })
}

/// A redeemed files ticket, its SSH session's end signal and the relay's registration.
struct Admitted {
    ticket: FilesTicket,
    session: SessionEnd,
    registration: FilesRelay,
}

/// Runs the upgraded tunnel to its end, then leaves the device.
async fn relay(
    client: axum::extract::ws::WebSocket,
    files: Arc<Files>,
    admitted: Admitted,
    opened: Opened,
) {
    let Admitted {
        ticket,
        session,
        registration,
    } = admitted;
    let Opened {
        link,
        remote,
        device_name,
    } = opened;
    let timing = files.timing;
    let trail = Trail {
        db: files.db.clone(),
        bus: files.bus.clone(),
        clock: Arc::clone(&files.clock),
        ticket,
        device_name,
    };
    Relay {
        client,
        remote: Arc::new(remote),
        session,
        trail,
        timing,
        max_entries: files.max_entries,
        work: None,
        active: None,
        hub_quiet: Quiet::new(timing.hub_silence, Instant::now()),
        client_quiet: Quiet::new(timing.client_idle, Instant::now()),
        work_due: after(timing.device),
        device_quiet: Quiet::new(timing.keep_alive, Instant::now()),
    }
    .run()
    .await;
    // Dropping the link ends the SSH connection.
    drop(link);
    // Only now, with any transfer's audit entry written, may shutdown stop waiting.
    drop(registration);
}
