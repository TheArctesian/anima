//! PRJ-041, K-E1 Section 7a: each refusal of the flow leaves the attempt as it was: a used or
//! wrong ticket, a wrong or missing state, a missing cookie, a second browser, and `/open`
//! without a session or from another origin.

use std::time::Duration;

use reqwest::StatusCode;
use sqlx::PgPool;

use super::{
    browser::{Browser, INSTALLATION, PUBLIC, install_state, served, shown},
    flow::at_github,
    link, started,
};

const SLOW: Duration = Duration::from_secs(3600);

#[sqlx::test(migrations = "./migrations")]
async fn prj_041_a_used_or_wrong_ticket_gets_the_fixed_page(pool: PgPool) {
    let served = served(&pool, SLOW).await;
    let id = started(&served.kit).await;
    let link = link(&served.kit, &id).await;
    let path = link.strip_prefix(PUBLIC).unwrap().to_owned();
    let mut first = Browser::new(&served);
    first.open(&link).await;
    let mut second = Browser::new(&served);
    for wrong in [path.clone(), "/connect/github?ticket=AAAA".to_owned()] {
        let refused = second.get(&wrong).await;
        assert_eq!(refused.status, StatusCode::BAD_REQUEST, "{wrong}");
        assert!(
            refused
                .body
                .contains("This link was already used or has ended.")
        );
        assert!(second.cookies.is_empty(), "{wrong}: not bound");
    }
    let page = first.get("/connect/github/start").await;
    assert_eq!(
        page.status,
        StatusCode::OK,
        "the first browser keeps the attempt"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_039_a_wrong_state_a_missing_cookie_and_a_second_browser_change_nothing(pool: PgPool) {
    let served = served(&pool, SLOW).await;
    let (mut browser, s1) = at_github(&served).await;
    let before = shown(&served.kit).await;
    let mut cookieless = Browser::new(&served);
    let mut second = Browser::new(&served);
    second
        .cookies
        .insert("__Host-anima-connect".to_owned(), "x".repeat(43));
    let answers = [
        browser.created("wrong").await,
        browser
            .get("/connect/github/created?code=CODEMARKER7c1f")
            .await,
        cookieless.created(&s1).await,
        second.created(&s1).await,
        cookieless.get("/connect/github/start").await,
        second.get("/connect/github/start").await,
    ];
    for answer in answers {
        assert_eq!(answer.status, StatusCode::BAD_REQUEST, "{}", answer.body);
        assert!(answer.body.contains("another browser"));
    }
    assert_eq!(shown(&served.kit).await, before, "unchanged");
    assert_eq!(
        super::browser::conversions(&served.kit),
        0,
        "nothing converted"
    );
    let made = browser.created(&s1).await;
    assert_eq!(
        made.status,
        StatusCode::SEE_OTHER,
        "no lockout: {}",
        made.body
    );
}

/// The install link carries S2 and GitHub sends it back, so the bound browser back from
/// Install without a state is treated as one with a wrong state: it only wakes the poll and
/// the hub looks up no installation it named.
#[sqlx::test(migrations = "./migrations")]
async fn prj_039_the_bound_browser_back_from_install_without_a_state_looks_up_nothing(
    pool: PgPool,
) {
    let served = served(&pool, SLOW).await;
    let (mut browser, s1) = at_github(&served).await;
    install_state(&browser.created(&s1).await);
    let lookup = format!("/app/installations/{INSTALLATION}");

    let back = browser
        .get(&format!(
            "/connect/github/installed?installation_id={INSTALLATION}&setup_action=install"
        ))
        .await;

    assert_eq!(back.status, StatusCode::OK);
    assert!(
        back.body.contains("If you installed the App"),
        "{}",
        back.body
    );
    let paths = served.kit.stub.paths();
    assert!(!paths.contains(&lookup), "{paths:?}");
    assert_eq!(shown(&served.kit).await["phase"], "APP_CREATED");
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_041_open_needs_a_browser_session_from_anima(pool: PgPool) {
    let served = served(&pool, SLOW).await;
    started(&served.kit).await;
    let mut browser = Browser::new(&served);
    let unsigned = browser
        .post("/connect/github/open", &[("origin", PUBLIC)])
        .await;
    assert_eq!(unsigned.status, StatusCode::UNAUTHORIZED);
    assert!(
        unsigned
            .body
            .contains("Sign in to Anima.ing in this browser")
    );

    browser.sign_in().await;
    for origin in [&[("origin", "https://evil.example")][..], &[]] {
        let refused = browser.post("/connect/github/open", origin).await;
        assert_eq!(refused.status, StatusCode::FORBIDDEN, "{origin:?}");
        assert!(!browser.cookies.contains_key("__Host-anima-connect"));
    }
    let bearer = format!("Bearer {}", served.kit.token);
    let mut script = Browser::new(&served);
    let refused = script
        .post(
            "/connect/github/open",
            &[("origin", PUBLIC), ("authorization", &bearer)],
        )
        .await;
    assert_eq!(
        refused.status,
        StatusCode::UNAUTHORIZED,
        "a bearer is not a browser session"
    );
    assert_eq!(shown(&served.kit).await["phase"], "WAITING_FOR_BROWSER");

    let bound = browser
        .post("/connect/github/open", &[("origin", PUBLIC)])
        .await;
    assert_eq!(bound.status, StatusCode::SEE_OTHER, "{}", bound.body);
    assert_eq!(bound.location(), "/connect/github/start");
    assert!(browser.cookies.contains_key("__Host-anima-connect"));
    assert_eq!(shown(&served.kit).await["phase"], "AT_GITHUB");
    browser.form_state().await;
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_041_open_without_a_waiting_attempt_is_409(pool: PgPool) {
    let served = served(&pool, SLOW).await;
    let mut browser = Browser::new(&served);
    browser.sign_in().await;
    let none = browser
        .post("/connect/github/open", &[("origin", PUBLIC)])
        .await;
    assert_eq!(none.status, StatusCode::CONFLICT);
    assert!(none.body.contains("No GitHub connection is waiting."));
}
