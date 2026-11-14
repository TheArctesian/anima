//! PRJ-039, PRJ-041, NET-002 (K-E1 Section 7a): what the security review of the flow found,
//! held as behaviour. A browser that leaves in the middle of `/created` or `/installed` does
//! not stop the work it started; a device that is not the bound browser makes the hub ask
//! GitHub nothing; a cross-site hit on `/start` or `/created` changes nothing.

use std::time::Duration;

use reqwest::StatusCode;
use serde_json::Value;
use sqlx::PgPool;

use super::{
    browser::{
        Browser, CONVERSION, INSTALLATION, Served, conversions, install_state, served, shown,
    },
    flow::at_github,
};

const SLOW: Duration = Duration::from_secs(3600);

async fn accounts(pool: &PgPool) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM sources.accounts")
        .fetch_one(pool)
        .await
        .unwrap()
}

/// How many times the hub asked GitHub for the App's installations.
fn lists(served: &Served) -> usize {
    let asked = served.kit.stub.gets();
    asked
        .iter()
        .filter(|path| path.starts_with("/app/installations?"))
        .count()
}

/// Waits up to 10 seconds until `githubConnect` shows `phase`.
async fn until_phase(served: &Served, phase: &str) -> Value {
    for _ in 0..400 {
        let attempt = shown(&served.kit).await;
        if attempt["phase"] == phase {
            return attempt;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("never reached {phase}: {}", shown(&served.kit).await);
}

/// `GET path` with `browser`'s cookies from a browser that gives up after 300 ms.
async fn given_up(served: &Served, browser: &Browser, path: &str) {
    let cookie: Vec<String> = (browser.cookies.iter())
        .map(|(name, value)| format!("{name}={value}"))
        .collect();
    let impatient = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_millis(300))
        .build()
        .unwrap();
    let sent = impatient
        .get(format!("{}{path}", served.base))
        .header("cookie", cookie.join("; "))
        .send()
        .await;
    assert!(sent.is_err(), "the browser gave up");
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_039_a_dropped_installed_request_still_ends_connected_with_one_account(pool: PgPool) {
    let served = served(&pool, Duration::from_millis(50)).await;
    let (mut browser, s1) = at_github(&served).await;
    let s2 = install_state(&browser.created(&s1).await);
    // GitHub's mint answers in 2 s; the browser gives up first.
    let mint = format!("/app/installations/{INSTALLATION}/access_tokens");
    served.kit.stub.hook(&mint, "SELECT pg_sleep(2)");
    let path = format!(
        "/connect/github/installed?installation_id={INSTALLATION}&setup_action=install&state={s2}"
    );
    given_up(&served, &browser, &path).await;
    let connected = until_phase(&served, "CONNECTED").await;
    assert_eq!(connected["account"]["label"], "octo-person");
    assert_eq!(accounts(&pool).await, 1, "stored once");
    let again = browser.installed(INSTALLATION, &s2).await;
    assert!(
        again.body.contains("GitHub is connected."),
        "{}",
        again.body
    );
    assert_eq!(accounts(&pool).await, 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_039_a_created_request_dropped_mid_conversion_reaches_app_created_and_is_audited(
    pool: PgPool,
) {
    let served = served(&pool, SLOW).await;
    let (mut browser, s1) = at_github(&served).await;
    // GitHub's conversion answers in 2 s; the browser gives up first.
    served.kit.stub.hook(CONVERSION, "SELECT pg_sleep(2)");
    let path = format!("/connect/github/created?code=CODEMARKER7c1f&state={s1}");
    given_up(&served, &browser, &path).await;
    let made = until_phase(&served, "APP_CREATED").await;
    assert_eq!(made["appSlug"], "anima-k3f9x2mq7b");
    let audited: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM events.audit_log WHERE target_id = 'sources.github.connect' \
         AND change->>'slug' = 'anima-k3f9x2mq7b'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(audited, 1, "App created was audited");
    // The operator's browser comes back with S1 and is sent to Install; the flow goes on.
    let s2 = install_state(&browser.created(&s1).await);
    assert_eq!(conversions(&served.kit), 1, "converted once");
    let done = browser.installed(INSTALLATION, &s2).await;
    assert!(done.body.contains("GitHub is connected."), "{}", done.body);
}

#[sqlx::test(migrations = "./migrations")]
async fn net_002_prj_041_unbound_hits_on_installed_make_no_github_call(pool: PgPool) {
    let served = served(&pool, SLOW).await;
    let (mut browser, s1) = at_github(&served).await;
    install_state(&browser.created(&s1).await);
    let before = shown(&served.kit).await;
    let mut stranger = Browser::new(&served);
    for _ in 0..40 {
        let hit = stranger.get("/connect/github/installed").await;
        assert_eq!(hit.status, StatusCode::OK);
        assert!(
            hit.body.contains("If you installed the App"),
            "{}",
            hit.body
        );
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(lists(&served), 0, "no ask beyond the scheduled ones");
    assert_eq!(shown(&served.kit).await, before, "nothing changed");
}

#[sqlx::test(migrations = "./migrations")]
async fn net_002_wakes_from_the_bound_browser_within_the_spacing_make_one_github_call(
    pool: PgPool,
) {
    let served = served(&pool, SLOW).await;
    let (mut browser, s1) = at_github(&served).await;
    install_state(&browser.created(&s1).await);
    for _ in 0..10 {
        let hit = browser.get("/connect/github/installed?state=wrong").await;
        assert_eq!(hit.status, StatusCode::OK);
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(
        lists(&served),
        1,
        "the first wake asks; the rest are too soon"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_041_a_cross_site_hit_on_created_leaves_the_attempt_unchanged(pool: PgPool) {
    let served = served(&pool, SLOW).await;
    let (mut browser, s1) = at_github(&served).await;
    let s2 = install_state(&browser.created(&s1).await);
    let before = shown(&served.kit).await;
    // The hostile page: `location = "https://hub/connect/github/created"`, with the Lax cookie.
    for forged in [
        "/connect/github/created",
        "/connect/github/created?state=wrong",
    ] {
        let answer = browser.get(forged).await;
        assert_eq!(answer.status, StatusCode::OK, "{forged}");
        assert!(answer.headers.get("location").is_none(), "{forged}");
        assert!(answer.body.contains("The App is made."), "{forged}");
        assert!(!answer.body.contains(&s2), "{forged}: no state");
        assert_eq!(shown(&served.kit).await, before, "{forged}: unchanged");
    }
    assert_eq!(conversions(&served.kit), 1);
    let back = browser.installed(INSTALLATION, &s2).await;
    assert!(back.body.contains("GitHub is connected."), "{}", back.body);
    assert_eq!(accounts(&pool).await, 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_041_a_second_start_returns_the_same_state_and_the_first_return_still_exchanges(
    pool: PgPool,
) {
    let served = served(&pool, SLOW).await;
    let (mut browser, s1) = at_github(&served).await;
    // A cross-site hit, or the operator's reload: the same form, the same S1.
    assert_eq!(browser.form_state().await, s1, "the same state");
    let back = browser.created(&s1).await;
    assert_eq!(back.status, StatusCode::SEE_OTHER, "{}", back.body);
    assert_eq!(shown(&served.kit).await["phase"], "APP_CREATED");
    let mut stranger = Browser::new(&served);
    let refused = stranger.get("/connect/github/start").await;
    assert_eq!(
        refused.status,
        StatusCode::BAD_REQUEST,
        "no cookie, no form"
    );
}
