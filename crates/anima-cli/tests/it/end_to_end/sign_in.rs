//! `anima auth` against the real hub: the password and the authenticator code at a terminal,
//! the stored token, and sign-out ending it on the hub (CMD-010, CMD-012, CMD-014, CMD-021).

use serde_json::{Value, json};
use sqlx::PgPool;

use crate::kit::Anima;
use crate::kit::real_hub::{PASSWORD, RealHub, USERNAME};
use crate::kit::terminal::{Terminal, assert_modes_restored};
use crate::kit::totp::{code_now, wrong_code};

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

/// `auth login` at a terminal with the modes shown, stopped at the password question.
async fn at_the_password(anima: &Anima) -> Terminal {
    let mut terminal = Terminal::start_showing_modes(anima, &["auth", "login"]);
    terminal.expect("User name: ").await;
    terminal.type_line(USERNAME).await;
    terminal.expect("Password: ").await;
    terminal
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

/// A person who types ahead: the user name and the password at once, before the password
/// question is on screen. The terminal must never show the password (CMD-010, LOG-007).
#[sqlx::test(migrations = "../hub/migrations")]
async fn cmd_010_a_password_typed_ahead_is_never_echoed(pool: PgPool) {
    let hub = RealHub::start(pool).await;
    let anima = hub.anima();
    let mut terminal = Terminal::start(&anima, &["auth", "login"]);
    terminal.expect("User name: ").await;
    terminal.type_line(&format!("{USERNAME}\r{PASSWORD}")).await;
    terminal.expect("Authenticator code").await;
    terminal.type_line("").await;
    let (code, screen) = terminal.finish().await;
    assert!(
        !screen.contains(PASSWORD),
        "the password was echoed: {screen}"
    );
    assert_eq!(code, 0, "{screen}");
}

/// Backspace and Ctrl-U edit the password; the terminal echoes again afterwards, in the modes
/// it had before (CMD-010).
#[sqlx::test(migrations = "../hub/migrations")]
async fn cmd_010_an_edited_password_signs_in_and_the_terminal_echoes_again(pool: PgPool) {
    let hub = RealHub::start(pool).await;
    let anima = hub.anima();
    let mut terminal = at_the_password(&anima).await;
    terminal
        .type_line(&format!("mistake\x15{PASSWORD}x\x7f"))
        .await;
    terminal.expect("Authenticator code").await;
    terminal.type_line("").await;
    let (code, screen) = terminal.finish().await;
    assert_eq!(code, 0, "{screen}");
    assert!(
        screen.contains(USERNAME),
        "the user name is shown: {screen}"
    );
    assert!(
        !screen.contains(PASSWORD) && !screen.contains("mistake"),
        "{screen}"
    );
    assert_modes_restored(&screen);
}

/// Ctrl-C at the password cancels with exit code 1 (DECLINED) and restores the terminal.
#[sqlx::test(migrations = "../hub/migrations")]
async fn cmd_010_ctrl_c_at_the_password_cancels_and_restores_the_terminal(pool: PgPool) {
    let hub = RealHub::start(pool).await;
    let anima = hub.anima();
    let mut terminal = at_the_password(&anima).await;
    terminal.press("zqx\x03").await;
    let (code, screen) = terminal.finish().await;
    assert_eq!(code, 1, "{screen}");
    assert!(screen.contains("the question was cancelled"), "{screen}");
    assert!(!screen.contains("zqx"), "{screen}");
    assert!(!anima.config_dir().join("tokens.json").exists());
    assert_modes_restored(&screen);
}

/// `SIGINT` from another program at the password ends `anima` as the signal would (128 + 2)
/// and restores the terminal first.
#[sqlx::test(migrations = "../hub/migrations")]
async fn cmd_010_sigint_at_the_password_restores_the_terminal(pool: PgPool) {
    let hub = RealHub::start(pool).await;
    let anima = hub.anima();
    let terminal = at_the_password(&anima).await;
    terminal.signal("INT").await;
    let (code, screen) = terminal.finish().await;
    assert_eq!(code, 130, "{screen}");
    assert_modes_restored(&screen);
}

/// Ctrl-D on an empty password is the end of input: a failure, and the terminal restored.
#[sqlx::test(migrations = "../hub/migrations")]
async fn cmd_010_end_of_input_at_the_password_fails_and_restores_the_terminal(pool: PgPool) {
    let hub = RealHub::start(pool).await;
    let anima = hub.anima();
    let mut terminal = at_the_password(&anima).await;
    terminal.press("\x04").await;
    let (code, screen) = terminal.finish().await;
    assert_eq!(code, 1, "{screen}");
    assert!(screen.contains("the input ended"), "{screen}");
    assert_modes_restored(&screen);
}

/// A password of multi-byte characters, pasted in one piece with the Return, signs in.
#[sqlx::test(migrations = "../hub/migrations")]
async fn cmd_010_a_pasted_multi_byte_password_signs_in(pool: PgPool) {
    let hub = RealHub::start(pool).await;
    let password = "pässwörd-鍵-🔑-staple";
    hub.set_password(password).await;
    let anima = hub.anima();
    let mut terminal = Terminal::start(&anima, &["auth", "login", "--username", USERNAME]);
    terminal.expect("Password: ").await;
    terminal.type_line(password).await;
    terminal.expect("Authenticator code").await;
    terminal.type_line("").await;
    let (code, screen) = terminal.finish().await;
    assert_eq!(code, 0, "{screen}");
    assert!(!screen.contains(password), "{screen}");
    let token = stored_token(&anima, &hub.origin);
    let me = hub.data(&token, "{ me { username } }", json!({})).await;
    assert_eq!(me["me"]["username"], USERNAME);
}
