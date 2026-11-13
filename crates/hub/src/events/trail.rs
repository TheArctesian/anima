//! The audit entry and the event of one change, written together in the caller's transaction
//! (LOG-004, LOG-005, LOG-007). Every domain's mutations write through [`Trail`]; each builds
//! its own [`TrailEntry`] from the stored rows with pure functions.

use std::sync::Arc;

use async_graphql::{Context, Result as GraphQlResult};
use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::PgConnection;

use super::{
    AuditAction, AuditTarget, Bus, ClientAddr, Error, NewAudit, NewEvent, Operator, Severity,
    Source, audit, record,
};
use crate::{
    api_errors::schema_dependency,
    clock::Clock,
    ids::{DeviceId, OperatorId},
};

/// One change's entry in both logs.
pub struct TrailEntry {
    pub source: Source,
    pub severity: Severity,
    /// The device the event is about, by id and name; `None` for a change that belongs to no
    /// single device.
    pub device: Option<(DeviceId, String)>,
    /// The event's sentence; it never holds a secret (LOG-007).
    pub message: String,
    pub action: AuditAction,
    pub target: AuditTarget,
    /// What changed, for the audit entry.
    pub change: Value,
}

/// Who did it, from where and when; the same `now` stamps the store write and both entries.
pub struct Trail<'a> {
    bus: &'a Bus,
    operator: Operator,
    client: ClientAddr,
    pub now: DateTime<Utc>,
}

impl<'a> Trail<'a> {
    /// A trail for an actor the request context does not hold, such as a files tunnel's ticket.
    #[must_use]
    pub fn new(
        bus: &'a Bus,
        (operator, client): (Operator, ClientAddr),
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            bus,
            operator,
            client,
            now,
        }
    }

    /// Reads the request's operator and address and the schema's bus and clock. The clock is
    /// read once.
    ///
    /// # Errors
    ///
    /// An `INTERNAL` GraphQL error if the schema lacks one of them.
    pub fn from_context(ctx: &Context<'a>) -> GraphQlResult<Self> {
        Ok(Self::new(
            schema_dependency::<Bus>(ctx)?,
            (
                schema_dependency::<Operator>(ctx)?.clone(),
                *schema_dependency::<ClientAddr>(ctx)?,
            ),
            schema_dependency::<Arc<dyn Clock>>(ctx)?.now(),
        ))
    }

    #[must_use]
    pub fn operator(&self) -> OperatorId {
        self.operator.id
    }

    /// Writes the audit entry, then the event. The caller commits both with its own write.
    ///
    /// The event goes last because `record` publishes it on the bus at once (D-5): if the
    /// audit write fails, no subscriber has seen an event for a change that is rolled back. A
    /// commit that fails after this still publishes an event that is not stored; the stored
    /// event list is the truth.
    ///
    /// # Errors
    ///
    /// `Error::Database` if either row cannot be written; the caller rolls back.
    pub async fn write(self, conn: &mut PgConnection, entry: TrailEntry) -> Result<(), Error> {
        let actor = self.operator.actor.kind();
        let row = NewAudit {
            operator: self.operator,
            action: entry.action,
            target: entry.target,
            client: self.client,
            change: entry.change,
        };
        audit(conn, self.now, row).await?;
        let event = NewEvent {
            severity: entry.severity,
            source: entry.source,
            device: entry.device,
            message: entry.message,
            actor,
        };
        record(conn, self.bus, self.now, event).await.map(drop)
    }
}
