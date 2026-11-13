//! Exit codes (CMD-021), JSON errors (CMD-022), `--no-input` (CMD-023) and the rule that no
//! output ever holds the token (CMD-014).

use serde_json::json;

use crate::fleet::devices::{fixture_devices, page};
use crate::kit::Anima;
use crate::kit::fake_hub::{FakeHub, data, refusal};

const TOKEN: &str = "anima_SecretKeyThatMustNeverPrint0123456789abcdef";

#[tokio::test]
async fn cmd_021_each_k7_code_has_its_exit_code() {
    let cases = [
        ("UNAUTHENTICATED", 4),
        ("FORBIDDEN", 5),
        ("NOT_FOUND", 3),
        ("VALIDATION", 2),
        ("CONFLICT", 6),
        ("UNAVAILABLE", 7),
        ("INTERNAL", 1),
    ];
    for (code, exit) in cases {
        let hub = FakeHub::start().await;
        hub.answer("DeviceList", refusal(code, "The hub said no."));
        let anima = Anima::new().with("ANIMA_TOKEN", TOKEN);
        let run = anima.run(&["device", "list", "--hub", &hub.origin]).await;
        assert_eq!(run.code, exit, "{code}: {run:?}");
        assert!(
            run.stderr.starts_with("error: The hub said no."),
            "{code}: {}",
            run.stderr
        );
        assert_eq!(run.stdout, "", "{code}: no data on stdout");
        let run = anima
            .run(&["device", "list", "--json", "--hub", &hub.origin])
            .await;
        assert_eq!((run.code, run.error()["code"].clone()), (exit, json!(code)));
    }
}

#[tokio::test]
async fn cmd_022_unauthenticated_is_exit_4_with_the_sign_in_hint() {
    let hub = FakeHub::start().await;
    hub.answer(
        "DeviceList",
        refusal("UNAUTHENTICATED", "Sign in to continue."),
    );
    let anima = Anima::new().with("ANIMA_TOKEN", TOKEN);
    let run = anima.run(&["device", "list", "--hub", &hub.origin]).await;
    assert_eq!(run.code, 4, "{run:?}");
    assert!(
        run.stderr.contains("hint: run `anima auth login`"),
        "{}",
        run.stderr
    );
    let run = anima
        .run(&["device", "list", "--json", "--hub", &hub.origin])
        .await;
    assert_eq!(
        run.error(),
        json!({ "code": "UNAUTHENTICATED", "message": "Sign in to continue.", "hint": "anima auth login" })
    );
}

#[tokio::test]
async fn cmd_022_a_validation_error_names_its_fields() {
    let hub = FakeHub::start().await;
    let body = json!({ "errors": [{ "message": "Check the input.", "extensions": {
        "code": "VALIDATION", "fields": [{ "field": "first", "message": "Ask for 1 to 200." }] } }] });
    hub.answer("DeviceList", body);
    let anima = Anima::new().with("ANIMA_TOKEN", TOKEN);
    let run = anima.run(&["device", "list", "--hub", &hub.origin]).await;
    assert!(
        run.stderr.contains("first: Ask for 1 to 200."),
        "{}",
        run.stderr
    );
    let run = anima
        .run(&["device", "list", "--json", "--hub", &hub.origin])
        .await;
    assert_eq!(run.error()["fields"][0]["field"], "first");
}

#[tokio::test]
async fn cmd_021_no_token_is_exit_4_before_any_request() {
    let hub = FakeHub::start().await;
    let run = Anima::new()
        .run(&["device", "list", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 4, "{run:?}");
    assert!(run.stderr.contains("not signed in"), "{}", run.stderr);
    assert!(run.stderr.contains("anima auth login"));
    assert!(hub.seen().is_empty());
}

#[tokio::test]
async fn cmd_021_a_hub_that_does_not_answer_is_exit_7() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("a port");
    let origin = format!("http://{}", listener.local_addr().expect("an address"));
    drop(listener);
    let run = Anima::new()
        .with("ANIMA_TOKEN", TOKEN)
        .run(&["device", "list", "--json", "--hub", &origin])
        .await;
    assert_eq!(
        (run.code, run.error()["code"].clone()),
        (7, json!("UNAVAILABLE")),
        "{run:?}"
    );
}

#[tokio::test]
async fn cmd_023_no_input_fails_with_exit_2_and_names_the_flag() {
    let anima = Anima::new();
    let run = anima
        .run(&[
            "auth",
            "login",
            "--no-input",
            "--hub",
            "https://hub.example",
        ])
        .await;
    assert_eq!(run.code, 2, "{run:?}");
    assert!(run.stderr.contains("--with-token"), "{}", run.stderr);

    let added = anima
        .run(&["context", "add", "home", "https://home.example"])
        .await;
    assert_eq!(added.code, 0, "{added:?}");
    let run = anima
        .run(&["context", "remove", "home", "--no-input", "--json"])
        .await;
    assert_eq!(
        (run.code, run.error()["code"].clone()),
        (2, json!("USAGE")),
        "{run:?}"
    );
    assert!(
        run.error()["message"]
            .as_str()
            .is_some_and(|text| text.contains("--yes"))
    );
    let listed = anima.run(&["context", "list", "--json"]).await;
    assert_eq!(listed.json()[0]["name"], "home", "nothing was removed");
}

#[tokio::test]
async fn cmd_021_wrong_usage_is_exit_2_and_json_when_asked() {
    let run = Anima::new().run(&["device", "lst"]).await;
    assert_eq!(run.code, 2, "{run:?}");
    let run = Anima::new()
        .run(&["device", "list", "--limit", "0", "--json"])
        .await;
    assert_eq!(
        (run.code, run.error()["code"].clone()),
        (2, json!("USAGE")),
        "{run:?}"
    );
}

/// Every path that prints, success and failure, table and JSON: the token is in none.
#[tokio::test]
async fn cmd_014_the_token_never_appears_in_any_output() {
    let hub = FakeHub::start().await;
    let devices = fixture_devices();
    hub.answer("DeviceList", page(&devices, None, devices.len()))
        .answer(
            "Me",
            data(json!({ "me": { "id": "op-1", "username": "operator", "totpEnabled": true } })),
        );
    let refusing = FakeHub::start().await;
    for code in ["UNAUTHENTICATED", "FORBIDDEN", "INTERNAL"] {
        refusing.answer("DeviceList", refusal(code, "No."));
    }
    let anima = Anima::new().with("ANIMA_TOKEN", TOKEN);
    for origin in [&hub.origin, &refusing.origin] {
        for args in [
            vec!["device", "list"],
            vec!["device", "list", "--json"],
            vec!["auth", "status"],
            vec!["auth", "status", "--json"],
        ] {
            let mut args = args;
            args.extend(["--hub", origin]);
            for _ in 0..3 {
                anima.run(&args).await.assert_never_shows(TOKEN);
            }
        }
    }
    let stored = Anima::new();
    let login = stored
        .run_with_input(
            &["auth", "login", "--with-token", "--hub", &hub.origin],
            TOKEN,
        )
        .await;
    assert_eq!(login.code, 0, "{login:?}");
    login.assert_never_shows(TOKEN);
    let config =
        std::fs::read_to_string(stored.config_dir().join("config.json")).unwrap_or_default();
    assert!(!config.contains(TOKEN), "the config file holds no secret");
    for args in [["auth", "status"], ["auth", "logout"]] {
        let run = stored.run(&[args[0], args[1], "--hub", &hub.origin]).await;
        run.assert_never_shows(TOKEN);
    }
}
