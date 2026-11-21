//! HMD-068, HMD-069: a hub of the pool, in the link mode with the public origin that the pool
//! gives it, runs the whole flow for a browser whose requests the door passes with the name that
//! the browser used. Nothing in the flow of the hub changes; only the origin is given.

use std::{collections::BTreeMap, time::Duration};

use anima_hub::config::Config;
use reqwest::StatusCode;
use serde_json::{Value, json};
use sqlx::PgPool;

use super::{
    START,
    browser::{Browser, INSTALLATION, Served, install_state, served_as, shown},
    link,
};

const SLOW: Duration = Duration::from_secs(3600);
/// The address of each hosted hub before the address per hub (AC-D45 of anima-cloud).
const SHARED: &str = "https://hub.anima.ing";
/// The address of one hub since then (HMD-069): `h` and 20 letters or digits.
const PER_HUB: &str = "https://hk3f9x2mq7b4n8c1d5e6a.hub.anima.ing";
const SOURCES: &str = "query { sources { key connectPageUrl } }";

/// The configuration of a hosted hub in the link mode with `origin`, as the pool starts it
/// (`anima-local-hub run --public-origin`), its cloud key in a file of `folder`.
fn hosted(origin: &str, folder: &tempfile::TempDir) -> Config {
    let path = |name: &str| folder.path().join(name).display().to_string();
    std::fs::write(folder.path().join("cloud_key"), "test-cloud-key\n").unwrap();
    let env = BTreeMap::from([
        ("ANIMA_LINK_SOCKET".to_owned(), path("link.sock")),
        ("ANIMA_LINK_CONTROL".to_owned(), path("control.sock")),
        ("ANIMA_HOSTED_CLOUD_KEY_FILE".to_owned(), path("cloud_key")),
        ("ANIMA_PUBLIC_ORIGIN".to_owned(), origin.to_owned()),
    ]);
    anima_hub::config::from_env(&env).unwrap()
}

fn host(origin: &str) -> &str {
    origin.strip_prefix("https://").unwrap()
}

async fn connect_page(served: &Served) -> Value {
    let kit = &served.kit;
    let answer = kit.hub.post(Some(&kit.token), SOURCES, json!({})).await;
    let sources = answer["data"]["sources"].as_array().unwrap().clone();
    let github = sources.iter().find(|source| source["key"] == "github");
    github.unwrap()["connectPageUrl"].clone()
}

/// The one-time link of a new attempt, after the page of the hub and of the attempt are checked.
async fn one_time_link(served: &Served, page: &str) -> String {
    assert_eq!(connect_page(served).await, page);
    let kit = &served.kit;
    let made = kit
        .hub
        .post(Some(&kit.token), START, json!({ "input": null }))
        .await;
    assert_eq!(
        made["data"]["startGithubConnect"]["pageUrl"], page,
        "{made}"
    );
    let id = made["data"]["startGithubConnect"]["id"].as_str().unwrap();
    let one_time = link(kit, id).await;
    assert!(one_time.starts_with(&format!("{page}?ticket=")));
    one_time
}

/// The hub answers only the name of its own origin: a name it was not given is refused.
async fn refuses_other_names(served: &Served, origin: &str) {
    let other = if origin == SHARED {
        host(PER_HUB)
    } else {
        host(SHARED)
    };
    for refused in [other, "evil.hub.anima.ing"] {
        let answer = Browser::named(served, refused).get("/connect/github").await;
        assert_eq!(answer.status, StatusCode::FORBIDDEN, "{refused}");
    }
}

/// Start, the one-time link, "Create GitHub App" and "Install", each through the gateway.
async fn connects_through_the_gateway(pool: &PgPool, origin: &str) {
    let folder = tempfile::tempdir().unwrap();
    let served = served_as(pool, SLOW, hosted(origin, &folder)).await;
    let page = format!("{origin}/connect/github");
    let one_time = one_time_link(&served, &page).await;

    let mut browser = Browser::named(&served, host(origin));
    let s1 = browser.open(&one_time).await;
    let start = browser.get("/connect/github/start").await;
    for callback in ["created", "installed"] {
        let address = format!("{page}/{callback}");
        assert!(
            start.body.contains(&address),
            "the manifest names {address}"
        );
    }
    let s2 = install_state(&browser.created(&s1).await);
    let done = browser.installed(INSTALLATION, &s2).await;
    assert_eq!(done.status, StatusCode::OK);
    assert!(done.body.contains("GitHub is connected."), "{}", done.body);
    assert_eq!(shown(&served.kit).await["phase"], "CONNECTED");
    refuses_other_names(&served, origin).await;
}

#[sqlx::test(migrations = "./migrations")]
async fn hmd_068_a_hosted_hub_at_the_shared_gateway_address_connects_github(pool: PgPool) {
    connects_through_the_gateway(&pool, SHARED).await;
}

#[sqlx::test(migrations = "./migrations")]
async fn hmd_068_a_hosted_hub_at_its_own_gateway_address_connects_github(pool: PgPool) {
    connects_through_the_gateway(&pool, PER_HUB).await;
}
