//! `Session` as the GraphQL type K1 describes. It does not expose the operator id.

use async_graphql::{Context, ID, Object, Result};
use chrono::{DateTime, Utc};

use crate::{
    api_errors::schema_dependency,
    scalars::IpAddr,
    sessions::{EndReason, Live, Protocol, Session, share::SessionShare},
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

    /// The shares no cause has ended (RA-035); empty once the session has ended.
    #[expect(
        clippy::unused_async,
        clippy::unused_async_trait_impl,
        reason = "the Object macro requires async resolvers"
    )]
    async fn shares(&self, ctx: &Context<'_>) -> Result<Vec<SessionShare>> {
        if !self.is_live() {
            return Ok(Vec::new());
        }
        Ok(schema_dependency::<Live>(ctx)?.shares().of_session(self.id))
    }
}
