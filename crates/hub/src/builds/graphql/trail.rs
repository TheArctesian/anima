//! The audit entry of each operator change to builds, written in the mutation's own
//! transaction (BLD-082, LOG-005). A `change` never holds a token (LOG-007).

use std::sync::Arc;

use async_graphql::{Context, Result};
use chrono::{DateTime, Utc};
use serde_json::{Value, json};
use sqlx::PgConnection;

use super::error::{internal, schema_dependency};
use crate::{
    builds::{BuildJob, Builder, Release},
    clock::Clock,
    events::{self, AuditAction, AuditTarget, ClientAddr, NewAudit, Operator},
};

/// One mutation's audit entry.
pub(super) struct Entry {
    action: AuditAction,
    target: AuditTarget,
    change: Value,
}

/// Who did it, from where and when; the same `at` stamps the store write and the entry.
pub(super) struct Trail {
    pub(super) operator: Operator,
    pub(super) client: ClientAddr,
    pub(super) now: DateTime<Utc>,
}

impl Trail {
    /// Reads the request's operator and address and the schema's clock, once.
    pub(super) fn from_context(ctx: &Context<'_>) -> Result<Self> {
        Ok(Self {
            operator: schema_dependency::<Operator>(ctx)?.clone(),
            client: *schema_dependency::<ClientAddr>(ctx)?,
            now: schema_dependency::<Arc<dyn Clock>>(ctx)?.now(),
        })
    }

    /// Writes the audit entry. The caller commits it with its own write.
    pub(super) async fn write(self, conn: &mut PgConnection, entry: Entry) -> Result<()> {
        let audit = NewAudit {
            operator: self.operator,
            action: entry.action,
            target: entry.target,
            client: self.client,
            change: entry.change,
        };
        events::audit(conn, self.now, audit).await.map_err(internal)
    }
}

pub(super) fn release_created(release: &Release) -> Entry {
    let targets: Vec<&str> = release.jobs.iter().map(|job| job.target.as_str()).collect();
    Entry {
        action: AuditAction::Create,
        target: AuditTarget::Release(release.id),
        change: json!({
            "version": release.version.to_string(),
            "commit": release.commit.as_str(),
            "targets": targets,
        }),
    }
}

pub(super) fn job_cancelled(job: &BuildJob) -> Entry {
    Entry {
        action: AuditAction::Cancel,
        target: AuditTarget::BuildJob(job.id),
        change: json!({
            "releaseId": job.release.to_string(),
            "version": job.version.to_string(),
            "target": job.target.as_str(),
        }),
    }
}

/// The builder's name, platform and address; never its token.
fn builder_fields(builder: &Builder) -> Value {
    json!({
        "name": builder.name,
        "os": builder.platform.os_str(),
        "arch": builder.platform.arch_str(),
        "clientAddr": builder.client.to_string(),
    })
}

pub(super) fn builder_enrolled(builder: &Builder) -> Entry {
    Entry {
        action: AuditAction::Create,
        target: AuditTarget::Builder(builder.id),
        change: builder_fields(builder),
    }
}

pub(super) fn builder_revoked(builder: &Builder) -> Entry {
    Entry {
        action: AuditAction::Delete,
        target: AuditTarget::Builder(builder.id),
        change: builder_fields(builder),
    }
}
