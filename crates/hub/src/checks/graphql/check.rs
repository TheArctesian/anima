//! `Check` as the GraphQL type K1 describes. Every field reads through the checks face, so the
//! status and the uptime follow the same rules as the device's (CHK-004, CHK-011).

use std::sync::Arc;

use async_graphql::{Context, ID, Object, Result, dataloader::DataLoader};
use chrono::{DateTime, Utc};
use sqlx::PgPool;

use super::{
    config_view::ConfigView,
    enums::CheckKind,
    error::{from_devices, from_shared, invalid, schema_dependency, to_graphql},
    heartbeat::{HeartbeatConnection, connection, paging},
    loaders::{CheckStatuses, CheckUptimes, LastHeartbeats},
};
use crate::{
    checks::{self, Check, Error, Heartbeat, UptimeWindow, store},
    clock::Clock,
    devices::{self, Device},
    status::Status,
};

fn now(ctx: &Context<'_>) -> Result<DateTime<Utc>> {
    Ok(schema_dependency::<Arc<dyn Clock>>(ctx)?.now())
}

fn pool<'a>(ctx: &Context<'a>) -> Result<&'a PgPool> {
    schema_dependency::<PgPool>(ctx)
}

fn seconds(value: u32) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}

#[Object]
impl Check {
    async fn id(&self) -> ID {
        ID(self.id.to_string())
    }

    async fn device(&self, ctx: &Context<'_>) -> Result<Device> {
        devices::get(pool(ctx)?, self.device)
            .await
            .map_err(from_devices)
    }

    async fn parent(&self, ctx: &Context<'_>) -> Result<Option<Check>> {
        let Some(parent) = self.parent else {
            return Ok(None);
        };
        match checks::get(pool(ctx)?, parent).await {
            Ok(check) => Ok(Some(check)),
            Err(Error::NotFound(_)) => Ok(None),
            Err(error) => Err(to_graphql(error)),
        }
    }

    #[graphql(name = "type")]
    async fn check_type(&self) -> CheckKind {
        Check::kind(self).into()
    }

    async fn name(&self) -> &str {
        &self.name
    }

    async fn interval_seconds(&self) -> i32 {
        seconds(self.settings.interval_seconds)
    }

    async fn retries(&self) -> i32 {
        i32::from(self.settings.retries)
    }

    async fn retry_interval_seconds(&self) -> i32 {
        seconds(self.settings.retry_interval_seconds)
    }

    async fn degraded_threshold_ms(&self) -> i32 {
        seconds(self.settings.degraded_threshold_ms)
    }

    async fn invert(&self) -> bool {
        self.settings.invert
    }

    async fn paused(&self) -> bool {
        self.paused
    }

    async fn config(&self) -> ConfigView {
        ConfigView::of(self)
    }

    /// The status shown: pause, groups, maintenance and the device's manual flag applied.
    async fn status(&self, ctx: &Context<'_>) -> Result<Status> {
        let shown = schema_dependency::<DataLoader<CheckStatuses>>(ctx)?
            .load_one(self.id)
            .await
            .map_err(from_shared)?;
        Ok(shown.unwrap_or(Status::Unknown))
    }

    async fn last_heartbeat(&self, ctx: &Context<'_>) -> Result<Option<Heartbeat>> {
        schema_dependency::<DataLoader<LastHeartbeats>>(ctx)?
            .load_one(self.id)
            .await
            .map_err(from_shared)
    }

    #[expect(
        clippy::unused_async,
        clippy::unused_async_trait_impl,
        reason = "the Object macro requires async resolvers"
    )]
    /// Days until the TLS certificate expires (CHK-007). Only TLS and HTTPS checks.
    #[graphql(name = "certificateDaysLeft")]
    async fn certificate_days(&self, ctx: &Context<'_>) -> Result<Option<i32>> {
        Ok(Check::certificate_days_left(self, now(ctx)?))
    }

    async fn heartbeats(
        &self,
        ctx: &Context<'_>,
        #[graphql(default_with = "Some(100)")] first: Option<i32>,
        after: Option<String>,
    ) -> Result<HeartbeatConnection> {
        let (size, before) =
            paging(first.unwrap_or(100), after.as_deref()).map_err(|e| invalid(&e))?;
        let limit = u32::try_from(size + 1).unwrap_or(u32::MAX);
        let fetched = store::heartbeats_before(pool(ctx)?, self.id, before, i64::from(limit))
            .await
            .map_err(to_graphql)?;
        Ok(connection(fetched, size))
    }

    async fn uptime(&self, ctx: &Context<'_>, window: UptimeWindow) -> Result<Option<f64>> {
        schema_dependency::<DataLoader<CheckUptimes>>(ctx)?
            .load_one((self.id, window))
            .await
            .map_err(from_shared)
    }
}
