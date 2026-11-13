//! `anima context`: a round trip through `config.json`, and which hub a command then talks to
//! (CMD-013, CMD-015).

use serde_json::json;

use crate::fleet::devices::page;
use crate::kit::Anima;
use crate::kit::fake_hub::FakeHub;

#[tokio::test]
async fn cmd_015_contexts_round_trip_through_the_config_file() {
    let anima = Anima::new();
    let run = anima
        .run(&["context", "add", "home", "https://home.example/"])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert!(
        run.stderr.contains("it is current"),
        "the first context becomes current"
    );
    let run = anima
        .run(&["context", "add", "lab", "http://127.0.0.1:8443", "--use"])
        .await;
    assert_eq!(run.code, 0, "{run:?}");

    let listed = anima.run(&["context", "list", "--json"]).await.json();
    assert_eq!(
        listed,
        json!([
            { "name": "home", "hub": "https://home.example", "current": false },
            { "name": "lab", "hub": "http://127.0.0.1:8443", "current": true },
        ])
    );
    let table = anima.run(&["context", "list"]).await.stdout;
    assert!(
        table
            .lines()
            .any(|line| line.starts_with('*') && line.contains("lab")),
        "{table}"
    );

    assert_eq!(anima.run(&["context", "use", "home"]).await.code, 0);
    assert_eq!(
        anima.run(&["context", "remove", "lab", "--yes"]).await.code,
        0
    );
    let text = std::fs::read_to_string(anima.config_dir().join("config.json")).expect("config");
    let config: serde_json::Value = serde_json::from_str(&text).expect("JSON");
    assert_eq!(
        config,
        json!({ "current": "home", "contexts": { "home": { "hub": "https://home.example" } } })
    );
}

#[tokio::test]
async fn cmd_015_bad_names_hubs_and_duplicates_are_refused() {
    let anima = Anima::new();
    let cases = [
        (vec!["context", "add", "web", "http://hub.example"], 2),
        (
            vec!["context", "add", "web", "https://hub.example/graphql"],
            2,
        ),
        (vec!["context", "add", "a b", "https://hub.example"], 2),
        (vec!["context", "use", "nowhere"], 3),
        (vec!["context", "remove", "nowhere", "--yes"], 3),
    ];
    for (args, code) in cases {
        let run = anima.run(&args).await;
        assert_eq!(run.code, code, "{args:?}: {run:?}");
    }
    assert_eq!(
        anima
            .run(&["context", "add", "web", "https://hub.example"])
            .await
            .code,
        0
    );
    let again = anima
        .run(&["context", "add", "web", "https://other.example", "--json"])
        .await;
    assert_eq!(
        (again.code, again.error()["code"].clone()),
        (6, json!("CONFLICT"))
    );
}

#[tokio::test]
async fn cmd_013_the_current_context_then_context_flag_then_hub_flag_choose_the_hub() {
    let current = FakeHub::start().await;
    let named = FakeHub::start().await;
    let flagged = FakeHub::start().await;
    for hub in [&current, &named, &flagged] {
        hub.answer("DeviceList", page(&[], None, 0));
    }
    let anima = Anima::new().with("ANIMA_TOKEN", "a-login-token");
    anima
        .run(&["context", "add", "current", &current.origin])
        .await;
    anima.run(&["context", "add", "named", &named.origin]).await;

    assert_eq!(anima.run(&["device", "list"]).await.code, 0);
    assert_eq!(
        anima
            .run(&["device", "list", "--context", "named"])
            .await
            .code,
        0
    );
    let both = [
        "device",
        "list",
        "--context",
        "named",
        "--hub",
        &flagged.origin,
    ];
    assert_eq!(anima.run(&both).await.code, 0);
    let counts: Vec<usize> = [&current, &named, &flagged]
        .iter()
        .map(|hub| hub.seen().len())
        .collect();
    assert_eq!(counts, [1, 1, 1]);

    let variable = anima.with("ANIMA_HUB", &flagged.origin);
    assert_eq!(variable.run(&["device", "list"]).await.code, 0);
    assert_eq!(
        flagged.seen().len(),
        2,
        "ANIMA_HUB is used before the current context"
    );
}
