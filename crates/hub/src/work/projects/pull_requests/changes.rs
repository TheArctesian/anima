//! The merge changes (K1; OD-E14, PRJ-027, PRJ-028, PRJ-033): merge one pull request, or many
//! one after another.
//!
//! Each merge checks its input first, then reads the pull request and the repository's merge
//! methods fresh from GitHub, and applies the merge rule ([`merge::decide`]); a refusal there
//! sends nothing and records nothing. The merge is sent once, bound to the head commit the
//! caller saw, and recorded by [`forge::send`]. Both roots are login or browser token only
//! (D-23, R-100; K-E1 Section 13.5): a merge makes code run with the repository's secrets. A
//! pull request from a fork is a person's trust decision (PRO-013).

use std::{sync::Arc, time::Duration};

use async_graphql::{Context, Object, Result};
use provider_github::live::pulls::PullDetail;
use source_exchange::{Change, MergeMethod as Method, RepositoryName, Settled};

use super::{
    back, bulk,
    bulk::PullRequestMerge,
    input::{self, Entry, MergePullRequestInput, PullRequestTargetInput, Seen},
    merge::{self, Ask, Decision, Merging, Refusal},
    views::{MergeMethod, PullRequestView, detailed},
};
use crate::{
    api_errors::{coded, schema_dependency},
    clock::Clock,
    fleet::{
        access::{Authenticated, operator_only},
        events::{ActorKind, Operator},
    },
    ids::RepositoryId,
    work::projects::{
        forge::{self, Asked, Subject, Target, failure, missing},
        on,
    },
};

/// How often the hub reads a pull request whose mergeability GitHub has not worked out, and
/// how long it waits between two reads (K-E1 Section 13.3).
const READS: u32 = 3;
const READ_GAP: Duration = Duration::from_secs(1);

/// What came of one merge the rule allowed or refused.
enum Step {
    /// GitHub has no such pull request.
    Missing,
    AlreadyMerged(Box<PullDetail>),
    Refused(Refusal),
    Sent(Settled),
}

#[derive(Default)]
pub struct Mutation;

#[Object(name = "PullRequestsMutation")]
impl Mutation {
    /// CONFLICT when the head moved, it cannot merge, GitHub has not computed it, or the method is not allowed. FORBIDDEN for a pull request from a fork without forkConfirmed, and for any pull request from a fork when the request declares an AI agent (PRO-013). Returns it unchanged when it is already merged. Writes an audit entry (PRJ-028). Login or browser token only (D-23, R-100).
    #[graphql(guard = "Authenticated")]
    async fn merge_pull_request(
        &self,
        ctx: &Context<'_>,
        input: MergePullRequestInput,
    ) -> Result<PullRequestView> {
        operator_only(ctx, "mergePullRequest")?;
        let seen = input::one(&input)?;
        // Boxed, as below: the merged Mutation holds every resolver's future, and a merge's
        // reads and send are deep.
        Box::pin(alone(ctx, &input, &seen)).await
    }

    /// 1 to 20 pull requests, no pair twice, merged one after another in the order given; a failure does not stop the rest. One result for each, in the same order (PRJ-033). Only CLEAN ones; never one from a fork. One audit entry for each one the hub sent to GitHub. Login or browser token only (D-23, R-100).
    #[graphql(guard = "Authenticated")]
    async fn merge_pull_requests(
        &self,
        ctx: &Context<'_>,
        pull_requests: Vec<PullRequestTargetInput>,
        method: MergeMethod,
    ) -> Result<Vec<PullRequestMerge>> {
        operator_only(ctx, "mergePullRequests")?;
        let entries = input::many(&pull_requests)?;
        Box::pin(many(ctx, &entries, method)).await
    }
}

/// The merge of one pull request, as `input` asks and the caller `seen` it.
async fn alone(
    ctx: &Context<'_>,
    input: &MergePullRequestInput,
    seen: &Seen,
) -> Result<PullRequestView> {
    let target = forge::target(ctx, &input.repository).await?;
    let methods = back::methods(ctx, &target).await?;
    let ask = Ask {
        method: input.method,
        expected_head: seen.head.as_str(),
        agent: declares_agent(ctx)?,
        merging: Merging::Alone {
            allow_failing_checks: input.allow_failing_checks.unwrap_or(false),
            fork_confirmed: input.fork_confirmed.as_deref(),
        },
    };
    match step(ctx, &target, (seen, &ask), &methods).await? {
        Step::Missing => Err(missing("pull request")),
        Step::AlreadyMerged(pull) => Ok(detailed(*pull, asked(&target))),
        Step::Refused(refusal) => Err(coded(refusal.words(), refusal.code())),
        Step::Sent(settled) => {
            failure::settled(settled, Subject::PullRequest)?;
            back::merged(ctx, &target, seen.number.get()).await
        }
    }
}

