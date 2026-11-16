//! The pure rules of a live read: a repository's access from its account and the operator's
//! choices (PRJ-032), the cut of GitHub's text, and the count of a run's jobs (PRJ-031).

use provider_github::live::runs::{Conclusion, Job, Jobs, RunStatus};

use crate::{
    engine::sources::Standing,
    work::projects::{
        forge::{
            reach::{RepositoryAccess, access},
            text,
        },
        workflows::jobs::{count, settled},
    },
};

#[test]
fn prj_032_access_names_the_first_reason_a_repository_cannot_be_read() {
    let ready = Some(Standing::Ready);
    for (left_out, gone, standing, expected) in [
        (false, false, ready, RepositoryAccess::Ready),
        (false, false, None, RepositoryAccess::NotConnected),
        (
            true,
            true,
            Some(Standing::NotFound),
            RepositoryAccess::NotConnected,
        ),
        (true, false, ready, RepositoryAccess::LeftOut),
        (
            true,
            true,
            Some(Standing::Paused),
            RepositoryAccess::LeftOut,
        ),
        (false, true, ready, RepositoryAccess::Gone),
        (
            false,
            false,
            Some(Standing::Paused),
            RepositoryAccess::Paused,
        ),
        (
            false,
            false,
            Some(Standing::NeedsReconnect),
            RepositoryAccess::NeedsReconnect,
        ),
    ] {
        assert_eq!(access(left_out, gone, standing), expected);
    }
}

#[test]
fn prj_031_text_is_cut_on_a_character_boundary_and_marked() {
    let (cut, was_cut) = text::cut("ééé<b>".to_owned(), 2);
    assert_eq!((cut.as_str(), was_cut), ("éé", true));
    let (whole, was_cut) = text::cut("<script>\u{1b}".to_owned(), 9);
    assert_eq!((whole.as_str(), was_cut), ("<script>\u{1b}", false));
    let (body, was_cut) = text::body(Some("x".repeat(text::BODY + 1)));
    assert_eq!(
        (body.map(|b| b.chars().count()), was_cut),
        (Some(text::BODY), true)
    );
    assert_eq!(text::body(None), (None, false));
}

fn job(name: &str, status: RunStatus, conclusion: Option<Conclusion>) -> Job {
    Job {
        id: 1,
        name: name.to_owned(),
        status,
        conclusion,
    }
}

#[test]
fn prj_031_jobs_are_counted_by_their_state() {
    let done = RunStatus::Completed;
    let jobs = Jobs {
        total_count: 120,
        jobs: vec![
            job("build", done, Some(Conclusion::Success)),
            job("lint", done, Some(Conclusion::Failure)),
            job(&"t".repeat(150), done, Some(Conclusion::TimedOut)),
            job("docs", done, Some(Conclusion::Skipped)),
            job("deploy", done, Some(Conclusion::Cancelled)),
            job("e2e", RunStatus::InProgress, None),
            job("pack", RunStatus::Queued, None),
            job("sign", RunStatus::Waiting, None),
        ],
    };
    let view = count(&jobs);
    assert_eq!(
        (
            view.total,
            view.succeeded,
            view.failed,
            view.running,
            view.queued,
            view.other
        ),
        (120, 1, 2, 1, 2, 2)
    );
    assert_eq!(view.failing, ["lint".to_owned(), "t".repeat(100)]);
    assert!(view.partial, "120 jobs, 8 given");
    assert!(!settled(&jobs));
}

#[test]
fn prj_031_a_run_with_no_jobs_yet_is_not_settled() {
    let none = Jobs {
        total_count: 0,
        jobs: Vec::new(),
    };
    assert!(!settled(&none));
    assert!(!count(&none).partial);
    let finished = Jobs {
        total_count: 1,
        jobs: vec![job(
            "build",
            RunStatus::Completed,
            Some(Conclusion::Success),
        )],
    };
    assert!(settled(&finished));
}
