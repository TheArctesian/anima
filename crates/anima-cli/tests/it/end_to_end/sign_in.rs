//! `anima auth` against the real hub: the password and the authenticator code at a terminal,
//! the stored token, and sign-out ending it on the hub (CMD-010, CMD-012, CMD-014, CMD-021).

use serde_json::{Value, json};
use sqlx::PgPool;

use super::hub::{PASSWORD, RealHub, USERNAME};
use super::terminal::Terminal;
use super::totp::{code_now, wrong_code};
use crate::kit::Anima;

/// `auth login` at a terminal, answering each question as it appears; its exit code and screen.
async fn log_in(anima: &Anima, code: &str) -> (i32, String) {
    let mut terminal = Terminal::start(anima, &["auth", "login"]);
    terminal.expect("User name: ").await;
    terminal.type_line(USERNAME).await;
    terminal.expect("Password: ").await;
    terminal.type_line(PASSWORD).await;
    terminal.expect("Authenticator code").await;
    terminal.type_line(code).await;
    terminal.finish().await
}

/// The token `anima` stored for `origin` in its 0600 token file.
fn stored_token(anima: &Anima, origin: &str) -> String {
    let file =
        std::fs::read_to_string(anima.config_dir().join("tokens.json")).expect("the token file");
    let tokens: Value = serde_json::from_str(&file).expect("the token file is JSON");
    let found = tokens
        .as_object()
        .and_then(|map| map.values().find_map(Value::as_str).map(str::to_owned));
    assert_eq!(
        tokens.as_object().map(serde_json::Map::len),
        Some(1),
        "{origin}"
    );
    found.expect("one stored token")
}

#[sqlx::test(migrations = "../hub/migrations")]
async fn cmd_010_login_at_a_terminal_with_totp_signs_in_and_logout_ends_the_token(pool: PgPool) {
    let hub = RealHub::start(pool).await;
    let seed = hub.enrol_totp().await;
    let anima = hub.anima();

    let (code, screen) = log_in(&anima, &wrong_code(&code_now(&seed))).await;
    assert_eq!(code, 4, "{screen}");
    assert!(!anima.config_dir().join("tokens.json").exists());

    let (code, screen) = log_in(&anima, &code_now(&seed)).await;
    assert_eq!(code, 0, "{screen}");
    assert!(
        !screen.contains(PASSWORD),
        "the password was echoed: {screen}"
    );
    let token = stored_token(&anima, &hub.origin);
    assert!(!screen.contains(&token), "{screen}");
    let me = hub.data(&token, "{ me { username } }", json!({})).await;
    assert_eq!(me["me"]["username"], USERNAME);

    let status = anima.run(&["auth", "status"]).await;
    assert_eq!(status.code, 0, "{status:?}");
    assert!(status.stdout.contains(USERNAME), "{status:?}");
    assert!(status.stdout.contains("login token"), "{status:?}");
    status.assert_never_shows(&token);
    let status = anima.run(&["auth", "status", "--json"]).await;
    assert_eq!(status.json()["tokenKind"], "LOGIN", "{status:?}");
    status.assert_never_shows(&token);

    let logout = anima.run(&["auth", "logout"]).await;
    assert_eq!(logout.code, 0, "{logout:?}");
    let refused = hub
        .graphql(Some(&token), "{ me { username } }", json!({}))
        .await;
    assert_eq!(
        refused["errors"][0]["extensions"]["code"], "UNAUTHENTICATED",
        "{refused}"
    );
    let later = Anima::new()
        .with("ANIMA_HUB", &hub.origin)
        .with("ANIMA_TOKEN", &token)
        .run(&["device", "list"])
        .await;
    assert_eq!(later.code, 4, "{later:?}");
    later.assert_never_shows(&token);
}
