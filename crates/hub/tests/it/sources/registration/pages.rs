//! K7, PRJ-041: the pages of the flow: every answer carries the required headers, the start
//! page's script is the one its policy hashes, the confirm page is the same with or without an
//! attempt, and no value from a query string is ever written into a page.

use std::time::Duration;

use base64::{Engine as _, engine::general_purpose::STANDARD};
use reqwest::StatusCode;
use sha2::{Digest as _, Sha256};
use sqlx::PgPool;

use super::{
    browser::{Answer, Browser, INSTALLATION, PUBLIC, install_state, served},
    flow::at_github,
    started,
};

const SLOW: Duration = Duration::from_secs(3600);
const MARKER: &str = "QUERYMARKER9e2";

fn assert_protected(answer: &Answer, what: &str) {
    let header = |name: &str| {
        answer
            .headers
            .get(name)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_else(|| panic!("{what}: no {name}"))
            .to_owned()
    };
    assert_eq!(header("cache-control"), "no-store", "{what}");
    assert_eq!(header("referrer-policy"), "no-referrer", "{what}");
    assert_eq!(header("x-content-type-options"), "nosniff", "{what}");
    assert_eq!(
        header("strict-transport-security"),
        "max-age=31536000",
        "{what}"
    );
    assert_eq!(
        header("cross-origin-opener-policy"),
        "same-origin",
        "{what}"
    );
    let policy = header("content-security-policy");
    assert!(
        policy.contains("frame-ancestors 'none'"),
        "{what}: {policy}"
    );
    assert!(policy.starts_with("default-src 'none'"), "{what}: {policy}");
}

#[sqlx::test(migrations = "./migrations")]
async fn k7_every_answer_of_the_flow_carries_the_headers(pool: PgPool) {
    let served = served(&pool, SLOW).await;
    let mut stranger = Browser::new(&served);
    let mut answers = vec![
        ("confirm", stranger.get("/connect/github").await),
        ("bad ticket", stranger.get("/connect/github?ticket=x").await),
        (
            "open unsigned",
            stranger
                .post("/connect/github/open", &[("origin", PUBLIC)])
                .await,
        ),
        (
            "open foreign",
            stranger.post("/connect/github/open", &[]).await,
        ),
        ("start unbound", stranger.get("/connect/github/start").await),
        ("created unbound", stranger.created("x").await),
        ("installed unbound", stranger.installed("1", "x").await),
        (
            "wrong method",
            stranger.post("/connect/github/start", &[]).await,
        ),
    ];
    let id = started(&served.kit).await;
    let link = super::link(&served.kit, &id).await;
    let mut browser = Browser::new(&served);
    answers.push((
        "ticket",
        browser.get(link.strip_prefix(PUBLIC).unwrap()).await,
    ));
    answers.push(("start", browser.get("/connect/github/start").await));
    let s1 = answers
        .last()
        .unwrap()
        .1
        .body
        .split("?state=")
        .nth(1)
        .unwrap();
    let s1 = s1.split('"').next().unwrap().to_owned();
    let made = browser.created(&s1).await;
    let s2 = install_state(&made);
    answers.push(("created", made));
    answers.push(("installed", browser.installed(INSTALLATION, &s2).await));
    for (what, answer) in &answers {
        assert_protected(answer, what);
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_039_the_start_page_runs_only_the_script_its_policy_hashes(pool: PgPool) {
    let served = served(&pool, SLOW).await;
    let (mut browser, _) = at_github(&served).await;
    let page = browser.get("/connect/github/start").await;
    let policy = page.headers["content-security-policy"].to_str().unwrap();
    let script = page
        .body
        .split("<script>")
        .nth(1)
        .and_then(|rest| rest.split("</script>").next())
        .unwrap();
    let hash = STANDARD.encode(Sha256::digest(script.as_bytes()));
    assert_eq!(
        policy,
        format!(
            "default-src 'none'; script-src 'sha256-{hash}'; style-src 'unsafe-inline'; \
             form-action https://github.com; base-uri 'none'; frame-ancestors 'none'"
        )
    );
    assert_eq!(page.body.matches("<script").count(), 1);
    assert!(
        page.body.contains("Continue to GitHub"),
        "a button for no scripts"
    );
    assert!(
        page.body.contains("name=\"manifest\" value=\"{&quot;"),
        "the escaped manifest"
    );
    assert!(
        page.body
            .contains("https://hub.anima.ing/connect/github/created")
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_041_the_confirm_page_is_the_same_with_or_without_an_attempt(pool: PgPool) {
    let served = served(&pool, SLOW).await;
    let mut browser = Browser::new(&served);
    let without = browser.get("/connect/github").await;
    started(&served.kit).await;
    let with = browser.get("/connect/github").await;
    assert_eq!(without.status, StatusCode::OK);
    assert_eq!(with.status, StatusCode::OK);
    assert_eq!(without.body, with.body);
    assert!(with.body.contains("action=\"/connect/github/open\""));
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_041_no_query_value_is_written_into_a_page(pool: PgPool) {
    let served = served(&pool, SLOW).await;
    let (mut browser, s1) = at_github(&served).await;
    let mut stranger = Browser::new(&served);
    let marked = format!("{MARKER}%3Cscript%3E{MARKER}<b>");
    let mut bodies = Vec::new();
    for path in [
        format!("/connect/github?ticket={marked}"),
        format!("/connect/github?x={marked}"),
        format!("/connect/github/start?state={marked}"),
        format!("/connect/github/created?code={marked}&state={marked}"),
        format!("/connect/github/installed?installation_id={marked}&state={marked}"),
    ] {
        bodies.push(stranger.get(&path).await.body);
        bodies.push(browser.get(&path).await.body);
    }
    bodies.push(
        browser
            .get(&format!("/connect/github/created?code={MARKER}&state={s1}"))
            .await
            .body,
    );
    bodies.push(
        browser
            .get(&format!(
                "/connect/github/installed?installation_id={MARKER}&setup_action={MARKER}"
            ))
            .await
            .body,
    );
    for body in bodies {
        assert!(!body.contains(MARKER), "{body}");
    }
}
