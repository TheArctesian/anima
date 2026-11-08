//! The builds GraphQL part (K1; Section 3.10): the operator's `Query`, `Mutation` and
//! `Subscription`, each field behind `Authenticated`, and the builder's mutations in
//! [`BuilderMutation`], behind `BuilderSignedIn` (BLD-022, D-1).
//!
//! Each operator mutation writes its change and one audit entry in one transaction (BLD-082).

mod builder;
mod error;
mod trail;
mod types;

use std::sync::Arc;

use async_graphql::{
    Context, ID, Object, Result, Subscription,
    futures_util::stream::{self, Stream},
};
use sqlx::{PgPool, Postgres, Transaction};
use tokio::sync::broadcast::{Receiver, error::RecvError};

pub use self::builder::BuilderMutation;
use self::{
    error::{invalid, parse_id, schema_dependency, to_graphql},
    trail::Trail,
    types::{EnrolmentPayload, PlatformFields, ReleaseConnection},
};
use super::{
    BuildJob, Builder, BuilderInput, Error, JobUpdate, LogLine, Release, ReleaseInput, Target,
    Updates,
};
use crate::{access::Authenticated, clock::Clock, ids::BuildJobId};

/// The page size of `releases` when the client names none (K1).
const FIRST_RELEASES: i32 = 20;

/// The page size of `buildJobLog` when the client names none (K1).
const FIRST_LOG_LINES: i32 = 500;

/// The lines of `buildJobLog` that cost as much as one field (API-004): a page of 1000 lines
/// costs 20 more than one line.
const LOG_LINES_PER_FIELD: usize = 50;

/// What a `buildJobLog` page of `first` lines costs against the query limit (API-004).
fn log_page_complexity(first: Option<i32>, child_complexity: usize) -> usize {
    let first = usize::try_from(first.unwrap_or(FIRST_LOG_LINES)).unwrap_or(0);
    first / LOG_LINES_PER_FIELD + child_complexity
}

fn pool<'a>(ctx: &Context<'a>) -> Result<&'a PgPool> {
    schema_dependency::<PgPool>(ctx)
}

/// The transaction a mutation writes its change and its audit entry in.
async fn begin(ctx: &Context<'_>) -> Result<Transaction<'static, Postgres>> {
    pool(ctx)?
        .begin()
        .await
        .map_err(|e| to_graphql(Error::Database(e)))
}

async fn finish(tx: Transaction<'static, Postgres>) -> Result<()> {
    tx.commit()
        .await
        .map_err(|e| to_graphql(Error::Database(e)))
}

#[derive(Default)]
pub struct Query;

#[Object(name = "BuildsQuery")]
impl Query {
    /// Newest first, each with its jobs (BLD-080, API-006).
    #[graphql(guard = "Authenticated")]
    async fn releases(
        &self,
        ctx: &Context<'_>,
        #[graphql(default_with = "Some(FIRST_RELEASES)")] first: Option<i32>,
        after: Option<String>,
    ) -> Result<ReleaseConnection> {
        let first = first.unwrap_or(FIRST_RELEASES);
        let page = super::releases(pool(ctx)?, first, after.as_deref())
            .await
            .map_err(to_graphql)?;
        Ok(page.into())
    }

    /// The highest version published or under way with the patch plus 1; null before the first
    /// release.
    #[graphql(guard = "Authenticated")]
    async fn next_release_version(&self, ctx: &Context<'_>) -> Result<Option<String>> {
        let next = super::next_version(pool(ctx)?).await.map_err(to_graphql)?;
        Ok(next.map(|version| version.to_string()))
    }

    #[graphql(guard = "Authenticated")]
    async fn builders(&self, ctx: &Context<'_>) -> Result<Vec<Builder>> {
        super::builders(pool(ctx)?).await.map_err(to_graphql)
    }

    /// Up to `first` (1 to 1000) of the job's stored lines after sequence `after`, oldest first;
    /// the next page starts after the last sequence returned (BLD-048, BLD-081).
    #[graphql(
        guard = "Authenticated",
        complexity = "log_page_complexity(first, child_complexity)"
    )]
    #[expect(
        clippy::too_many_arguments,
        reason = "K1 fixes the argument list of Query.buildJobLog"
    )]
    async fn build_job_log(
        &self,
        ctx: &Context<'_>,
        id: ID,
        after: Option<i64>,
        #[graphql(default_with = "Some(FIRST_LOG_LINES)")] first: Option<i32>,
    ) -> Result<Vec<LogLine>> {
        let id: BuildJobId = parse_id(&id, "id")?;
        let first = first.unwrap_or(FIRST_LOG_LINES);
        super::job_log(pool(ctx)?, id, after, first)
            .await
            .map_err(to_graphql)
    }
}

#[derive(Default)]
pub struct Mutation;

