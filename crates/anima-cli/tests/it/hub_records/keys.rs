//! `anima key list`, `create` and `revoke` against the scripted hub (INT-003, CMD-024).

use serde_json::{Value, json};

use crate::kit::Anima;
use crate::kit::fake_hub::{FakeHub, data, refusal};

const TOKEN: &str = "login-token-for-key-tests";
const SECRET: &str = "anima_NewSecretNewSecretNewSecretNewSecretNew12";

fn keys() -> Vec<Value> {
    let text = include_str!("../../../../../contracts/fixtures/phase3.json");
    let fixtures: Value = serde_json::from_str(text).expect("fixtures");
    fixtures["apiKeys"].as_array().expect("apiKeys").clone()
}

fn signed_in() -> Anima {
    Anima::new().with("ANIMA_TOKEN", TOKEN)
}

#[tokio::test]
async fn int_003_key_list_shows_prefix_and_expiry_and_never_a_secret() {
    let hub = FakeHub::start().await;
    hub.answer("KeyList", data(json!({ "apiKeys": keys() })));
    let run = signed_in()
        .run(&["key", "list", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    let row = run
        .stdout
        .lines()
        .find(|line| line.starts_with("metrics-scrape"))
        .expect("a row");
    assert!(
        row.contains("READ")
            && row.contains("USER")
            && row.contains("anima_Ab12Cd")
            && row.contains("2027-06-01"),
        "{row}"
    );
    let unused = run
        .stdout
        .lines()
        .find(|line| line.starts_with("old-script"))
        .expect("a row");
    assert!(
        unused.contains("never") && unused.contains("AGENT"),
        "{unused}"
    );

    let run = signed_in()
        .run(&["key", "list", "--json", "--hub", &hub.origin])
        .await;
    assert_eq!(run.json()[1]["scope"], "WRITE");
}

fn created(key: &Value) -> Value {
    data(json!({ "createApiKey": { "key": key, "secret": SECRET } }))
}

#[tokio::test]
async fn int_003_key_create_prints_the_secret_once_with_a_note_on_standard_error() {
    let hub = FakeHub::start().await;
    let mut key = keys()[0].clone();
    key["name"] = json!("ci");
    hub.answer("KeyCreate", created(&key));
    let run = signed_in()
        .run(&[
            "key",
            "create",
            "--name",
            "ci",
            "--for",
            "user",
            "--scope",
            "write",
            "--expires",
            "90d",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert_eq!(
        run.stdout,
        format!("{SECRET}\n"),
        "the secret alone on stdout"
    );
    assert!(run.stderr.contains("shown one time"), "{}", run.stderr);
    assert!(!run.stderr.contains(SECRET));
    run.assert_never_shows(TOKEN);
    let input = &hub.seen()[0].variables["input"];
    assert_eq!(
        (input["name"].clone(), input["scope"].clone()),
        (json!("ci"), json!("WRITE"))
    );
    let expires = input["expiresAt"].as_str().expect("a time");
    assert!(expires.ends_with('Z') && expires.len() == 20, "{expires}");

    let json_run = [
        "key",
        "create",
        "--name",
        "ci",
        "--for",
        "user",
        "--expires",
        "30d",
    ];
    let run = signed_in()
        .run(&[&json_run[..], &["--json", "--hub", &hub.origin]].concat())
        .await;
    let json = run.json();
    assert_eq!(
        (json["secret"].clone(), json["key"]["name"].clone()),
        (json!(SECRET), json!("ci"))
    );
    assert_eq!(
        hub.seen()[1].variables["input"]["scope"],
        "READ",
        "a key reads unless told otherwise"
    );
}

#[tokio::test]
async fn int_003_key_create_needs_an_expiry_of_at_most_a_year() {
    let hub = FakeHub::start().await;
    let run = signed_in()
        .run(&[
            "key",
            "create",
            "--name",
            "ci",
            "--for",
            "user",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 2, "{run:?}");
    assert!(run.stderr.contains("--expires"), "{}", run.stderr);
    let run = signed_in()
        .run(&[
            "key",
            "create",
            "--name",
            "ci",
            "--for",
            "user",
            "--expires",
            "400d",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 2, "{run:?}");
    assert!(run.stderr.contains("at most one year"), "{}", run.stderr);
    assert!(hub.seen().is_empty(), "nothing was sent");
}

#[tokio::test]
async fn log_015_key_create_needs_for_and_sends_it() {
    let hub = FakeHub::start().await;
    let missing = signed_in()
        .run(&[
            "key",
            "create",
            "--name",
            "ci",
            "--expires",
            "30d",
            "--no-input",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(missing.code, 2, "{missing:?}");
    assert!(missing.stderr.contains("--for"), "{}", missing.stderr);
    assert!(hub.seen().is_empty(), "nothing was sent");

    let mut key = keys()[1].clone();
    key["name"] = json!("claude");
    hub.answer("KeyCreate", created(&key));
    let run = signed_in()
        .run(&[
            "key",
            "create",
            "--name",
            "claude",
            "--for",
            "agent",
            "--expires",
            "30d",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert_eq!(hub.seen()[0].variables["input"]["madeFor"], "AGENT");
    assert!(run.stderr.contains("for AGENT"), "{}", run.stderr);
    assert!(
        hub.seen()[0].actor.is_none(),
        "the command line declares nothing"
    );
}

#[tokio::test]
async fn int_003_key_create_with_an_api_key_is_forbidden_with_exit_5() {
    let hub = FakeHub::start().await;
    hub.answer(
        "KeyCreate",
        refusal("FORBIDDEN", "An API key cannot make API keys."),
    );
    let run = signed_in()
        .run(&[
            "key",
            "create",
            "--name",
            "ci",
            "--for",
            "user",
            "--expires",
            "30d",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 5, "{run:?}");
}

#[tokio::test]
async fn cmd_024_key_revoke_confirms_and_finds_a_key_by_its_name() {
    let hub = FakeHub::start().await;
    let id = keys()[1]["id"].as_str().expect("id").to_owned();
    hub.answer("KeyList", data(json!({ "apiKeys": keys() })))
        .answer("KeyRevoke", data(json!({ "revokeApiKey": id })));
    let refused = signed_in()
        .run(&[
            "key",
            "revoke",
            "old-script",
            "--no-input",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(refused.code, 2, "{refused:?}");
    assert!(refused.stderr.contains("--yes"), "{}", refused.stderr);
    assert!(hub.seen().iter().all(|seen| seen.operation != "KeyRevoke"));

    let run = signed_in()
        .run(&["key", "revoke", "old-script", "--yes", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert!(run.stdout.contains(&id), "{}", run.stdout);
    let revoke = hub
        .seen()
        .into_iter()
        .find(|seen| seen.operation == "KeyRevoke")
        .expect("a revoke");
    assert_eq!(revoke.variables["id"], json!(id));

    let missing = signed_in()
        .run(&[
            "key",
            "revoke",
            "no-such-key",
            "--yes",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(missing.code, 3, "{missing:?}");
}

#[tokio::test]
async fn cmd_020_c7_a_revoke_prints_the_bare_id_with_json() {
    let hub = FakeHub::start().await;
    let id = "0190a000-0000-7000-8020-000000000001";
    hub.answer("KeyRevoke", data(json!({ "revokeApiKey": id })));
    let run = signed_in()
        .run(&["key", "revoke", id, "--yes", "--json", "--hub", &hub.origin])
        .await;
    assert_eq!(run.json(), json!(id));
}
