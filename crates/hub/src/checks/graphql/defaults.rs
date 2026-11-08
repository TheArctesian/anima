//! `Query.checkDefaults` and `Mutation.setCheckDefaults` (D-26, DB-011). The mutation writes
//! the row, one audit entry and one event in one transaction (API-003, LOG-005).

use std::sync::Arc;

use async_graphql::{Context, InputObject, Object, Result};
use serde_json::json;

use super::{
    error::{internal, invalid, schema_dependency, to_graphql},
    form::{keep, number},
    write::pool,
};
use crate::{
    checks::{self, CheckDefaults, Error, FieldError},
    clock::Clock,
    events::{
        self, AuditAction, AuditTarget, Bus, ClientAddr, NewAudit, NewEvent, Operator, Severity,
        Source,
    },
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
    /// The numbers that fit their types, or every field that does not.
    fn parse(&self) -> Result<CheckDefaults, Vec<FieldError>> {
        let mut errors = Vec::new();
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
    let bus = schema_dependency::<Bus>(ctx)?;
    let operator = schema_dependency::<Operator>(ctx)?.clone();
    let client = *schema_dependency::<ClientAddr>(ctx)?;
    let now = schema_dependency::<Arc<dyn Clock>>(ctx)?.now();
    let mut tx = pool(ctx)?
        .begin()
        .await
        .map_err(|e| to_graphql(Error::Database(e)))?;
    let stored = checks::set_defaults(&mut tx, wanted)
        .await
        .map_err(to_graphql)?;
    // The settings have no audit target of their own yet: the entry names the operator who
    // changed them.
    let audit = NewAudit {
        target: AuditTarget::Operator(operator.id),
        operator,
        action: AuditAction::Update,
        client,
        change: change(stored),
    };
    events::audit(&mut tx, now, audit).await.map_err(internal)?;
    let event = NewEvent {
        severity: Severity::Info,
        source: Source::Check,
        device: None,
        message: "The default check settings were changed.".to_owned(),
    };
    events::record(&mut tx, bus, now, event)
        .await
        .map_err(internal)?;
    tx.commit()
        .await
        .map_err(|e| to_graphql(Error::Database(e)))?;
    Ok(stored)
}
