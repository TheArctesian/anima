//! `anima pr merge <repository> <number>...`: merges one pull request, or up to twenty one after
//! another, through the hub (PRJ-029, PRJ-033, PRO-013). Each merge sends the head commit the
//! operator saw, so a pull request that changed since is refused.

mod consent;
mod many;
mod one;

use serde_json::Value;

use super::documents::{PullRequestGet, pull_request_get};
use crate::failure::{Failure, Hint};
use crate::hub::Hub;
use crate::invocation::Invocation;
use crate::output::Table;

/// The most pull requests one bulk merge takes; the hub refuses more.
const MAX_BULK: usize = 20;

#[derive(Debug, clap::Args)]
pub struct MergeArgs {
    /// The repository: `owner/name` or its ID.
    repository: String,
    /// The pull request's number; give up to 20 to merge them one after another.
    #[arg(required = true, num_args = 1..=MAX_BULK, value_parser = clap::value_parser!(i64).range(1..))]
    numbers: Vec<i64>,
    /// How to merge; the repository must allow it.
    #[arg(long, value_enum)]
    method: MethodArg,
    /// The 40-digit head commit you looked at. Required for one number when no terminal shows
    /// the head before you confirm.
    #[arg(long, value_name = "SHA")]
    sha: Option<String>,
    /// Also merge when a check that is not required failed. One number only.
    #[arg(long)]
    allow_failing_checks: bool,
    /// For a pull request from a fork: its head repository, typed exactly. One number only;
    /// `--yes` does not stand for it.
    #[arg(long, value_name = "OWNER/NAME")]
    fork: Option<String>,
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
pub enum MethodArg {
    Merge,
    Squash,
    Rebase,
}

impl MethodArg {
    /// The word as `--method` takes it.
    const fn word(self) -> &'static str {
        match self {
            Self::Merge => "merge",
            Self::Squash => "squash",
            Self::Rebase => "rebase",
        }
    }
}

impl MergeArgs {
    /// Reads each pull request, shows it, asks once (or takes `--yes`, CMD-024) and merges. One
    /// line for each pull request; with `--json`, the K1 `PullRequest` for one number and the
    /// list of K1 `PullRequestMerge` for several.
    ///
    /// # Errors
    /// [`Failure::Usage`] for flags that need one number, a number given twice, or a missing
    /// `--sha` or `--fork`; [`Failure::Declined`] when the operator says no; the hub's refusal;
    /// [`Failure::Ended`] (exit 1) when any of several was not merged.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        if let [number] = self.numbers[..] {
            let asked = one::Asked {
                name: &self.repository,
                number,
                method: self.method,
                sha: self.sha.as_deref(),
                allow_failing_checks: self.allow_failing_checks,
                fork: self.fork.as_deref(),
            };
            return one::merge(invocation, &asked).await;
        }
        if self.sha.is_some() || self.fork.is_some() || self.allow_failing_checks {
            return Err(Failure::Usage {
                message: "--sha, --fork and --allow-failing-checks take one pull request: merge it alone.".to_owned(),
                hint: None,
            });
        }
        let mut seen = self.numbers.clone();
        seen.sort_unstable();
        seen.dedup();
        if seen.len() != self.numbers.len() {
            return Err(Failure::Usage {
                message: "A pull request number is given twice.".to_owned(),
                hint: None,
            });
        }
        many::merge(invocation, &self.repository, &self.numbers, self.method).await
    }
}

/// The pull request as the hub reads it from GitHub now, never from its cache.
async fn read(hub: &Hub, repository: &str, shown: &str, number: i64) -> Result<Value, Failure> {
    let variables = pull_request_get::Variables {
        repository: repository.to_owned(),
        number,
        fresh: Some(true),
    };
    let pull = hub
        .run::<PullRequestGet>(variables)
        .await?
        .pull_request
        .ok_or_else(|| Failure::NotFound {
            message: format!("{shown} has no pull request number {number}."),
            hint: Some(Hint::run(format!("anima pr list {shown}"))),
        })?;
    Ok(serde_json::to_value(&pull).unwrap_or_default())
}

/// One line for each pull request: which, the outcome, and what to know of it.
fn outcomes(rows: Vec<[String; 3]>) -> Table {
    let mut table = Table::new(vec!["PULL REQUEST", "OUTCOME", "DETAIL"]);
    for [pull, outcome, detail] in rows {
        table.row(vec![pull.into(), outcome.as_str().into(), detail.into()]);
    }
    table
}
