//! `Check` as the GraphQL type K1 describes. Every field reads through the checks face, so the
//! status and the uptime follow the same rules as the device's (CHK-004, CHK-011).

use std::sync::Arc;

use async_graphql::{Context, ID, Object, Result};
use chrono::{DateTime, Utc};
use sqlx::PgPool;

use super::{
    config_view::ConfigView,
    enums::CheckKind,
    error::{from_devices, invalid, schema_dependency, to_graphql},
    heartbeat::{HeartbeatConnection, connection, paging},
};
use crate::{
    checks::{self, Check, Error, Heartbeat, UptimeWindow},
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
        checks::check_status(pool(ctx)?, self, now(ctx)?)
            .await
            .map_err(to_graphql)
    }

    async fn last_heartbeat(&self, ctx: &Context<'_>) -> Result<Option<Heartbeat>> {
        let latest = checks::heartbeats(pool(ctx)?, self.id, None, 1)
            .await
            .map_err(to_graphql)?;
        Ok(latest.into_iter().next())
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
        let fetched = checks::heartbeats(pool(ctx)?, self.id, before, limit)
            .await
            .map_err(to_graphql)?;
        Ok(connection(fetched, size))
    }

    async fn uptime(&self, ctx: &Context<'_>, window: UptimeWindow) -> Result<Option<f64>> {
        checks::check_uptime(pool(ctx)?, self.id, window, now(ctx)?)
            .await
            .map_err(to_graphql)
    }
}
