//! `anima release` against the scripted hub (BLD-001, BLD-048, CMD-024, CMD-026).

use serde_json::{Value, json};

use crate::kit::Anima;
use crate::kit::fake_hub::{FakeHub, data, refusal};

const TOKEN: &str = "login-token-for-release-tests";
const JOB: &str = "0190a000-0000-7000-9002-000000000001";
const COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";

fn signed_in() -> Anima {
    Anima::new().with("ANIMA_TOKEN", TOKEN)
}

fn release(version: &str) -> Value {
    json!({
        "id": "0190a000-0000-7000-9003-000000000001", "version": version, "commit": COMMIT,
        "createdAt": "2026-11-10T09:00:00+00:00", "createdBy": "operator",
        "jobs": [
            { "id": JOB, "target": "LINUX_X86_64", "state": "PUBLISHED", "failureReason": null },
            { "id": "0190a000-0000-7000-9002-000000000002", "target": "DARWIN_UNIVERSAL",
              "state": "FAILED", "failureReason": "xcode" },
        ],
    })
}

fn releases_page(next: Option<&str>) -> Value {
    data(json!({ "releases": {
        "edges": [{ "cursor": "r1", "node": release("0.4.0") }],
        "pageInfo": { "hasNextPage": next.is_some(), "endCursor": next },
    }}))
}

#[tokio::test]
async fn cmd_026_release_list_prints_jobs_and_names_the_next_page() {
    let hub = FakeHub::start().await;
    hub.answer("ReleaseList", releases_page(Some("r1")));
    let run = signed_in()
        .run(&["release", "list", "--limit", "5", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    let row = run.stdout.lines().nth(1).expect("a row");
    assert!(
        row.starts_with("0.4.0") && row.contains("0123456789ab"),
        "{row}"
    );
    assert!(
        row.contains("LINUX_X86_64 PUBLISHED, DARWIN_UNIVERSAL FAILED"),
        "{row}"
    );
    assert!(run.stderr.contains("--after r1"), "{}", run.stderr);
    assert_eq!(hub.seen()[0].variables["first"], 5);

    let run = signed_in()
        .run(&["release", "list", "--json", "--hub", &hub.origin])
        .await;
    assert_eq!(
        run.json()["edges"][0]["node"]["jobs"][1]["failureReason"],
        "xcode"
    );
}

#[tokio::test]
async fn cmd_026_release_list_all_follows_the_cursor() {
    let hub = FakeHub::start().await;
    hub.answer("ReleaseList", releases_page(Some("r1")))
        .answer("ReleaseList", releases_page(None));
    let run = signed_in()
        .run(&["release", "list", "--all", "--json", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert_eq!(run.json()["edges"].as_array().expect("edges").len(), 2);
    assert_eq!(hub.seen()[1].variables["after"], "r1");
}

#[tokio::test]
async fn release_next_version_prints_the_version_or_a_dash() {
    let hub = FakeHub::start().await;
    hub.answer(
        "ReleaseNextVersion",
        data(json!({ "nextReleaseVersion": "0.4.1" })),
    );
    let run = signed_in()
        .run(&["release", "next-version", "--hub", &hub.origin])
        .await;
    assert!(run.stdout.contains("0.4.1"), "{run:?}");
    let hub = FakeHub::start().await;
    hub.answer(
        "ReleaseNextVersion",
        data(json!({ "nextReleaseVersion": null })),
    );
    let run = signed_in()
        .run(&["release", "next-version", "--hub", &hub.origin])
        .await;
    assert!(run.stdout.trim_end().ends_with('-'), "{run:?}");
}

#[tokio::test]
async fn bld_001_release_create_sends_one_target_for_each_flag() {
    let hub = FakeHub::start().await;
    hub.answer(
        "ReleaseCreate",
        data(json!({ "createRelease": release("0.5.0") })),
    );
    let run = signed_in()
        .run(&[
            "release",
            "create",
            "--version",
            "0.5.0",
            "--commit",
            COMMIT,
            "--target",
            "linux-x86-64",
            "--target",
            "darwin-universal",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert!(run.stdout.contains("0.5.0"), "{}", run.stdout);
    let sent = &hub.seen()[0].variables;
    assert_eq!(sent["targets"], json!(["LINUX_X86_64", "DARWIN_UNIVERSAL"]));
    assert_eq!(
        (sent["version"].clone(), sent["commit"].clone()),
        (json!("0.5.0"), json!(COMMIT))
    );
}

#[tokio::test]
async fn release_create_refused_exits_5() {
    let hub = FakeHub::start().await;
    hub.answer(
        "ReleaseCreate",
        refusal("FORBIDDEN", "Login or browser token only."),
    );
    let run = signed_in()
        .run(&[
            "release",
            "create",
            "--version",
            "0.5.0",
            "--commit",
            COMMIT,
            "--target",
            "linux-x86-64",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 5, "{run:?}");
}

#[tokio::test]
async fn cmd_024_release_cancel_needs_yes() {
    let hub = FakeHub::start().await;
    let job = json!({
        "id": JOB, "releaseId": "0190a000-0000-7000-9003-000000000001", "target": "LINUX_X86_64",
        "version": "0.4.0", "state": "CANCELLED", "failureReason": null,
        "stateChangedAt": "2026-11-10T10:00:00+00:00",
    });
    hub.answer("ReleaseCancel", data(json!({ "cancelBuildJob": job })));
    let refused = signed_in()
        .run(&["release", "cancel", JOB, "--hub", &hub.origin])
        .await;
    assert_eq!(refused.code, 2, "{refused:?}");
    assert!(hub.seen().is_empty());
    let run = signed_in()
        .run(&["release", "cancel", JOB, "--yes", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert!(run.stdout.contains("CANCELLED"), "{}", run.stdout);
    assert_eq!(hub.seen()[0].variables["id"], JOB);
}

fn log_lines(from: i64, to: i64) -> Value {
    let lines: Vec<Value> = (from..=to)
        .map(|n| json!({ "sequence": n, "text": format!("line {n}\u{1b}[31m") }))
        .collect();
    data(json!({ "buildJobLog": lines }))
}

#[tokio::test]
async fn bld_048_release_log_prints_clean_lines_and_all_follows_the_sequence() {
    let hub = FakeHub::start().await;
    hub.answer("BuildJobLog", log_lines(1, 3));
    let run = signed_in()
        .run(&["release", "log", JOB, "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert_eq!(run.stdout.lines().next(), Some("line 1 [31m"));
    assert_eq!(run.stdout.lines().count(), 3);
    assert!(!run.stdout.contains('\u{1b}'));

    let hub = FakeHub::start().await;
    hub.answer("BuildJobLog", log_lines(1, 1000))
        .answer("BuildJobLog", log_lines(1001, 1003));
    let run = signed_in()
        .run(&["release", "log", JOB, "--all", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert_eq!(run.stdout.lines().count(), 1003);
    let seen = hub.seen();
    assert_eq!(
        (
            seen[0].variables["after"].clone(),
            seen[1].variables["after"].clone()
        ),
        (Value::Null, json!(1000))
    );
}

#[tokio::test]
async fn release_log_of_an_unknown_job_exits_3() {
    let hub = FakeHub::start().await;
    hub.answer("BuildJobLog", refusal("NOT_FOUND", "No such job."));
    let run = signed_in()
        .run(&["release", "log", JOB, "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 3, "{run:?}");
}
