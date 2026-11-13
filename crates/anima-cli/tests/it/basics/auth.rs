//! `anima auth`: a key from standard input, the token store, `ANIMA_TOKEN` before it, and
//! sign-out (CMD-011..014).

use std::os::unix::fs::PermissionsExt as _;

use anima_cli::hub::{Hub, Origin, Token};
use serde_json::json;
use zeroize::Zeroizing;

use crate::kit::Anima;
use crate::kit::fake_hub::{FakeHub, data, refusal};

const KEY: &str = "anima_TestKeyForTheAuthTests0123456789abcdefgh";
const LOGIN_TOKEN: &str = "login-token-for-the-auth-tests";

fn me() -> serde_json::Value {
    data(
        json!({ "me": { "id": "0190a000-0000-7000-8000-000000000001", "username": "operator", "totpEnabled": true } }),
    )
}

#[tokio::test]
async fn cmd_011_login_with_token_checks_the_key_and_stores_it_privately() {
    let hub = FakeHub::start().await;
    hub.answer("Me", me());
    let anima = Anima::new();
    let run = anima
        .run_with_input(
            &["auth", "login", "--with-token", "--hub", &hub.origin],
            &format!("{KEY}\n"),
        )
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert!(
        run.stderr.contains("as operator with an API key"),
        "{}",
        run.stderr
    );
    assert_eq!(hub.seen()[0].authorization, Some(format!("Bearer {KEY}")));
    let file = anima.config_dir().join("tokens.json");
    let mode = std::fs::metadata(&file)
        .expect("the token file")
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o600);

    let run = anima.run(&["auth", "status", "--hub", &hub.origin]).await;
    assert_eq!(run.code, 0, "{run:?}");
    assert!(
        run.stdout.contains("an API key, from the key store"),
        "{}",
        run.stdout
    );
    assert!(run.stdout.contains(&hub.origin) && run.stdout.contains("operator"));
    let run = anima
        .run(&["auth", "status", "--json", "--hub", &hub.origin])
        .await;
    let status = run.json();
    assert_eq!(
        (
            status["tokenKind"].clone(),
            status["tokenSource"].clone(),
            status["operator"]["username"].clone()
        ),
        (json!("API_KEY"), json!("STORE"), json!("operator"))
    );
    assert_eq!(status["hub"], hub.origin);
}

#[tokio::test]
async fn cmd_011_a_refused_key_is_not_stored() {
    let hub = FakeHub::start().await;
    hub.answer("Me", refusal("UNAUTHENTICATED", "Sign in to continue."));
    let anima = Anima::new();
    let run = anima
        .run_with_input(
            &["auth", "login", "--with-token", "--hub", &hub.origin],
            KEY,
        )
        .await;
    assert_eq!(run.code, 4, "{run:?}");
    assert!(!anima.config_dir().join("tokens.json").exists());
    let run = anima.run(&["auth", "status", "--hub", &hub.origin]).await;
    assert_eq!(run.code, 4, "still signed out: {run:?}");
    let empty = anima
        .run_with_input(
            &["auth", "login", "--with-token", "--hub", &hub.origin],
            "  \n",
        )
        .await;
    assert_eq!(empty.code, 1, "{empty:?}");
}

#[tokio::test]
async fn cmd_013_anima_token_is_used_before_the_stored_token() {
    let hub = FakeHub::start().await;
    hub.answer("Me", me());
    let anima = Anima::new();
    let stored = anima
        .run_with_input(
            &["auth", "login", "--with-token", "--hub", &hub.origin],
            KEY,
        )
        .await;
    assert_eq!(stored.code, 0, "{stored:?}");
    let with_variable = anima.with("ANIMA_TOKEN", LOGIN_TOKEN);
    let run = with_variable
        .run(&["auth", "status", "--hub", &hub.origin])
        .await;
    assert!(
        run.stdout.contains("a login token, from ANIMA_TOKEN"),
        "{}",
        run.stdout
    );
    let last = hub.seen().pop().expect("a request");
    assert_eq!(last.authorization, Some(format!("Bearer {LOGIN_TOKEN}")));
}

#[tokio::test]
async fn cmd_012_logout_ends_a_login_token_at_the_hub_and_deletes_it() {
    let hub = FakeHub::start().await;
    hub.answer("Me", me())
        .answer("Logout", data(json!({ "logout": true })));
    let anima = Anima::new();
    anima
        .run_with_input(
            &["auth", "login", "--with-token", "--hub", &hub.origin],
            LOGIN_TOKEN,
        )
        .await;
    let run = anima.run(&["auth", "logout", "--hub", &hub.origin]).await;
    assert_eq!(run.code, 0, "{run:?}");
    let logout = hub.seen().pop().expect("a request");
    assert_eq!(
        (logout.operation.as_str(), logout.authorization),
        ("Logout", Some(format!("Bearer {LOGIN_TOKEN}")))
    );
    assert!(
        !anima.config_dir().join("tokens.json").exists(),
        "the last token went, and its file"
    );
    let again = anima.run(&["auth", "status", "--hub", &hub.origin]).await;
    assert_eq!(again.code, 4);
}

#[tokio::test]
async fn cmd_012_logout_of_an_api_key_deletes_it_here_only() {
    let hub = FakeHub::start().await;
    hub.answer("Me", me());
    let anima = Anima::new();
    anima
        .run_with_input(
            &["auth", "login", "--with-token", "--hub", &hub.origin],
            KEY,
        )
        .await;
    let run = anima.run(&["auth", "logout", "--hub", &hub.origin]).await;
    assert_eq!(run.code, 0, "{run:?}");
    assert!(hub.seen().iter().all(|seen| seen.operation != "Logout"));
    assert!(run.stderr.contains("still works"), "{}", run.stderr);
}

#[tokio::test]
async fn cmd_012_a_token_file_others_can_read_is_refused() {
    let hub = FakeHub::start().await;
    hub.answer("Me", me());
    let anima = Anima::new();
    anima
        .run_with_input(
            &["auth", "login", "--with-token", "--hub", &hub.origin],
            KEY,
        )
        .await;
    let file = anima.config_dir().join("tokens.json");
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o644)).expect("chmod");
    let run = anima.run(&["auth", "status", "--hub", &hub.origin]).await;
    assert_eq!(run.code, 1, "{run:?}");
    assert!(run.stderr.contains("0600"), "{}", run.stderr);
    run.assert_never_shows(KEY);
}

#[test]
fn cmd_014_debug_formatting_of_a_hub_client_hides_the_token() {
    let origin = Origin::parse("https://hub.example").expect("origin");
    let token = Token::new(Zeroizing::new(KEY.to_owned())).expect("a token");
    let hub = Hub::new(origin, Some(token)).expect("a client");
    let shown = format!("{hub:?} {hub:#?}");
    assert!(
        !shown.contains(KEY) && !shown.contains("TestKey"),
        "{shown}"
    );
    assert!(shown.contains("hub.example"));
}
