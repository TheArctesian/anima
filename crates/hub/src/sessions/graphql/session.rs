//! `Session` as the GraphQL type K1 describes. It does not expose the operator id.

use async_graphql::{Context, ID, Object, Result, dataloader::DataLoader};
use chrono::{DateTime, Utc};

use super::recording::{RecordingStateValue, RecordingStates};
use crate::{
    api_errors::schema_dependency,
    scalars::IpAddr,
    sessions::{EndReason, Protocol, Session},
};

#[Object]
impl Session {
    async fn id(&self) -> ID {
        ID(self.id.to_string())
    }

    async fn device_id(&self) -> Option<ID> {
        self.device.map(|device| ID(device.to_string()))
    }

    /// The device's name when the session started; kept after the device is deleted.
    async fn device_name(&self) -> &str {
        &self.device_name
    }

    async fn protocol(&self) -> Protocol {
        self.protocol
    }

    async fn operator_name(&self) -> &str {
        &self.operator_name
    }

    async fn started_at(&self) -> DateTime<Utc> {
        self.started_at
    }

    async fn ended_at(&self) -> Option<DateTime<Utc>> {
        self.end.map(|end| end.at)
    }

    async fn end_reason(&self) -> Option<EndReason> {
        self.end.map(|end| end.reason)
    }

    /// The address of the computer that started the session (RA-034).
    async fn client_addr(&self) -> IpAddr {
        IpAddr(self.client)
    }

    /// Whether the session was recorded and whether its recording stopped early (RA-040,
    /// G-5): `OFF` with no recording, `STOPPED` with a stop reason, else `ON`.
    async fn recording(&self, ctx: &Context<'_>) -> Result<RecordingStateValue> {
        let state = schema_dependency::<DataLoader<RecordingStates>>(ctx)?
            .load_one(self.id)
            .await?;
        // `RecordingStates` answers every key, `OFF` for a session with no recording row, so
        // a missing answer cannot happen; should it ever, `OFF` is the state a session with
        // no recording row has.
        Ok(state.unwrap_or(RecordingStateValue::Off))
    }
}
