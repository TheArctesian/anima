//! The K1 types of builds. The domain types are the output objects; the enums are the domain
//! enums themselves.

use std::sync::Arc;

use async_graphql::{Context, ID, InputObject, Object, Result, SimpleObject};
use chrono::{DateTime, Utc};

use super::error::schema_dependency;
use crate::{
    builds::{
        Arch, BuildJob, Builder, Enrolment, JobState, JobUpdate, LogLine, Os, Platform, Release,
        ReleasePage, Target, UploadedFile, cursor,
    },
    clock::Clock,
    paging::PageEnd,
    scalars::IpAddr,
};

#[Object(name = "Release")]
impl Release {
    async fn id(&self) -> ID {
        ID(self.id.to_string())
    }

    async fn version(&self) -> String {
        self.version.to_string()
    }

    /// 40 lower-case hex digits.
    async fn commit(&self) -> &str {
        self.commit.as_str()
    }

    async fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    /// The operator who started it.
    async fn created_by(&self) -> &str {
        &self.created_by
    }

    /// One job for each selected target.
    async fn jobs(&self) -> &[BuildJob] {
        &self.jobs
    }
}

#[Object(name = "BuildJob")]
impl BuildJob {
    async fn id(&self) -> ID {
        ID(self.id.to_string())
    }

    async fn release_id(&self) -> ID {
        ID(self.release.to_string())
    }

    async fn target(&self) -> Target {
        self.target
    }

    async fn version(&self) -> String {
        self.version.to_string()
    }

    async fn commit(&self) -> &str {
        self.commit.as_str()
    }

    async fn state(&self) -> JobState {
        self.state
    }

    async fn failure_reason(&self) -> Option<&str> {
        self.failure_reason.as_deref()
    }

    /// Leases that ended without a report (BLD-006).
    async fn attempts(&self) -> i32 {
        i32::from(self.attempts)
    }

    async fn builder_id(&self) -> Option<ID> {
        self.builder.map(|id| ID(id.to_string()))
    }

    /// The builder that last claimed it.
    async fn builder_name(&self) -> Option<&str> {
        self.builder_name.as_deref()
    }

    async fn lease_ends_at(&self) -> Option<DateTime<Utc>> {
        self.lease_ends_at
    }

    async fn queued_at(&self) -> DateTime<Utc> {
        self.times.queued
    }

    async fn claimed_at(&self) -> Option<DateTime<Utc>> {
        self.times.claimed
    }

    async fn building_at(&self) -> Option<DateTime<Utc>> {
        self.times.building
    }

    async fn uploading_at(&self) -> Option<DateTime<Utc>> {
        self.times.uploading
    }

    async fn published_at(&self) -> Option<DateTime<Utc>> {
        self.times.published
    }

    async fn failed_at(&self) -> Option<DateTime<Utc>> {
        self.times.failed
    }

    async fn cancelled_at(&self) -> Option<DateTime<Utc>> {
        self.times.cancelled
    }

    /// When it entered its current state (BLD-080).
    async fn state_changed_at(&self) -> DateTime<Utc> {
        self.times.changed
    }
}

#[Object(name = "BuildLogLine")]
impl LogLine {
    async fn sequence(&self) -> i64 {
        self.sequence
    }

    /// Redacted before it was stored (BLD-049).
    async fn text(&self) -> &str {
        &self.text
    }
}

#[Object(name = "BuildJobUpdate")]
impl JobUpdate {
    async fn job(&self) -> &BuildJob {
        &self.job
    }

    /// The log lines this change added.
    async fn log_lines(&self) -> &[LogLine] {
        &self.lines
    }
}

/// K1 `BuilderPlatform` and `BuilderPlatformInput`.
#[derive(Clone, Copy, SimpleObject, InputObject)]
#[graphql(name = "BuilderPlatform", input_name = "BuilderPlatformInput")]
pub(super) struct PlatformFields {
    os: Os,
    arch: Arch,
}

impl From<Platform> for PlatformFields {
    fn from(platform: Platform) -> Self {
        Self {
            os: platform.os,
            arch: platform.arch,
        }
    }
}

impl From<PlatformFields> for Platform {
    fn from(fields: PlatformFields) -> Self {
        Self {
            os: fields.os,
            arch: fields.arch,
        }
    }
}

#[Object(name = "Builder")]
impl Builder {
    async fn id(&self) -> ID {
        ID(self.id.to_string())
    }

    async fn name(&self) -> &str {
        &self.name
    }

    async fn platform(&self) -> PlatformFields {
        self.platform.into()
    }

    /// The VPN address it enrolled from and must call from (BLD-024).
    async fn client_addr(&self) -> IpAddr {
        IpAddr(self.client)
    }

    async fn enrolled_at(&self) -> DateTime<Utc> {
        self.enrolled_at
    }

    async fn last_seen_at(&self) -> Option<DateTime<Utc>> {
        self.last_seen_at
    }

    /// It called within the last 60 seconds (BLD-007).
    #[expect(
        clippy::unused_async,
        clippy::unused_async_trait_impl,
        reason = "the Object macro requires async resolvers"
    )]
    async fn online(&self, ctx: &Context<'_>) -> Result<bool> {
        let now = schema_dependency::<Arc<dyn Clock>>(ctx)?.now();
        Ok(self.online_at(now))
    }
}

/// K1 `BuilderEnrolment`. No `Debug`: it holds the token.
pub(super) struct EnrolmentPayload(pub(super) Enrolment);

#[Object(name = "BuilderEnrolment")]
impl EnrolmentPayload {
    async fn builder(&self) -> &Builder {
        &self.0.builder
    }

    /// The builder token, shown once. The client keeps it in the key store (BLD-023).
    async fn token(&self) -> &str {
        self.0.token.secret()
    }

    async fn expires_at(&self) -> DateTime<Utc> {
        self.0.token.expires_at
    }
}

/// K1 `BuildFileInput`.
#[derive(InputObject)]
#[graphql(name = "BuildFileInput")]
pub(super) struct FileFields {
    name: String,
    /// 64 hex digits.
    sha256: String,
    /// In bytes.
    size: i64,
}

impl From<FileFields> for UploadedFile {
    fn from(fields: FileFields) -> Self {
        Self {
            name: fields.name,
            sha256: fields.sha256,
            size: fields.size,
        }
    }
}

#[derive(SimpleObject)]
#[graphql(name = "ReleaseEdge")]
pub(super) struct ReleaseEdge {
    cursor: String,
    node: Release,
}

#[derive(SimpleObject)]
#[graphql(name = "ReleaseConnection")]
pub(super) struct ReleaseConnection {
    edges: Vec<ReleaseEdge>,
    page_info: PageEnd,
}

impl From<ReleasePage> for ReleaseConnection {
    fn from(page: ReleasePage) -> Self {
        let end_cursor = page.releases.last().map(cursor);
        Self {
            edges: page
                .releases
                .into_iter()
                .map(|node| ReleaseEdge {
                    cursor: cursor(&node),
                    node,
                })
                .collect(),
            page_info: PageEnd {
                has_next_page: page.has_next_page,
                end_cursor,
            },
        }
    }
}
