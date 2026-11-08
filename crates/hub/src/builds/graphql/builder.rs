//! The builder's mutations: claim a job, report on it, report its upload (BLD-004 to BLD-010,
//! BLD-063). Each is behind `BuilderSignedIn` and then checks the builder record and the
//! address it enrolled from (BLD-022, BLD-024).

use async_graphql::{Context, ID, Object, Result};
use sqlx::PgPool;

use super::{
    error::{parse_id, schema_dependency, to_graphql},
    invalid_field, now,
    types::{FileFields, PlatformFields},
};
use crate::{
    access::{BuilderSignedIn, Identity},
    builds::{
        self, BuildJob, BuildQueue, Builder, Error, JobState, Platform, Report, Updates, Upload,
    },
    events::{Bus, ClientAddr},
    ids::BuildJobId,
};

/// The builder that makes this request, checked against its record and address. The process
/// log gets a refused address by builder id only, never the token (LOG-007).
async fn signed_builder(ctx: &Context<'_>) -> Result<Builder> {
    let identity = schema_dependency::<Identity>(ctx)?;
    let token = identity
        .builder_token()
        .ok_or_else(|| to_graphql(Error::Forbidden("Only a builder token may do this.")))?;
    let client = ctx.data_opt::<ClientAddr>().map(|client| client.0);
    let db = schema_dependency::<PgPool>(ctx)?;
    builds::builder_for(db, token, client, now(ctx)?)
        .await
        .map_err(|error| {
            if matches!(error, Error::Forbidden(_)) {
                tracing::warn!(domain = "builds", "builder request refused");
            }
            to_graphql(error)
        })
}

fn build_queue<'a>(ctx: &Context<'a>) -> Result<BuildQueue<'a>> {
    Ok(BuildQueue {
        db: schema_dependency::<PgPool>(ctx)?,
        bus: schema_dependency::<Bus>(ctx)?,
        updates: schema_dependency::<Updates>(ctx)?,
    })
}

#[derive(Default)]
pub struct BuilderMutation;

#[Object(name = "BuilderMutation")]
impl BuilderMutation {
    /// The oldest queued job this builder's platform can build, with a 90-second lease; null
    /// when none waits or the builder still holds one (BLD-003 to BLD-005, BLD-029).
    #[graphql(guard = "BuilderSignedIn")]
    async fn claim_build_job(
        &self,
        ctx: &Context<'_>,
        platform: PlatformFields,
    ) -> Result<Option<BuildJob>> {
        let builder = signed_builder(ctx).await?;
        if Platform::from(platform) != builder.platform {
            return Err(invalid_field(
                "platform",
                "This builder enrolled with another platform.",
            ));
        }
        builds::claim(build_queue(ctx)?, &builder, now(ctx)?)
            .await
            .map_err(to_graphql)
    }

    /// Extends the lease, records the state change and appends the log lines; only the lease
    /// holder may report (BLD-005, BLD-010, BLD-048). `failureReason` is read for `FAILED`.
    #[graphql(guard = "BuilderSignedIn")]
    async fn report_build_job(
        &self,
        ctx: &Context<'_>,
        id: ID,
        state: Option<JobState>,
        log_lines: Vec<String>,
        failure_reason: Option<String>,
    ) -> Result<BuildJob> {
        let job: BuildJobId = parse_id(&id, "id")?;
        let builder = signed_builder(ctx).await?;
        let report = Report {
            job,
            state,
            reason: failure_reason,
            lines: log_lines,
        };
        builds::report(build_queue(ctx)?, &builder, report, now(ctx)?)
            .await
            .map_err(to_graphql)
    }

    /// Records the uploaded files and the proposed feed of an `UPLOADING` job and extends its
    /// lease; the job stays `UPLOADING` until the hub publishes it (BLD-063, D-5).
    #[graphql(guard = "BuilderSignedIn")]
    async fn report_build_uploaded(
        &self,
        ctx: &Context<'_>,
        id: ID,
        files: Vec<FileFields>,
        feed: String,
    ) -> Result<BuildJob> {
        let job: BuildJobId = parse_id(&id, "id")?;
        let builder = signed_builder(ctx).await?;
        let uploaded = Upload {
            job,
            files: files.into_iter().map(Into::into).collect(),
            feed,
        };
        builds::report_uploaded(build_queue(ctx)?, &builder, uploaded, now(ctx)?)
            .await
            .map_err(to_graphql)
    }
}
