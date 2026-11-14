//! The jobs of a run, counted as K1's `WorkflowJobs` (PRJ-031). Pure.

use async_graphql::SimpleObject;
use provider_github::live::runs::{Conclusion, Job, Jobs, RunStatus};

use crate::work::projects::forge::{int, text};

/// The most names of failed jobs, and the most characters of each.
const MOST_FAILING: usize = 10;
const NAME_LENGTH: usize = 100;

#[derive(Clone, Debug, Default, PartialEq, Eq, SimpleObject)]
#[graphql(name = "WorkflowJobs")]
pub struct JobsView {
    pub total: i32,
    pub succeeded: i32,
    pub failed: i32,
    pub running: i32,
    pub queued: i32,
    /// Skipped or cancelled.
    pub other: i32,
    /// Up to 10 names of failed jobs, each at most 100 characters. Text from GitHub.
    pub failing: Vec<String>,
    /// True: the run has more than 100 jobs; the counts cover the first 100.
    pub partial: bool,
}

/// How one job counts.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Tally {
    Succeeded,
    Failed,
    Running,
    Queued,
    Other,
}

/// A finished job by its conclusion: failed, timed out and action required are failed;
/// skipped, cancelled, neutral, stale and values the hub does not know are other.
const fn tally(job: &Job) -> Tally {
    match (job.status, job.conclusion) {
        (RunStatus::Completed, Some(Conclusion::Success)) => Tally::Succeeded,
        (
            RunStatus::Completed,
            Some(Conclusion::Failure | Conclusion::TimedOut | Conclusion::ActionRequired),
        ) => Tally::Failed,
        (RunStatus::InProgress, _) => Tally::Running,
        (RunStatus::Queued | RunStatus::Waiting | RunStatus::Requested | RunStatus::Pending, _) => {
            Tally::Queued
        }
        _ => Tally::Other,
    }
}

/// The counts of the first page of a run's `jobs`; `total` is GitHub's count of all of them.
#[must_use]
pub fn count(jobs: &Jobs) -> JobsView {
    let mut view = JobsView {
        total: int(jobs.total_count),
        partial: jobs.total_count > jobs.jobs.len() as u64,
        ..JobsView::default()
    };
    for job in &jobs.jobs {
        match tally(job) {
            Tally::Succeeded => view.succeeded += 1,
            Tally::Running => view.running += 1,
            Tally::Queued => view.queued += 1,
            Tally::Other => view.other += 1,
            Tally::Failed => {
                view.failed += 1;
                if view.failing.len() < MOST_FAILING {
                    view.failing
                        .push(text::capped(job.name.clone(), NAME_LENGTH));
                }
            }
        }
    }
    view
}

/// The run has jobs and every one has finished, so the answer will not change soon.
#[must_use]
pub fn settled(jobs: &Jobs) -> bool {
    !jobs.jobs.is_empty()
        && jobs
            .jobs
            .iter()
            .all(|job| job.status == RunStatus::Completed)
}
