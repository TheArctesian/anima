//! What a retention change leaves behind: one audit entry and one event, written in the
//! mutation's own transaction (LOG-004, LOG-005, LOG-007).
//!
//! The event source is `Hub`: a period belongs to no single device. The audit target is the
//! setting `retention.<subject>`.

use std::sync::Arc;

use async_graphql::{Context, Result};
use chrono::{DateTime, Utc};
use serde_json::json;
use sqlx::PgConnection;

use super::error::{internal, schema_dependency};
use crate::{
    clock::Clock,
    events::{
        self, AuditAction, AuditTarget, Bus, ClientAddr, NewAudit, NewEvent, Operator, Severity,
        Source,
    },
    retention::Subject,
};

/// One mutation's entry in both logs.
pub(super) struct Entry {
    message: String,
    target: AuditTarget,
    change: serde_json::Value,
}

/// Who did it, from where and when; the same `at` stamps the store write and both entries.
pub(super) struct Trail<'a> {
    bus: &'a Bus,
    operator: Operator,
    client: ClientAddr,
    pub(super) now: DateTime<Utc>,
}

impl<'a> Trail<'a> {
    /// Reads the request's operator and address and the schema's bus and clock. The clock is
    /// read once.
    pub(super) fn from_context(ctx: &Context<'a>) -> Result<Self> {
        Ok(Self {
            bus: schema_dependency::<Bus>(ctx)?,
            operator: schema_dependency::<Operator>(ctx)?.clone(),
            client: *schema_dependency::<ClientAddr>(ctx)?,
            now: schema_dependency::<Arc<dyn Clock>>(ctx)?.now(),
        })
    }

    /// Writes the audit entry, then the event. The caller commits both with its own write.
    ///
    /// The event goes last because `events::record` publishes it on the bus at once: if the
    /// audit write fails, no subscriber has seen an event for a change that is rolled back.
    pub(super) async fn write(self, conn: &mut PgConnection, entry: Entry) -> Result<()> {
        let audit = NewAudit {
            operator: self.operator,
            action: AuditAction::Update,
            target: entry.target,
            client: self.client,
            change: entry.change,
        };
        events::audit(conn, self.now, audit)
            .await
            .map_err(internal)?;
        let event = NewEvent {
            severity: Severity::Info,
            source: Source::Hub,
            device: None,
            message: entry.message,
        };
        events::record(conn, self.bus, self.now, event)
            .await
            .map(drop)
            .map_err(internal)
    }
}

/// A subject's period changed from `previous` to `days`.
pub(super) fn changed(subject: Subject, days: u16, previous: i32) -> Entry {
    Entry {
        message: format!(
            "Retention of {} set to {days} days (was {previous}).",
            subject.as_str()
        ),
        target: AuditTarget::Setting(setting_name(subject)),
        change: json!({ "subject": subject.as_str(), "days": days, "previousDays": previous }),
    }
}

const fn setting_name(subject: Subject) -> &'static str {
    match subject {
        Subject::Heartbeats => "retention.heartbeats",
        Subject::Metrics => "retention.metrics",
        Subject::Events => "retention.events",
        Subject::Recordings => "retention.recordings",
        Subject::Deliveries => "retention.deliveries",
    }
}
