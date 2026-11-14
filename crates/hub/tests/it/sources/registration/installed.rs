//! PRJ-041 (K-E1 Sections 7a and 10a): the bound browser back from Install gets the page of how
//! the attempt ended, even when the installation watch, not this request, stores the connection.
//! The page waits for the attempt to end up to a bound, then shows the "finishing" page.

use std::time::{Duration, Instant};

use reqwest::StatusCode;
use sqlx::PgPool;

use super::{
    browser::{
        Answer, Browser, INSTALLATION, LIST, Served, install_state, reply, served_waiting, shown,
    },
    flow::at_github,
};

const SLOW: Duration = Duration::from_secs(3600);
const MINT: &str = "/app/installations/4321/access_tokens";

/// The bound browser, back on `/installed` from Install with the right state.
async fn back_from_install(served: &Served) -> (Browser, Answer) {
    let (mut browser, s1) = at_github(served).await;
    let s2 = install_state(&browser.created(&s1).await);
    let back = browser.installed(INSTALLATION, &s2).await;
    (browser, back)
}

fn assert_connected_page(browser: &Browser, page: &Answer) {
    assert_eq!(page.status, StatusCode::OK);
    assert!(page.body.contains("GitHub is connected."), "{}", page.body);
    assert!(
        !browser.cookies.contains_key("__Host-anima-connect"),
        "the done page clears the cookie"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_041_the_bound_browser_gets_the_connected_page_when_the_watch_finds_the_installation(
    pool: PgPool,
) {
    let served = served_waiting(&pool, SLOW, Duration::from_secs(10)).await;
    // GitHub does not yet show the installation by its ID; the App's list already does.
    let stub = &served.kit.stub;
    stub.reply(
        &format!("/app/installations/{INSTALLATION}"),
        reply(404, "{}"),
    );
    stub.reply("/app/installations", reply(200, LIST));
    let (browser, back) = back_from_install(&served).await;
    assert_connected_page(&browser, &back);
    assert_eq!(shown(&served.kit).await["phase"], "CONNECTED");
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_041_the_bound_browser_gets_the_connected_page_while_the_watch_stores(pool: PgPool) {
    let served = served_waiting(&pool, Duration::from_millis(50), Duration::from_secs(10)).await;
    let stub = &served.kit.stub;
    stub.reply("/app/installations", reply(200, LIST));
    // The watch's store mints a token that GitHub answers in 1 s.
    stub.hook(MINT, "SELECT pg_sleep(1)");
    let (mut browser, s1) = at_github(&served).await;
    let s2 = install_state(&browser.created(&s1).await);
    for _ in 0..500 {
        if stub.paths().iter().any(|path| path == MINT) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(
        stub.paths().iter().any(|path| path == MINT),
        "the watch is storing"
    );
    let back = browser.installed(INSTALLATION, &s2).await;
    assert_connected_page(&browser, &back);
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_041_an_attempt_that_does_not_end_within_the_bound_gets_the_finishing_page(
    pool: PgPool,
) {
    let bound = Duration::from_secs(1);
    let served = served_waiting(&pool, SLOW, bound).await;
    // GitHub shows the installation neither by its ID nor in the App's list.
    let stub = &served.kit.stub;
    stub.reply(
        &format!("/app/installations/{INSTALLATION}"),
        reply(404, "{}"),
    );
    let (mut browser, s1) = at_github(&served).await;
    let s2 = install_state(&browser.created(&s1).await);
    let asked = Instant::now();
    let back = browser.installed(INSTALLATION, &s2).await;
    let took = asked.elapsed();
    assert_eq!(back.status, StatusCode::OK);
    assert!(
        back.body.contains("Anima finishes the connection"),
        "{}",
        back.body
    );
    assert!(
        browser.cookies.contains_key("__Host-anima-connect"),
        "the cookie stays"
    );
    assert!(
        took >= bound,
        "the page waited for the attempt to end: {took:?}"
    );
    assert!(
        took < bound + Duration::from_secs(3),
        "the page came within the bound: {took:?}"
    );
    assert_eq!(shown(&served.kit).await["phase"], "APP_CREATED");
}
