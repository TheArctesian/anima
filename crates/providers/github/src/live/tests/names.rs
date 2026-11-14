//! PRJ-030: a repository name or a ref that could move a request is refused before a path is
//! built, and a reply that cannot be read is a problem, never a panic.

use source_exchange::{Body, Response};

use super::ok;
use crate::live::{StateFilter, checks, comments, issues, pulls, runs, workflows};

const HOSTILE: [&str; 8] = [
    "../../../app/installations",
    "owner/../x",
    "owner/..",
    "./name",
    "owner/name/extra",
    "owner/na me",
    "owner@host/name",
    "owner/name?x=1",
];

#[test]
fn prj_030_a_hostile_repository_name_builds_no_request_in_any_read() {
    for name in HOSTILE.into_iter().chain(["", "owner", "/name", "owner/"]) {
        let all = [
            pulls::list_request(name, StateFilter::Open, 1),
            pulls::one_request(name, 1),
            pulls::merge_methods_request(name),
            issues::list_request(name, StateFilter::All, 1),
            issues::one_request(name, 1),
            comments::request(name, 1, 1),
            runs::list_request(name, runs::RunFilter::default(), 1),
            runs::one_request(name, 1),
            runs::jobs_request(name, 1, 1),
            workflows::list_request(name, 1),
            checks::check_runs_request(name, "main", 1),
            checks::status_request(name, "main", 1),
        ];
        for result in all {
            let problem = result.unwrap_err();
            assert_eq!(problem.code, "repository_name_unusable", "{name}");
            assert!(!problem.message.contains(name) || name.is_empty());
        }
    }
}

#[test]
fn prj_030_a_ref_with_a_dot_segment_builds_no_request() {
    for git_ref in ["..", "a/../b", "./x", ""] {
        assert_eq!(
            checks::check_runs_request("o/r", git_ref, 1)
                .unwrap_err()
                .code,
            "ref_unusable"
        );
        assert_eq!(
            checks::status_request("o/r", git_ref, 1).unwrap_err().code,
            "ref_unusable"
        );
    }
}

#[test]
fn prj_032_a_malformed_body_is_a_problem_in_every_parse() {
    let junk = ok(b"{ not json");
    let wrong_shape = ok(b"{\"unexpected\": true}");
    for response in [&junk, &wrong_shape] {
        assert_eq!(
            pulls::list_parse(response).unwrap_err().code,
            "reply_unreadable"
        );
        assert_eq!(
            pulls::one_parse(response).unwrap_err().code,
            "reply_unreadable"
        );
        assert_eq!(
            pulls::merge_methods_parse(response).unwrap_err().code,
            "reply_unreadable"
        );
        assert_eq!(
            issues::list_parse(response).unwrap_err().code,
            "reply_unreadable"
        );
        assert_eq!(
            issues::one_parse(response).unwrap_err().code,
            "reply_unreadable"
        );
        assert_eq!(
            comments::parse(response).unwrap_err().code,
            "reply_unreadable"
        );
        assert_eq!(
            runs::list_parse(response).unwrap_err().code,
            "reply_unreadable"
        );
        assert_eq!(
            runs::one_parse(response).unwrap_err().code,
            "reply_unreadable"
        );
        assert_eq!(
            runs::jobs_parse(response).unwrap_err().code,
            "reply_unreadable"
        );
        assert_eq!(
            workflows::list_parse(response).unwrap_err().code,
            "reply_unreadable"
        );
        assert_eq!(
            checks::check_runs_parse(response).unwrap_err().code,
            "reply_unreadable"
        );
        assert_eq!(
            checks::status_parse(response).unwrap_err().code,
            "reply_unreadable"
        );
    }
}

#[test]
fn prj_032_a_status_other_than_200_and_a_too_large_body_are_problems_with_fixed_words() {
    let not_found = Response {
        status: 404,
        body: Body::Read(b"{\"message\":\"secret words from GitHub\"}"),
    };
    let problem = pulls::list_parse(&not_found).unwrap_err();
    assert_eq!(problem.code, "unexpected_status");
    assert_eq!(problem.message, "GitHub answered with status 404");

    let too_large = Response {
        status: 200,
        body: Body::TooLarge,
    };
    assert_eq!(
        runs::list_parse(&too_large).unwrap_err().code,
        "reply_too_large"
    );
}

#[test]
fn prj_032_one_entry_missing_a_required_field_is_a_problem_not_a_panic() {
    let body = br#"[{"number": 1}]"#;

    assert_eq!(
        pulls::list_parse(&ok(body)).unwrap_err().code,
        "reply_unreadable"
    );
    assert_eq!(
        issues::list_parse(&ok(body)).unwrap_err().code,
        "reply_unreadable"
    );
}