#[Object(name = "BuildsMutation")]
impl Mutation {
    /// Queues one job for each target (BLD-001, BLD-002).
    #[graphql(guard = "Authenticated")]
    #[expect(
        clippy::too_many_arguments,
        reason = "K1 fixes the argument list of Mutation.createRelease"
    )]
    async fn create_release(
        &self,
        ctx: &Context<'_>,
        version: String,
        commit: String,
        targets: Vec<Target>,
    ) -> Result<Release> {
        let trail = Trail::from_context(ctx)?;
        let input = ReleaseInput {
            version,
            commit,
            targets,
        };
        let mut tx = begin(ctx).await?;
        let release = super::create_release(&mut tx, &trail.operator, &input, trail.now)
            .await
            .map_err(to_graphql)?;
        trail
            .write(&mut tx, trail::release_created(&release))
            .await?;
        finish(tx).await?;
        Ok(release)
    }

    /// Only a queued job can be cancelled (BLD-008).
    #[graphql(guard = "Authenticated")]
    async fn cancel_build_job(&self, ctx: &Context<'_>, id: ID) -> Result<BuildJob> {
        let id: BuildJobId = parse_id(&id, "id")?;
        let trail = Trail::from_context(ctx)?;
        let updates = schema_dependency::<Updates>(ctx)?;
        let mut tx = begin(ctx).await?;
        let job = super::cancel_job(&mut tx, id, trail.now)
            .await
            .map_err(to_graphql)?;
        trail.write(&mut tx, trail::job_cancelled(&job)).await?;
        finish(tx).await?;
        updates.publish(JobUpdate {
            job: job.clone(),
            lines: Vec::new(),
        });
        Ok(job)
    }

    /// Records this computer as a builder at the request's address and returns its token once
    /// (BLD-021, BLD-022).
    #[graphql(guard = "Authenticated")]
    async fn enrol_builder(
        &self,
        ctx: &Context<'_>,
        name: String,
        platform: PlatformFields,
    ) -> Result<EnrolmentPayload> {
        let trail = Trail::from_context(ctx)?;
        let input = BuilderInput {
            name,
            platform: platform.into(),
            client: trail.client.0,
        };
        let mut tx = begin(ctx).await?;
        let enrolment = super::enrol(&mut tx, trail.operator.id, &input, trail.now)
            .await
            .map_err(to_graphql)?;
        trail
            .write(&mut tx, trail::builder_enrolled(&enrolment.builder))
            .await?;
        finish(tx).await?;
        Ok(EnrolmentPayload(enrolment))
    }

    /// Deletes the builder and revokes its token (BLD-025). Returns the id.
    #[graphql(guard = "Authenticated")]
    async fn revoke_builder(&self, ctx: &Context<'_>, id: ID) -> Result<ID> {
        let builder = parse_id(&id, "id")?;
        let trail = Trail::from_context(ctx)?;
        let mut tx = begin(ctx).await?;
        let revoked = super::revoke(&mut tx, builder).await.map_err(to_graphql)?;
        trail
            .write(&mut tx, trail::builder_revoked(&revoked))
            .await?;
        finish(tx).await?;
        Ok(id)
    }
}

#[derive(Default)]
pub struct Subscription;

#[Subscription(name = "BuildsSubscription")]
impl Subscription {
    /// Each state change and each part of the log of `jobId` (of every job when null)
    /// (BLD-081).
    #[expect(
        clippy::unused_async,
        clippy::unused_async_trait_impl,
        reason = "the Subscription macro requires async resolvers"
    )]
    #[graphql(guard = "Authenticated")]
    async fn build_job_updated(
        &self,
        ctx: &Context<'_>,
        job_id: Option<ID>,
    ) -> Result<impl Stream<Item = JobUpdate>> {
        let job = job_id
            .as_ref()
            .map(|id| parse_id::<BuildJobId>(id, "jobId"))
            .transpose()?;
        let updates = schema_dependency::<Updates>(ctx)?;
        Ok(changes(updates.subscribe(), job))
    }
}

/// The changes of `job` (of any job when `None`). A subscriber that fell behind skips what it
/// missed; the stream ends when the broadcast closes.
fn changes(
    received: Receiver<JobUpdate>,
    job: Option<BuildJobId>,
) -> impl Stream<Item = JobUpdate> {
    stream::unfold(received, move |mut received| async move {
        loop {
            match received.recv().await {
                Ok(update) if job.is_none_or(|wanted| update.job.id == wanted) => {
                    return Some((update, received));
                }
                Ok(_) | Err(RecvError::Lagged(_)) => {}
                Err(RecvError::Closed) => return None,
            }
        }
    })
}

/// The time now by the schema's clock.
fn now(ctx: &Context<'_>) -> Result<chrono::DateTime<chrono::Utc>> {
    Ok(schema_dependency::<Arc<dyn Clock>>(ctx)?.now())
}

/// A `VALIDATION` error for one field.
fn invalid_field(field: &'static str, message: &str) -> async_graphql::Error {
    invalid(&[super::FieldError::new(field, message)])
}
