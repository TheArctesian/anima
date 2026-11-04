//! `Device` as the GraphQL type K1 describes, with the fields that join the checks domain
//! (R-011): `status`, `checks`, `uptime` and the resolved `ip`.

use async_graphql::{Context, ID, Object, Result, dataloader::DataLoader};
use chrono::{DateTime, Utc};

use super::{
    error::{internal, schema_dependency, to_graphql},
    ssh::DeviceSsh,
    standing::{ChecksOf, ResolvedIpOf, StatusOf, UptimeOf},
};
use crate::{
    checks::{Check, UptimeWindow},
    devices::{
        Error, Missing, OperatingSystem, SitesById,
        model::{Device, Site},
    },
    scalars::IpAddr,
    status::Status,
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

    /// The host when it is an IP literal, else the address the last check resolved (R-026(3)).
    async fn ip(&self, ctx: &Context<'_>) -> Result<Option<IpAddr>> {
        if let Some(literal) = self.ip {
            return Ok(Some(IpAddr(literal)));
        }
        let resolved = schema_dependency::<DataLoader<ResolvedIpOf>>(ctx)?
            .load_one(self.id)
            .await
            .map_err(internal)?;
        Ok(resolved.map(IpAddr))
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

    /// The worst status of the device's checks (CHK-004).
    async fn status(&self, ctx: &Context<'_>) -> Result<Status> {
        let status = schema_dependency::<DataLoader<StatusOf>>(ctx)?
            .load_one(self.id)
            .await
            .map_err(internal)?;
        Ok(status.unwrap_or(Status::Unknown))
    }

    async fn checks(&self, ctx: &Context<'_>) -> Result<Vec<Check>> {
        let checks = schema_dependency::<DataLoader<ChecksOf>>(ctx)?
            .load_one(self.id)
            .await
            .map_err(internal)?;
        Ok(checks.unwrap_or_default())
    }

    /// Uptime ratio 0..1 from the aggregate tables; null with no counted beats (CHK-011).
    async fn uptime(&self, ctx: &Context<'_>, window: UptimeWindow) -> Result<Option<f64>> {
        schema_dependency::<DataLoader<UptimeOf>>(ctx)?
            .load_one((self.id, window))
            .await
            .map_err(internal)
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
