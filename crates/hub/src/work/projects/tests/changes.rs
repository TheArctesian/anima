//! The pure rules of a change to GitHub: the K7 code and fixed words of each outcome (K-E1
//! Section 13.7, decisions item 7) and the record it leaves (PRJ-028, LOG-007).

use async_graphql::{Error as GraphQlError, Value};
use chrono::{TimeZone as _, Utc};
use source_exchange::{
    Change, CloseReason, IssueState, Label, Labels, Number, Receipt, Refusal, RepositoryName,
    Settled, Text, Title,
};
use uuid::Uuid;

use crate::{
    engine::sources::LiveFailure,
    fleet::events::{AuditAction, AuditTarget, Severity},
    ids::RepositoryId,
    work::projects::forge::{
        failure::{self, Subject, UNKNOWN},
        trail,
    },
};

const MARK: &str = "OPERATOR-TEXT-4471";

fn code(error: &GraphQlError) -> Option<&Value> {
    error.extensions.as_ref()?.get("code")
}

fn said(error: &GraphQlError) -> (&str, String) {
    let code = match code(error) {
        Some(Value::String(code)) => code.clone(),
        other => panic!("no code: {other:?}"),
    };
    (error.message.as_str(), code)
}

fn create() -> Change {
    Change::CreateIssue {
        repository: RepositoryName::parse("octo/widgets").unwrap(),
        title: Title::parse(&format!("{MARK} title")).unwrap(),
        body: Some(Text::parse(&format!("{MARK} body")).unwrap()),
        labels: Labels::new(vec![Label::parse(MARK).unwrap()]).unwrap(),
    }
}

#[test]
fn od_e14_each_outcome_of_a_sent_change_has_its_code_and_fixed_words() {
    let until = Utc.with_ymd_and_hms(2030, 5, 1, 9, 7, 0).unwrap();
    let cases = [
        (
            Settled::Refused(Refusal::KeyRefused),
            "GitHub refused the key. Connect the account again.",
            "CONFLICT",
        ),
        (
            Settled::Refused(Refusal::NotPermitted),
            "The GitHub App is not permitted to do that. Check its permissions on GitHub.",
            "FORBIDDEN",
        ),
        (
            Settled::Refused(Refusal::NotFound),
            "GitHub has no such issue, or the App may not change it.",
            "NOT_FOUND",
        ),
        (
            Settled::Refused(Refusal::Gone),
            "Issues are off for this repository.",
            "CONFLICT",
        ),
        (
            Settled::Refused(Refusal::Invalid),
            "GitHub refused a value of the change. Nothing was changed.",
            "VALIDATION",
        ),
        (
            Settled::Refused(Refusal::Unexplained),
            "GitHub refused the change. Nothing was changed.",
            "UNAVAILABLE",
        ),
        (
            Settled::Limited { until },
            "GitHub limits requests until 09:07.",
            "UNAVAILABLE",
        ),
        (Settled::Unknown, UNKNOWN, "UNAVAILABLE"),
    ];
    for (settled, words, wire) in cases {
        let error = failure::settled(settled.clone(), Subject::Issue).unwrap_err();
        assert_eq!(said(&error), (words, wire.to_owned()), "{settled:?}");
    }
    let made = failure::settled(Settled::Done(Receipt::Made), Subject::Issue);
    assert_eq!(made.unwrap(), Receipt::Made);
    let missing = failure::refused(Refusal::NotFound, Subject::IssueOrPullRequest);
    assert_eq!(
        missing.message,
        "GitHub has no such issue or pull request, or the App may not change it."
    );
}

#[test]
fn od_e14_a_change_that_could_not_connect_says_nothing_was_changed() {
    let error = failure::unsent(LiveFailure::Unreachable);
    assert_eq!(
        said(&error),
        (
            "GitHub could not be reached. Nothing was changed.",
            "UNAVAILABLE".to_owned()
        )
    );
    assert_eq!(said(&failure::unsent(LiveFailure::Paused)).1, "CONFLICT");
}

#[test]
fn prj_028_the_record_names_function_repository_target_and_outcome_never_the_text() {
    let repository = RepositoryId::from(Uuid::now_v7());
    let outcomes = [
        (
            Settled::Done(Receipt::Created { number: 7 }),
            "done",
            "issue#7",
        ),
        (Settled::Refused(Refusal::Invalid), "refused", "issue#new"),
        (
            Settled::Limited {
                until: Utc.with_ymd_and_hms(2030, 5, 1, 9, 7, 0).unwrap(),
            },
            "limited",
            "issue#new",
        ),
        (Settled::Unknown, "unknown", "issue#new"),
    ];
    for (settled, outcome, target) in outcomes {
        let entry = trail::entry(&create(), (repository, "widgets"), &settled);
        assert_eq!(entry.change["function"], "create_issue");
        assert_eq!(entry.change["repository"], "octo/widgets");
        assert_eq!(entry.change["target"], target);
        assert_eq!(entry.change["outcome"], outcome);
        assert_eq!(entry.action, AuditAction::Create);
        assert_eq!(entry.target, AuditTarget::Repository(repository));
        let expected = if outcome == "done" {
            Severity::Info
        } else {
            Severity::Warning
        };
        assert_eq!(entry.severity, expected);
        assert!(
            entry
                .message
                .starts_with("Project widgets: create_issue on ")
        );
        let all = format!("{} {}", entry.message, entry.change);
        assert!(!all.contains(MARK), "{all}");
    }
    let refused = trail::entry(
        &create(),
        (repository, "widgets"),
        &Settled::Refused(Refusal::Gone),
    );
    assert_eq!(refused.change["refusal"], "gone");
}

#[test]
fn prj_028_closing_an_issue_is_an_update_with_its_number_and_reason() {
    let close = Change::SetIssueOpen {
        repository: RepositoryName::parse("octo/widgets").unwrap(),
        number: Number::new(3).unwrap(),
        state: IssueState::Closed(Some(CloseReason::NotPlanned)),
    };
    let entry = trail::entry(
        &close,
        (RepositoryId::from(Uuid::now_v7()), "widgets"),
        &Settled::Done(Receipt::Made),
    );
    assert_eq!(entry.action, AuditAction::Update);
    assert_eq!(entry.change["target"], "issue#3");
    assert_eq!(entry.change["open"], false);
    assert_eq!(entry.change["reason"], "not_planned");
}