/// Each of `entries` in order, merged with `method` one after another until the deadline; a
/// failure does not stop the rest, the hub's own failure does.
async fn many(
    ctx: &Context<'_>,
    entries: &[Entry],
    method: MergeMethod,
) -> Result<Vec<PullRequestMerge>> {
    on(ctx).await?;
    let clock = schema_dependency::<Arc<dyn Clock>>(ctx)?;
    let started = clock.now();
    let agent = declares_agent(ctx)?;
    let mut methods = Vec::new();
    let mut results = Vec::with_capacity(entries.len());
    for entry in entries {
        if bulk::past(started, clock.now()) {
            results.push(bulk::not_attempted(entry));
            continue;
        }
        match one_of_many(ctx, entry, (method, agent), &mut methods).await {
            Ok(result) => results.push(result),
            Err(error) if bulk::is_internal(&error) => return Err(error),
            Err(error) => results.push(bulk::failed(entry, &error)),
        }
    }
    Ok(results)
}

/// One merge of a bulk merge, with `method` for a request that declares an `agent` or not.
/// `known` holds the merge methods of each repository read so far in this request.
async fn one_of_many(
    ctx: &Context<'_>,
    entry: &Entry,
    (method, agent): (MergeMethod, bool),
    known: &mut Vec<(RepositoryId, Vec<MergeMethod>)>,
) -> Result<PullRequestMerge> {
    let target = forge::target(ctx, &entry.repository).await?;
    let held = known.iter().find(|(id, _)| *id == target.repository);
    let methods = if let Some((_, methods)) = held {
        methods.clone()
    } else {
        let methods = back::methods(ctx, &target).await?;
        known.push((target.repository, methods.clone()));
        methods
    };
    let ask = Ask {
        method,
        expected_head: entry.seen.head.as_str(),
        agent,
        merging: Merging::InBulk,
    };
    Ok(
        match step(ctx, &target, (&entry.seen, &ask), &methods).await? {
            Step::Missing => bulk::missing(entry),
            Step::AlreadyMerged(_) => bulk::already(entry),
            Step::Refused(refusal) => bulk::refused(entry, refusal),
            Step::Sent(settled) => bulk::sent(entry, settled),
        },
    )
}

/// Reads the pull request `seen` names, applies the rule to what `ask` asks in a repository that
/// allows `methods`, and sends the merge when the rule allows it.
async fn step(
    ctx: &Context<'_>,
    target: &Target,
    (seen, ask): (&Seen, &Ask<'_>),
    methods: &[MergeMethod],
) -> Result<Step> {
    let mut reads = 1;
    let (pull, decision) = loop {
        let Some(pull) = back::before(ctx, target, seen.number.get()).await? else {
            return Ok(Step::Missing);
        };
        let decision = merge::decide(&pull, methods, ask);
        if decision != Decision::Refused(Refusal::NotReady) || reads == READS {
            break (pull, decision);
        }
        reads += 1;
        tokio::time::sleep(READ_GAP).await;
    };
    match decision {
        Decision::AlreadyMerged => Ok(Step::AlreadyMerged(Box::new(pull))),
        Decision::Refused(refusal) => Ok(Step::Refused(refusal)),
        Decision::Merge => {
            let change = Change::MergePullRequest {
                repository: RepositoryName::parse(&target.name).map_err(|_| failure::unnamed())?,
                number: seen.number,
                head: seen.head.clone(),
                method: sent_method(ask.method),
            };
            Ok(Step::Sent(forge::send(ctx, target, &change).await?))
        }
    }
}

/// Whether the request declares an AI agent, or its key was made for one (LOG-016, LOG-017).
fn declares_agent(ctx: &Context<'_>) -> Result<bool> {
    let operator = schema_dependency::<Operator>(ctx)?;
    Ok(operator.actor.kind() == ActorKind::Agent)
}

fn asked(target: &Target) -> Asked {
    Asked {
        target: target.clone(),
        fresh: false,
    }
}

const fn sent_method(method: MergeMethod) -> Method {
    match method {
        MergeMethod::Merge => Method::Merge,
        MergeMethod::Squash => Method::Squash,
        MergeMethod::Rebase => Method::Rebase,
    }
}
