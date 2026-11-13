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
        row.contains("READ") && row.contains("anima_Ab12Cd") && row.contains("2027-06-01"),
        "{row}"
    );
    let unused = run
        .stdout
        .lines()
        .find(|line| line.starts_with("old-script"))
        .expect("a row");
    assert!(unused.contains("never"), "{unused}");

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
            "--scope",
            "write",
            "--expires",
            "90d",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    let secret = run
        .stdout
        .lines()
        .find(|line| line.starts_with("Secret"))
        .expect("a secret line");
    assert!(secret.ends_with(SECRET), "{secret}");
    assert_eq!(run.stdout.matches(SECRET).count(), 1);
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

    let run = signed_in()
        .run(&[
            "key",
            "create",
            "--name",
            "ci",
            "--expires",
            "30d",
            "--json",
            "--hub",
            &hub.origin,
        ])
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
        .run(&["key", "create", "--name", "ci", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 2, "{run:?}");
    assert!(run.stderr.contains("--expires"), "{}", run.stderr);
    let run = signed_in()
        .run(&[
            "key",
            "create",
            "--name",
            "ci",
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
