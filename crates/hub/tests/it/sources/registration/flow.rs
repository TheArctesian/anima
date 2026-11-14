//! PRJ-039, PRJ-018, SRC-008: the whole flow through the hub's pages and GitHub's redirects
//! ends with one account, one sealed key and a first run; the poll finds an installation when
//! the browser never comes back; a replayed `/created` converts once; a forged installation is
//! ignored.

use std::time::Duration;

use reqwest::StatusCode;
use serde_json::Value;
use sqlx::PgPool;

use super::{
    browser::{
        Browser, INSTALLATION, LIST, Served, conversions, install_state, reply, served, shown,
    },
    link, started,
};
use crate::sources::set_projects;

const SLOW: Duration = Duration::from_secs(3600);

/// `(accounts, sealed keys, runs started by the connect)`.
async fn stored(pool: &PgPool) -> (i64, i64, i64) {
    sqlx::query_as(
        "SELECT (SELECT count(*) FROM sources.accounts), \
         (SELECT count(*) FROM credentials.source_secrets), \
         (SELECT count(*) FROM sources.sync_runs WHERE trigger = 'connect')",
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

/// Waits until `githubConnect` shows `phase`.
async fn until_phase(served: &Served, phase: &str) -> Value {
    for _ in 0..200 {
        let attempt = shown(&served.kit).await;
        if attempt["phase"] == phase {
            return attempt;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!(
        "the attempt never reached {phase}: {}",
        shown(&served.kit).await
    );
}

/// Waits until the first run has ended.
async fn first_run_ended(pool: &PgPool) {
    for _ in 0..200 {
        let ended: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM sources.sync_runs WHERE trigger = 'connect' AND ended_at IS NOT NULL",
        )
        .fetch_one(pool)
        .await
        .unwrap();
        if ended == 1 {
            return;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("the first run did not end");
}

/// A browser bound by a fresh link, at GitHub with S1.
pub async fn at_github(served: &Served) -> (Browser, String) {
    let id = started(&served.kit).await;
    let link = link(&served.kit, &id).await;
    let mut browser = Browser::new(served);
    let s1 = browser.open(&link).await;
    (browser, s1)
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_039_three_presses_end_with_one_account_one_sealed_key_and_a_first_run(pool: PgPool) {
    let served = served(&pool, SLOW).await;
    set_projects(&pool, true).await;
    served.kit.stub.set_pages(&[&[("a", "1")]]);
    let (mut browser, s1) = at_github(&served).await;
    assert!(browser.cookies.contains_key("__Host-anima-connect"));
    assert_eq!(shown(&served.kit).await["phase"], "AT_GITHUB");

    let s2 = install_state(&browser.created(&s1).await);
    let made = shown(&served.kit).await;
    assert_eq!(made["phase"], "APP_CREATED");
    assert_eq!(
        (made["appSlug"].as_str(), made["ownerLogin"].as_str()),
        (Some("anima-k3f9x2mq7b"), Some("octo-person"))
    );
    assert_eq!(
        made["installUrl"],
        "https://github.com/apps/anima-k3f9x2mq7b/installations/new"
    );

    let kept = browser.cookies.clone();
    let done = browser.installed(INSTALLATION, &s2).await;
    assert_eq!(done.status, StatusCode::OK);
    assert!(done.body.contains("GitHub is connected."), "{}", done.body);
    assert!(
        !browser.cookies.contains_key("__Host-anima-connect"),
        "the done page clears it"
    );
    let connected = shown(&served.kit).await;
    assert_eq!(connected["phase"], "CONNECTED");
    assert_eq!(
        connected["account"]["label"], "octo-person",
        "the owner's login"
    );
    assert_eq!(connected["account"]["credentialStored"], true);
    first_run_ended(&pool).await;
    assert_eq!(stored(&pool).await, (1, 1, 1));
    let instance: String = sqlx::query_scalar("SELECT instance_key FROM sources.accounts")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(instance, "1234567/4321");

    browser.cookies = kept;
    let again = browser.installed(INSTALLATION, &s2).await;
    assert!(
        again.body.contains("GitHub is connected."),
        "the done page again"
    );
    assert_eq!(stored(&pool).await, (1, 1, 1), "stored once");
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_039_the_poll_finds_the_installation_when_the_browser_never_comes_back(pool: PgPool) {
    let served = served(&pool, Duration::from_millis(50)).await;
    served
        .kit
        .stub
        .reply("/app/installations", reply(200, LIST));
    let (mut browser, s1) = at_github(&served).await;
    install_state(&browser.created(&s1).await);
    let connected = until_phase(&served, "CONNECTED").await;
    assert_eq!(connected["account"]["label"], "octo-person");
    assert!(
        connected["message"]
            .as_str()
            .unwrap()
            .contains("Projects is off"),
        "{connected}"
    );
    assert_eq!(
        stored(&pool).await,
        (1, 1, 0),
        "no first run while Projects is off"
    );
    let asked = served.kit.stub.gets();
    assert!(
        asked
            .iter()
            .any(|path| path == "/app/installations?per_page=100"),
        "{asked:?}"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_039_a_replayed_return_from_github_converts_once(pool: PgPool) {
    let served = served(&pool, SLOW).await;
    let (mut browser, s1) = at_github(&served).await;
    let first = install_state(&browser.created(&s1).await);
    let second = install_state(&browser.created(&s1).await);
    assert_ne!(first, second, "a new state for the install page");
    assert_eq!(conversions(&served.kit), 1, "the stub saw one conversion");
    assert_eq!(shown(&served.kit).await["phase"], "APP_CREATED");
    let mut stranger = Browser::new(&served);
    let refused = stranger.created(&s1).await;
    assert_eq!(refused.status, StatusCode::BAD_REQUEST);
    assert_eq!(conversions(&served.kit), 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_039_a_forged_installation_is_ignored(pool: PgPool) {
    let served = served(&pool, SLOW).await;
    // Installation 4399: this App, but in another account.
    let other = super::browser::ONE
        .replace("4321", "4399")
        .replace("\"id\": 9001", "\"id\": 9002");
    served
        .kit
        .stub
        .reply("/app/installations/4399", reply(200, &other));
    let (mut browser, s1) = at_github(&served).await;
    let s2 = install_state(&browser.created(&s1).await);
    for forged in ["4399", "4321/../4399", "99999", "-1", "4321%2F"] {
        let answer = browser.installed(forged, &s2).await;
        assert_eq!(answer.status, StatusCode::OK, "{forged}");
        assert!(
            answer.body.contains("Anima finishes the connection"),
            "{forged}"
        );
        assert_eq!(shown(&served.kit).await["phase"], "APP_CREATED", "{forged}");
    }
    assert_eq!(stored(&pool).await, (0, 0, 0));
    let done = browser.installed(INSTALLATION, &s2).await;
    assert!(done.body.contains("GitHub is connected."));
    assert_eq!(stored(&pool).await.0, 1);
}
