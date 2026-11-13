//! `anima retention list` and `set` against the scripted hub (HUB-005, D-25).

use serde_json::{Value, json};

use crate::kit::Anima;
use crate::kit::fake_hub::{FakeHub, data, refusal};

const TOKEN: &str = "login-token-for-retention-tests";

fn periods() -> Vec<Value> {
    let text = include_str!("../../../../contracts/fixtures/phase3.json");
    let fixtures: Value = serde_json::from_str(text).expect("fixtures");
    fixtures["retention"].as_array().expect("retention").clone()
}

fn signed_in() -> Anima {
    Anima::new().with("ANIMA_TOKEN", TOKEN)
}

#[tokio::test]
async fn hub_005_retention_list_shows_each_subject_with_its_range() {
    let hub = FakeHub::start().await;
    hub.answer("RetentionList", data(json!({ "retention": periods() })));
    let run = signed_in()
        .run(&["retention", "list", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    let events = run
        .stdout
        .lines()
        .find(|line| line.starts_with("EVENTS"))
        .expect("a row");
    let columns: Vec<&str> = events.split_whitespace().collect();
    assert_eq!(columns, ["EVENTS", "90", "30", "365", "90"]);

    let run = signed_in()
        .run(&["retention", "list", "--json", "--hub", &hub.origin])
        .await;
    assert_eq!(run.json()[3]["subject"], "DELIVERIES");
}

#[tokio::test]
async fn hub_005_retention_set_sends_the_subject_and_days() {
    let hub = FakeHub::start().await;
    let mut period = periods()[2].clone();
    period["days"] = json!(120);
    hub.answer("RetentionSet", data(json!({ "setRetention": period })));
    let run = signed_in()
        .run(&["retention", "set", "events", "120", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    let days = run
        .stdout
        .lines()
        .find(|line| line.starts_with("Days"))
        .expect("days");
    assert!(days.ends_with("120"), "{days}");
    let seen = hub.seen();
    assert_eq!(
        seen[0].variables,
        json!({ "subject": "EVENTS", "days": 120 })
    );
}

#[tokio::test]
async fn int_003_retention_set_with_an_api_key_is_refused_with_exit_5() {
    let hub = FakeHub::start().await;
    hub.answer(
        "RetentionSet",
        refusal("FORBIDDEN", "An API key cannot change retention."),
    );
    let run = Anima::new()
        .with(
            "ANIMA_TOKEN",
            "anima_Ab12Cd34Ef56Gh78Ij90Kl12Mn34Op56Qr78St90Uv1",
        )
        .run(&[
            "retention",
            "set",
            "metrics",
            "10",
            "--json",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 5, "{run:?}");
    assert_eq!(run.error()["code"], "FORBIDDEN");
}

#[tokio::test]
async fn hub_005_days_outside_the_range_are_a_validation_error_with_exit_2() {
    let hub = FakeHub::start().await;
    hub.answer(
        "RetentionSet",
        refusal("VALIDATION", "Events are kept 30 to 365 days."),
    );
    let run = signed_in()
        .run(&["retention", "set", "events", "5", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 2, "{run:?}");
    assert!(run.stderr.contains("30 to 365"), "{}", run.stderr);
}
