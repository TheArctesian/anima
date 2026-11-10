//! The builder's mutations: claim a job, report on it, get its upload URLs, report its upload
//! (BLD-004 to BLD-010, BLD-060, BLD-063). Each is behind `BuilderSignedIn` and then checks the builder record and the
//! address it enrolled from (BLD-022, BLD-024).

use std::sync::Arc;

use async_graphql::{Context, ID, Object, Result};
use sqlx::PgPool;

use super::{
    error::{parse_id, to_graphql},
    invalid_field, now,
    types::{FileFields, PlatformFields},
    uploads::UploadUrlFields,
};
use crate::{
    access::{BuilderSignedIn, Identity},
    api_errors::schema_dependency,
    builds::{
        self, Bucket, BuildJob, BuildQueue, Builder, Error, Grant, JobState, Platform, Report,
        Updates, Upload, UploadedFile,
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

/// The refusal of `buildUploadUrls` on a hub with no release bucket set up.
const NOT_PUBLISHING: &str = "Publishing is not set up on this hub.";

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
    #[expect(
        clippy::too_many_arguments,
        reason = "K1 fixes the argument list of Mutation.reportBuildJob"
    )]
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

    /// One presigned `PUT` URL for each named file of an `UPLOADING` job its builder holds; the
    /// files become the job's grant (BLD-060, BLD-061, D-26). `CONFLICT` when this hub does not
    /// publish. No error names a URL (LOG-007).
    #[graphql(guard = "BuilderSignedIn")]
    async fn build_upload_urls(
        &self,
        ctx: &Context<'_>,
        id: ID,
        files: Vec<FileFields>,
    ) -> Result<Vec<UploadUrlFields>> {
        let job: BuildJobId = parse_id(&id, "id")?;
        let builder = signed_builder(ctx).await?;
        let bucket = schema_dependency::<Option<Arc<Bucket>>>(ctx)?
            .as_deref()
            .ok_or_else(|| to_graphql(Error::Conflict(NOT_PUBLISHING.to_owned())))?;
        let files: Vec<UploadedFile> = files.into_iter().map(Into::into).collect();
        let grant = Grant {
            builder: &builder,
            job,
            files: &files,
        };
        let db = schema_dependency::<PgPool>(ctx)?;
        let urls = builds::upload_urls(db, bucket, grant, now(ctx)?)
            .await
            .map_err(to_graphql)?;
        Ok(urls.into_iter().map(Into::into).collect())
    }

    /// Records the uploaded files and the proposed feed of an `UPLOADING` job and ends its
    /// lease; the hub then verifies and publishes it in the background (BLD-063 to BLD-070,
    /// D-25).
    #[graphql(guard = "BuilderSignedIn")]
    #[expect(
        clippy::too_many_arguments,
        reason = "K1 fixes the argument list of Mutation.reportBuildUploaded"
    )]
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
