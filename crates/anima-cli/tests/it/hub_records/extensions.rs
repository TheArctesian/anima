//! `anima extension list`, `on` and `off` against the scripted hub (EXT-011, EXT-012, CMD-024).

use serde_json::{Value, json};

use crate::kit::Anima;
use crate::kit::fake_hub::{FakeHub, data, refusal};

fn extension(key: &str, on: bool, needs: &[&str], needed_by: &[&str]) -> Value {
    json!({
        "key": key, "name": key, "dataClass": "B", "built": true, "fixed": false, "on": on,
        "needs": needs, "worksWith": [], "neededBy": needed_by,
        "navigation": { "group": "WORK", "label": key, "route": format!("/{key}") },
        "changedAt": null,
    })
}

fn signed_in() -> Anima {
    Anima::new().with("ANIMA_TOKEN", "login-token-for-extension-tests")
}

fn listed() -> Vec<Value> {
    vec![
        extension("projects", false, &["integrations"], &[]),
        extension("integrations", false, &[], &[]),
        extension("links", true, &["projects"], &[]),
    ]
}

#[tokio::test]
async fn ext_011_extension_list_prints_the_k1_value_with_json() {
    let hub = FakeHub::start().await;
    hub.answer("ExtensionList", data(json!({ "extensions": listed() })));
    let run = signed_in()
        .run(&["extension", "list", "--json", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert_eq!(run.json(), json!(listed()));

    let run = signed_in()
        .run(&["extension", "list", "--hub", &hub.origin])
        .await;
    let row = run
        .stdout
        .lines()
        .find(|line| line.starts_with("projects"))
        .expect("a row");
    assert!(row.contains("off") && row.contains("integrations"), "{row}");
}

#[tokio::test]
async fn ext_011_extension_on_names_what_it_also_turns_on_and_asks_unless_yes() {
    let hub = FakeHub::start().await;
    hub.answer("ExtensionList", data(json!({ "extensions": listed() })));
    let mut after = listed();
    after[0]["on"] = json!(true);
    hub.answer("ExtensionSet", data(json!({ "setExtension": after })));

    let run = signed_in()
        .run(&["extension", "on", "projects", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 2, "no terminal and no --yes: {run:?}");
    assert!(run.stderr.contains("--yes"), "{}", run.stderr);
    assert!(
        hub.seen()
            .iter()
            .all(|seen| seen.operation != "ExtensionSet"),
        "nothing was set"
    );

    let run = signed_in()
        .run(&[
            "extension",
            "on",
            "projects",
            "--yes",
            "--json",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert_eq!(run.json()[0]["on"], true);
    let set = hub.seen().pop().expect("a request");
    assert_eq!(set.operation, "ExtensionSet");
    assert_eq!(set.variables, json!({ "key": "projects", "on": true }));
}

#[tokio::test]
async fn ext_011_extension_on_for_an_unknown_key_is_not_found_and_sets_nothing() {
    let hub = FakeHub::start().await;
    hub.answer("ExtensionList", data(json!({ "extensions": listed() })));
    let run = signed_in()
        .run(&["extension", "on", "nope", "--yes", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 3, "{run:?}");
    assert_eq!(hub.seen().len(), 1);
}

#[tokio::test]
async fn ext_012_extension_off_shows_the_conflict_with_the_name_and_exits_6() {
    let hub = FakeHub::start().await;
    hub.answer(
        "ExtensionSet",
        refusal(
            "CONFLICT",
            "projects is needed by links; turn links off first",
        ),
    );
    let run = signed_in()
        .run(&["extension", "off", "projects", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 6, "{run:?}");
    assert!(run.stderr.contains("needed by links"), "{}", run.stderr);
    assert_eq!(
        hub.seen()[0].variables,
        json!({ "key": "projects", "on": false })
    );

    let run = signed_in()
        .run(&[
            "extension",
            "off",
            "projects",
            "--json",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 6);
    assert_eq!(run.error()["code"], "CONFLICT");
}
