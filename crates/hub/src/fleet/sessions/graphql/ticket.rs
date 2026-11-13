//! `startSession`'s input and output as K1 describes them (`StartSessionInput`,
//! `DisplayInput`, `SessionTicket`; API-009), and what the start needs to wake the device.

use async_graphql::{Context, ID, InputObject, Object, Result, dataloader::DataLoader};
use chrono::{DateTime, Utc};

use crate::{
    api_errors::schema_dependency,
    fleet::{
        credentials::Vault,
        devices::graphql::StatusOf,
        events::Bus,
        sessions::{
            DEFAULT_DPI, DisplayRequest, FieldError, Protocol, StartRequest, Ticket, Waking,
        },
        targets::AddressPolicy,
    },
    ids::DeviceId,
    status::Status,
};

/// Only these values come from the client (GW-012).
#[derive(InputObject)]
pub(super) struct DisplayInput {
    width_px: i32,
    height_px: i32,
    #[graphql(default_with = "Some(DEFAULT_DPI)")]
    dpi: Option<i32>,
    timezone: Option<String>,
}

#[derive(InputObject)]
pub(super) struct StartSessionInput {
    device_id: ID,
    protocol: Protocol,
    display: DisplayInput,
}

impl StartSessionInput {
    /// The request `start` checks; a null `dpi` means K1's default.
    pub(super) fn into_request(self) -> Result<StartRequest, Vec<FieldError>> {
        let device = self
            .device_id
            .parse::<DeviceId>()
            .map_err(|_| vec![FieldError::new("deviceId", "That device id is not valid.")])?;
        let display = self.display;
        Ok(StartRequest {
            device,
            protocol: self.protocol,
            display: DisplayRequest {
                width_px: display.width_px.into(),
                height_px: display.height_px.into(),
                dpi: i64::from(display.dpi.unwrap_or(DEFAULT_DPI)),
                timezone: display.timezone,
            },
            wake: None,
        })
    }
}

#[Object(name = "SessionTicket")]
impl Ticket {
    async fn session_id(&self) -> ID {
        ID(self.session.to_string())
    }

    /// Shown once; the hub keeps only its hash (LOG-007).
    #[graphql(name = "token")]
    async fn token_field(&self) -> &str {
        self.token()
    }

    /// The protocol's tunnel: `/terminal` for SSH, `/tunnel` for RDP and VNC (K4 §1), or
    /// `/files` (K4 §9).
    async fn tunnel_path(&self) -> &str {
        self.tunnel_path
    }

    async fn expires_at(&self) -> DateTime<Utc> {
        self.expires_at
    }

    /// Whether `startSession` sent a Wake-on-LAN packet first (K4 §1); the opening then
    /// waits up to 120 seconds instead of 8.
    async fn wake_sent(&self) -> bool {
        self.wake_sent
    }
}

/// What `start` needs to wake `device` first: whether it is asleep, which this resolver reads
/// as the device's status through devices' `StatusOf` loader, which checks fills, because
/// sessions may not import checks; the one place in sessions that reads a status. Also the
/// schema's vault, address policy and bus, the ones `wakeDevice` uses. A status that cannot
/// be read counts as not asleep, so no packet is sent and the start goes on.
///
/// # Errors
/// `INTERNAL` when the schema lacks one of them.
pub(super) async fn waking(ctx: &Context<'_>, device: DeviceId) -> Result<Waking> {
    let loaded = schema_dependency::<DataLoader<StatusOf>>(ctx)?
        .load_one(device)
        .await;
    let status = loaded.unwrap_or_else(|error| {
        tracing::warn!(%device, %error, "the device's status could not be read before a session start");
        None
    });
    Ok(Waking {
        asleep: status == Some(Status::Down),
        vault: schema_dependency::<Vault>(ctx)?.clone(),
        policy: schema_dependency::<AddressPolicy>(ctx)?.clone(),
        bus: schema_dependency::<Bus>(ctx)?.clone(),
    })
}
