//! `Query.checkDefaults` and `Mutation.setCheckDefaults` (D-26, DB-011). The mutation writes
//! the row, one audit entry and one event in one transaction (API-003, LOG-005).

use async_graphql::{Context, InputObject, Object, Result};
use serde_json::json;

use super::{
    error::to_graphql,
    form::{keep, number},
};
use crate::{
    api_errors::{begin, commit, internal, invalid, pool},
    checks::{self, CheckDefaults, FieldError, rules},
    events::{AuditAction, AuditTarget, Severity, Source, Trail, TrailEntry},
};

#[Object(name = "CheckDefaults")]
impl CheckDefaults {
    async fn interval_seconds(&self) -> i32 {
        i32::try_from(self.interval_seconds).unwrap_or(i32::MAX)
    }

    async fn retries(&self) -> i32 {
        i32::from(self.retries)
    }

    async fn retry_interval_seconds(&self) -> i32 {
        i32::try_from(self.retry_interval_seconds).unwrap_or(i32::MAX)
    }

    async fn degraded_threshold_ms(&self) -> i32 {
        i32::try_from(self.degraded_threshold_ms).unwrap_or(i32::MAX)
    }
}

#[derive(InputObject)]
#[graphql(name = "CheckDefaultsInput")]
pub(super) struct DefaultsInput {
    interval_seconds: i32,
    retries: i32,
    retry_interval_seconds: i32,
    degraded_threshold_ms: i32,
}

impl DefaultsInput {
    /// The numbers inside the bounds of a check, or every field that is not.
    fn parse(&self) -> Result<CheckDefaults, Vec<FieldError>> {
        let mut errors = rules::setting_bounds(
            self.interval_seconds,
            self.retries,
            self.retry_interval_seconds,
            self.degraded_threshold_ms,
        );
        if !errors.is_empty() {
            return Err(errors);
        }
        let interval = keep(
            number("intervalSeconds", self.interval_seconds),
            &mut errors,
        );
        let retries = keep(number("retries", self.retries), &mut errors);
        let retry_interval = keep(
            number("retryIntervalSeconds", self.retry_interval_seconds),
            &mut errors,
        );
        let degraded = keep(
            number("degradedThresholdMs", self.degraded_threshold_ms),
            &mut errors,
        );
        match (interval, retries, retry_interval, degraded) {
            (
                Some(interval_seconds),
                Some(retries),
                Some(retry_interval_seconds),
                Some(degraded),
            ) => Ok(CheckDefaults {
                interval_seconds,
                retries,
                retry_interval_seconds,
                degraded_threshold_ms: degraded,
            }),
            _ => Err(errors),
        }
    }
}

pub(super) async fn read(ctx: &Context<'_>) -> Result<CheckDefaults> {
    checks::defaults(pool(ctx)?).await.map_err(to_graphql)
}

fn change(stored: CheckDefaults) -> serde_json::Value {
    json!({
        "intervalSeconds": stored.interval_seconds,
        "retries": stored.retries,
        "retryIntervalSeconds": stored.retry_interval_seconds,
        "degradedThresholdMs": stored.degraded_threshold_ms,
    })
}

pub(super) async fn set(ctx: &Context<'_>, input: &DefaultsInput) -> Result<CheckDefaults> {
    let wanted = input.parse().map_err(|errors| invalid(&errors))?;
    let trail = Trail::from_context(ctx)?;
    let mut tx = begin(ctx).await?;
    let stored = checks::set_defaults(&mut tx, wanted)
        .await
        .map_err(to_graphql)?;
    let entry = TrailEntry {
        source: Source::Check,
        severity: Severity::Info,
        device: None,
        message: "The default check settings were changed.".to_owned(),
        action: AuditAction::Update,
        target: AuditTarget::Setting("checks.defaults"),
        change: change(stored),
    };
    trail.write(&mut tx, entry).await.map_err(internal)?;
    commit(tx).await?;
    Ok(stored)
}
