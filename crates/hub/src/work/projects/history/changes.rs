//! A commit status written by Anima (K1; OD-E14, PRO-010, PRJ-028).
//!
//! Its context is fenced to `anima/`, so Anima cannot write the status of another system; its
//! address, when it has one, is below the hub's own public origin (`ANIMA_PUBLIC_ORIGIN`); its
//! description is at most 140 characters. A status can satisfy a required check and so open a
//! merge: login or browser token only (D-23, R-100; K-E1 Section 13.5).
#![expect(
    clippy::unused_async_trait_impl,
    reason = "the async-graphql Enum derive generates async trait impls"
)]

use std::sync::Arc;

use async_graphql::{Context, Enum, ID, InputObject, Object, Result, SimpleObject};
use chrono::{DateTime, Utc};
use source_exchange::{
    Change, CommitSha, RepositoryName, StatusContext, StatusDescription, StatusState, TargetUrl,
};

use crate::{
    api_errors::{FieldError, invalid, schema_dependency},
    clock::Clock,
    config::Config,
    fleet::access::{Authenticated, operator_only},
    work::projects::forge::{self, Subject, Target, failure},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
pub enum CommitStatusState {
    Pending,
    Success,
    Failure,
    Error,
}

impl CommitStatusState {
    const fn part(self) -> StatusState {
        match self {
            Self::Pending => StatusState::Pending,
            Self::Success => StatusState::Success,
            Self::Failure => StatusState::Failure,
            Self::Error => StatusState::Error,
        }
    }
}

#[derive(InputObject)]
#[graphql(name = "CommitStatusInput")]
pub struct CommitStatusInput {
    repository: ID,
    /// 40 hex digits.
    sha: String,
    state: CommitStatusState,
    /// anima/ then 1 to 40 of a-z, 0-9 and -. Anima cannot write the context of another system.
    context: String,
    /// At most 140 characters.
    description: Option<String>,
    /// Null, or an address below the hub's public origin.
    target_url: Option<String>,
}

#[derive(SimpleObject)]
#[graphql(name = "CommitStatus")]
pub struct CommitStatusView {
    repository: ID,
    sha: String,
    context: String,
    state: CommitStatusState,
    description: Option<String>,
    target_url: Option<String>,
    created_at: DateTime<Utc>,
}

/// The checked parts of a status.
struct Status {
    sha: CommitSha,
    context: StatusContext,
    description: Option<StatusDescription>,
    target_url: Option<TargetUrl>,
}

#[derive(Default)]
pub struct Mutation;

#[Object(name = "HistoryMutation")]
impl Mutation {
    /// Writes a commit status under an anima/ context (PRO-010). Writes an audit entry (PRJ-028). Login or browser token only (D-23, R-100).
    #[graphql(guard = "Authenticated")]
    async fn set_commit_status(
        &self,
        ctx: &Context<'_>,
        input: CommitStatusInput,
    ) -> Result<CommitStatusView> {
        operator_only(ctx, "setCommitStatus")?;
        let origin = &schema_dependency::<Arc<Config>>(ctx)?.public_origin;
        let status = status(&input, origin)?;
        let target = forge::target(ctx, &input.repository).await?;
        let repository = RepositoryName::parse(&target.name).map_err(|_| failure::unnamed())?;
        let change = Change::SetCommitStatus {
            repository,
            sha: status.sha.clone(),
            state: input.state.part(),
            context: status.context.clone(),
            description: status.description.clone(),
            target_url: status.target_url.clone(),
        };
        forge::make(ctx, &target, &change, Subject::Commit).await?;
        let accepted = schema_dependency::<Arc<dyn Clock>>(ctx)?.now();
        Ok(view(&target, input.state, status, accepted))
    }
}

/// The status GitHub accepted, as the hub sent it, stamped with the hub's time of the answer:
/// GitHub's reply is not read back (LOG-007 keeps its text out of the outcome).
fn view(
    target: &Target,
    state: CommitStatusState,
    status: Status,
    accepted: DateTime<Utc>,
) -> CommitStatusView {
    CommitStatusView {
        repository: ID(target.repository.to_string()),
        sha: status.sha.as_str().to_owned(),
        context: status.context.as_str().to_owned(),
        state,
        description: status.description.map(|d| d.as_str().to_owned()),
        target_url: status.target_url.map(|u| u.as_str().to_owned()),
        created_at: accepted,
    }
}

/// The parts of `input`, with its address checked against `origin`, or one `VALIDATION` error
/// naming each field that is not one. An empty description or address is none. The words are
/// fixed: never the value given (LOG-007).
fn status(input: &CommitStatusInput, origin: &str) -> Result<Status> {
    let sha = CommitSha::parse(&input.sha)
        .map_err(|_| FieldError::new("sha", "Give the commit as 40 hexadecimal digits."));
    let context = StatusContext::parse(&input.context).map_err(|_| {
        FieldError::new(
            "context",
            "Give anima/ then 1 to 40 of a-z, 0-9 and -. Anima writes only its own contexts.",
        )
    });
    let description = given(input.description.as_deref())
        .map(StatusDescription::parse)
        .transpose()
        .map_err(|_| FieldError::new("description", "Give at most 140 characters on one line."));
    let target_url = given(input.target_url.as_deref())
        .map(|url| TargetUrl::parse(url, origin))
        .transpose()
        .map_err(|_| {
            FieldError::new(
                "targetUrl",
                "Give an address below the hub's own public address, or none.",
            )
        });
    match (sha, context, description, target_url) {
        (Ok(sha), Ok(context), Ok(description), Ok(target_url)) => Ok(Status {
            sha,
            context,
            description,
            target_url,
        }),
        (sha, context, description, target_url) => Err(invalid(
            &[
                sha.err(),
                context.err(),
                description.err(),
                target_url.err(),
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>(),
        )),
    }
}

/// `text` when it is set and not empty.
fn given(text: Option<&str>) -> Option<&str> {
    text.filter(|text| !text.is_empty())
}
