//! The three presses through the real hub (PRJ-039, PRJ-041, CMD-030): `anima source connect
//! github --no-browser` at a terminal prints the one-time link, the browser stand-in follows it
//! through Create (the stub plays GitHub's conversion), the hub's poll finds the installation,
//! the command ends with exit 0, and `anima source list` and `anima project list` show the
//! account and the stub's repositories. The hub reads a loopback GitHub, never GitHub.

use std::time::Duration;

use rsa::{
    RsaPrivateKey,
    pkcs1::{EncodeRsaPrivateKey as _, LineEnding},
    rand_core::OsRng,
};
use serde_json::{Value, json};
use sqlx::PgPool;

use super::words;
use crate::kit::browser::Browser;
use crate::kit::github::{self, Reply, Stub};
use crate::kit::real_hub::RealHub;
use crate::kit::terminal::Terminal;

const CODE: &str = "CODEMARKER7c1f";
const INSTALLATIONS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../providers/github/tests/fixtures/installations.json"
));
const LINK_START: &str = "https://hub.anima.ing/connect/github?ticket=";

/// GitHub's reply to the conversion of `CODE`: the App, its owner and a throwaway key.
fn conversion(stub: &Stub) {
    let key = RsaPrivateKey::new(&mut OsRng, 2048).expect("a key");
    let pem = key.to_pkcs1_pem(LineEnding::LF).expect("PEM");
    let body = json!({
        "id": 1_234_567, "slug": "anima-k3f9x2mq7b",
        "owner": { "login": "octo-person", "id": 9001, "type": "User" },
        "client_secret": "CLIENT-SECRET-MARKER-0000", "webhook_secret": "WEBHOOK-SECRET-MARKER-1111",
        "pem": pem.as_str(),
    });
    let reply = |body: String| Reply {
        status: 201,
        headers: Vec::new(),
        body,
    };
    stub.reply(
        &format!("/app-manifests/{CODE}/conversions"),
        reply(body.to_string()),
    );
    stub.reply(
        "/app/installations",
        Reply {
            status: 200,
            headers: Vec::new(),
            body: INSTALLATIONS.to_owned(),
        },
    );
}

/// The one-time link as the terminal printed it.
fn link_on(screen: &str) -> String {
    let at = screen.find(LINK_START).expect("the link on the screen");
    screen[at..]
        .split(|c: char| c.is_whitespace())
        .next()
        .expect("a link")
        .to_owned()
}

async fn slugs(anima: &crate::kit::Anima) -> Vec<String> {
    let list = anima.run(&words("project list --json")).await;
    assert_eq!(list.code, 0, "{list:?}");
    let list = list.json();
    let edges = list["edges"].as_array().expect("edges");
    edges
        .iter()
        .map(|edge| edge["node"]["slug"].as_str().expect("a slug").to_owned())
        .collect()
}

#[sqlx::test(migrations = "../hub/migrations")]
async fn prj_039_the_command_connects_github_in_three_presses_and_the_projects_appear(
    pool: PgPool,
) {
    let (hub, stub) = RealHub::with_github_polling(pool, Duration::from_millis(100)).await;
    conversion(&stub);
    github::repositories(&stub, &["me/site", "me/blog"]);
    let token = hub.login_token().await;
    let anima = hub.anima().with("ANIMA_TOKEN", &token);

    let mut terminal = Terminal::start(&anima, &words("source connect github --no-browser"));
    terminal
        .expect("Open this link in your browser. It works once, for 5 minutes:")
        .await;
    terminal.expect(LINK_START).await;
    terminal.expect("\n").await;
    let link = link_on(&terminal.screen());
    let mut browser = Browser::at(&hub.origin);
    let state = browser.open(&link).await;
    terminal
        .expect("Waiting for you to press \"Create GitHub App\" on GitHub.")
        .await;

    let made = browser.created(CODE, &state).await;
    assert_eq!(made.status, reqwest::StatusCode::SEE_OTHER, "{}", made.body);
    terminal
        .expect("GitHub App anima-k3f9x2mq7b is made for octo-person.")
        .await;
    terminal
        .expect("GitHub is connected as \"octo-person\".")
        .await;
    let (code, screen) = terminal.finish().await;
    assert_eq!(code, 0, "{screen}");
    assert_eq!(
        screen.matches(&link).count(),
        1,
        "the link is on screen once"
    );
    assert!(
        !screen.contains("MARKER"),
        "no secret of GitHub's reply: {screen}"
    );

    let listed = anima.run(&words("source list --json")).await;
    assert_eq!(listed.code, 0, "{listed:?}");
    let accounts = listed.json();
    assert_eq!(accounts[0]["label"], "octo-person");
    assert_eq!(accounts[0]["credentialStored"], true);

    let on = anima.run(&words("extension on projects")).await;
    assert_eq!(on.code, 0, "{on:?}");
    let sync = anima.run(&words("source sync octo-person")).await;
    assert_eq!(sync.code, 0, "{sync:?}");
    for _ in 0..200 {
        if slugs(&anima).await.len() == 2 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert_eq!(slugs(&anima).await, ["blog", "site"]);

    let state: Value = anima
        .run(&words("source connect github --state --json"))
        .await
        .json();
    assert_eq!(state["phase"], "CONNECTED");
    assert!(
        !state.to_string().contains("ticket"),
        "the attempt holds no link: {state}"
    );
}
