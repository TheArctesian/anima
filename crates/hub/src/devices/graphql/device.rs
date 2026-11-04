//! `Device` as the GraphQL type K1 describes (R-011 seam: later packages add `status`,
//! `checks` and `uptime` to this block).

use async_graphql::{Context, ID, Object, Result, dataloader::DataLoader};
use chrono::{DateTime, Utc};

use super::{
    error::{schema_dependency, to_graphql},
    ssh::DeviceSsh,
};
use crate::{
    devices::{
        Error, Missing, OperatingSystem, SitesById,
        model::{Device, Site},
    },
    scalars::IpAddr,
};

#[Object]
impl Device {
    async fn id(&self) -> ID {
        ID(self.id.to_string())
    }

    async fn site(&self, ctx: &Context<'_>) -> Result<Site> {
        schema_dependency::<DataLoader<SitesById>>(ctx)?
            .load_one(self.site)
            .await
            .map_err(to_graphql)?
            .ok_or_else(|| to_graphql(Error::NotFound(Missing::Site(self.site))))
    }

    async fn name(&self) -> &str {
        &self.name
    }

    async fn host(&self) -> &str {
        &self.host
    }

    async fn ip(&self) -> Option<IpAddr> {
        self.ip.map(IpAddr)
    }

    async fn kind(&self) -> &str {
        &self.kind
    }

    async fn os(&self) -> OperatingSystem {
        self.os
    }

    async fn tags(&self) -> &[String] {
        &self.tags
    }

    /// Manual maintenance flag (CHK-006).
    async fn maintenance(&self) -> bool {
        self.maintenance
    }

    async fn ssh(&self) -> DeviceSsh {
        DeviceSsh {
            device: self.id,
            settings: self.ssh.clone(),
            credential_stored: None,
        }
    }

    async fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    async fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }
}
