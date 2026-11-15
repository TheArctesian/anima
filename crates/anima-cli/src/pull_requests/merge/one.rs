//! Merging one pull request: read it, show it, confirm, then send the head the operator saw.

use super::consent::{expected_head, fork, short, summary};
use super::{MethodArg, outcomes, read};
use crate::failure::Failure;
use crate::forge::render::text;
use crate::invocation::Invocation;
use crate::projects::repository::resolve;
use crate::pull_requests::documents::{PullRequestMergeOne, pull_request_merge_one};

/// What the operator asked for one pull request.
pub struct Asked<'a> {
    /// The repository as the operator named it.
    pub name: &'a str,
    pub number: i64,
    pub method: MethodArg,
    pub sha: Option<&'a str>,
    pub allow_failing_checks: bool,
    pub fork: Option<&'a str>,
}

/// Merges the pull request, or says it was merged before without sending anything.
pub async fn merge(invocation: &Invocation, asked: &Asked<'_>) -> Result<(), Failure> {
    let hub = invocation.hub()?;
    let repository = resolve(&hub, asked.name).await?;
    let shown = format!("{}#{}", asked.name, asked.number);
    let pull = read(&hub, &repository, asked.name, asked.number).await?;
    match text(&pull, "state") {
        "MERGED" => {
            let row = [
                shown,
                "ALREADY_MERGED".to_owned(),
                "Merged before: nothing was changed.".to_owned(),
            ];
            return invocation.output.data(&pull, |_| outcomes(vec![row]));
        }
        "CLOSED" => {
            return Err(Failure::Conflict {
                message: format!("{shown} is closed, so it cannot merge. Nothing was changed."),
            });
        }
        _ => {}
    }
    let interactive = invocation.prompter.interactive();
    let head = expected_head(asked.sha, text(&pull, "headSha"), interactive)?;
    invocation.before_question(&summary(&shown, &pull, asked.method.word()));
    let fork_confirmed = fork(asked.fork, &pull, invocation.prompter)?;
    invocation.prompter.confirm(&format!(
        "Merge {shown} at {} by {}?",
        short(&head),
        asked.method.word()
    ))?;
    let variables = pull_request_merge_one::Variables {
        input: pull_request_merge_one::MergePullRequestInput {
            repository,
            number: asked.number,
            expected_head_sha: head,
            method: method(asked.method),
            allow_failing_checks: Some(asked.allow_failing_checks),
            fork_confirmed,
        },
    };
    let merged = hub
        .run::<PullRequestMergeOne>(variables)
        .await?
        .merge_pull_request;
    invocation.output.data(&merged, |merged| {
        let shown_merged = serde_json::to_value(merged).unwrap_or_default();
        let row = [
            shown.clone(),
            text(&shown_merged, "state").to_owned(),
            text(&shown_merged, "url").to_owned(),
        ];
        outcomes(vec![row])
    })
}

const fn method(method: MethodArg) -> pull_request_merge_one::MergeMethod {
    match method {
        MethodArg::Merge => pull_request_merge_one::MergeMethod::MERGE,
        MethodArg::Squash => pull_request_merge_one::MergeMethod::SQUASH,
        MethodArg::Rebase => pull_request_merge_one::MergeMethod::REBASE,
    }
}
