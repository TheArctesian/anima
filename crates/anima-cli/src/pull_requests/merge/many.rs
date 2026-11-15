//! Merging several pull requests one after another: read them all, show them, confirm once,
//! then one outcome for each. Never one from a fork (PRO-013).

use super::consent::short;
use super::{MethodArg, outcomes, read};
use crate::failure::{Failure, Hint, exit};
use crate::forge::render::{flag, text};
use crate::invocation::Invocation;
use crate::output::text::enum_name;
use crate::projects::repository::resolve;
use crate::pull_requests::documents::{PullRequestMergeMany, pull_request_merge_many};

type Merged = pull_request_merge_many::PullRequestMergeManyMergePullRequests;

/// Merges `numbers` of the repository `name` in that order.
pub async fn merge(
    invocation: &Invocation,
    name: &str,
    numbers: &[i64],
    method: MethodArg,
) -> Result<(), Failure> {
    let hub = invocation.hub()?;
    let repository = resolve(&hub, name).await?;
    let mut targets = Vec::with_capacity(numbers.len());
    let mut lines = Vec::with_capacity(numbers.len());
    for &number in numbers {
        let pull = read(&hub, &repository, name, number).await?;
        if flag(&pull, "fromFork") {
            return Err(Failure::Usage {
                message: format!(
                    "{name}#{number} is from a fork: merge it alone, with --fork (PRO-013). Nothing was merged."
                ),
                hint: Some(Hint::say("Leave it out of this list.")),
            });
        }
        let head = text(&pull, "headSha").to_owned();
        lines.push(format!(
            "{name}#{number} at {}: {}",
            short(&head),
            text(&pull, "title")
        ));
        targets.push(pull_request_merge_many::PullRequestTargetInput {
            repository: repository.clone(),
            number,
            expected_head_sha: head,
        });
    }
    invocation.before_question(&lines.join("\n"));
    invocation.prompter.confirm(&format!(
        "Merge these {} pull requests of {name} by {}, one after another?",
        numbers.len(),
        method.word()
    ))?;
    let variables = pull_request_merge_many::Variables {
        pull_requests: targets,
        method: k1(method),
    };
    let results = hub
        .run::<PullRequestMergeMany>(variables)
        .await?
        .merge_pull_requests;
    invocation
        .output
        .data(&results, |results| outcomes(rows(name, results)))?;
    let missed = results.iter().filter(|result| !merged(result)).count();
    if missed > 0 {
        return Err(Failure::Ended {
            message: format!(
                "{missed} of {} pull requests were not merged.",
                results.len()
            ),
            status: exit::FAILURE,
        });
    }
    Ok(())
}

/// Whether the pull request is merged now, by this request or before it.
const fn merged(result: &Merged) -> bool {
    matches!(
        result.outcome,
        pull_request_merge_many::MergeOutcome::MERGED
            | pull_request_merge_many::MergeOutcome::ALREADY_MERGED
    )
}

fn rows(name: &str, results: &[Merged]) -> Vec<[String; 3]> {
    results
        .iter()
        .map(|result| {
            let detail = match &result.merge_sha {
                Some(sha) => format!("{} {}", result.message, short(sha)),
                None => result.message.clone(),
            };
            [
                format!("{name}#{}", result.number),
                enum_name(&result.outcome),
                detail,
            ]
        })
        .collect()
}

const fn k1(method: MethodArg) -> pull_request_merge_many::MergeMethod {
    match method {
        MethodArg::Merge => pull_request_merge_many::MergeMethod::MERGE,
        MethodArg::Squash => pull_request_merge_many::MergeMethod::SQUASH,
        MethodArg::Rebase => pull_request_merge_many::MergeMethod::REBASE,
    }
}
