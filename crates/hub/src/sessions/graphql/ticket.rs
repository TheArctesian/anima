//! `startSession`'s input and output as K1 describes them (`StartSessionInput`,
//! `DisplayInput`, `SessionTicket`; API-009).

use async_graphql::{ID, InputObject, Object};
use chrono::{DateTime, Utc};

use crate::{
    ids::DeviceId,
    sessions::{DEFAULT_DPI, DisplayRequest, FieldError, Protocol, StartRequest, Ticket},
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

    /// The protocol's tunnel, `/terminal` for SSH (K4 §1, R-062), or `/files` (K4 §9).
    async fn tunnel_path(&self) -> &str {
        self.tunnel_path
    }

    async fn expires_at(&self) -> DateTime<Utc> {
        self.expires_at
    }
}
