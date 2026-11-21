//! A hosted hub at its own address, `https://<hub id>.hub.anima.ing` (hosted-mode.md HMD-061,
//! HMD-062, HMD-069; the review HM-T32). Each other name under `hub.anima.ing` is the hub of a
//! different account, and the same site for a browser, so `SameSite` does not keep it out: the
//! hub answers only its own name, and reads its cookie only from its own origin. The browser
//! session ends 30 minutes after its last request and 12 hours after its sign-in.

use std::sync::{Arc, Mutex, PoisonError};

use anima_hub::clock::{Clock, FixedClock};
use axum::Router;
use chrono::{DateTime, TimeDelta, Utc};
use serde_json::{Value, json};
use sqlx::PgPool;

use super::{
    control::{FakeLink, code, ok},
    hosted::{hosted_env, parts_of},
    hosted_browser::{SIGN_IN_CALL, post, serving, set_cookie, ticket},
};
use crate::hub::start;

const OWN_HOST: &str = "h0123456789abcdefghij.hub.anima.ing";
const OWN: &str = "https://h0123456789abcdefghij.hub.anima.ing";
/// The hub of a different account.
const SIBLING: &str = "https://h9876543210zyxwvutsrq.hub.anima.ing";
/// The one address of every hub before the move (AC-D45 of anima-cloud).
const SHARED: &str = "https://hub.anima.ing";
const ME: &str = "{ me { username } }";

/// A clock that a test moves on.
struct Moving(Mutex<DateTime<Utc>>);

impl Moving {
    fn pass(&self, time: TimeDelta) {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner) += time;
    }
}

impl Clock for Moving {
    fn now(&self) -> DateTime<Utc> {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// What a test keeps while it runs: the fake link and the folder of the cloud key.
type Kept = (FakeLink, tempfile::TempDir);

/// A hosted hub with the public origin `origin`, or with none, as the pool gives it.
async fn hub_at(pool: &PgPool, origin: Option<&str>, clock: Arc<dyn Clock>) -> (Router, Kept) {
    let (link, folder) = (
        FakeLink::start(|_| ok(&json!([]))),
        tempfile::tempdir().unwrap(),
    );
    let mut env = hosted_env(&link, &folder);
    if let Some(origin) = origin {
        env.insert("ANIMA_PUBLIC_ORIGIN".to_owned(), origin.to_owned());
    }
    (
        serving(pool, parts_of(pool, &env, clock)).await,
        (link, folder),
    )
}

fn fixed() -> Arc<dyn Clock> {
    Arc::new(FixedClock(start()))
}

/// The `Cookie` header of a browser that the gateway signed in at `origin` with `host`.
async fn signed_in(hub: &Router, origin: &str, host: &str) -> Option<String> {
    let ticket = ticket(hub).await;
    let from = [("origin", origin), ("host", host)];
    let (headers, _) = post(hub, &from, SIGN_IN_CALL, json!({ "t": ticket })).await;
    set_cookie(&headers).map(|cookie| cookie.split(';').next().unwrap().to_owned())
}

/// `{ me }` with the cookie, from the hub's own origin and name.
async fn me(hub: &Router, cookie: &str) -> Value {
    let from = [("origin", OWN), ("host", OWN_HOST), ("cookie", cookie)];
    post(hub, &from, ME, json!({})).await.1
}

#[sqlx::test(migrations = "./migrations")]
async fn hmd_069_a_hub_at_its_own_address_answers_only_its_own_name(pool: PgPool) {
    let (hub, _link) = hub_at(&pool, Some(OWN), fixed()).await;
    let own_port = format!("{OWN_HOST}:443");
    for host in [OWN_HOST, own_port.as_str()] {
        let (_, answer) = post(
            &hub,
            &[("origin", OWN), ("host", host)],
            "{ __typename }",
            json!({}),
        )
        .await;
        assert!(answer["data"]["__typename"].is_string(), "{host}: {answer}");
    }
    let refused = [
        "hub.anima.ing",
        "h9876543210zyxwvutsrq.hub.anima.ing",
        "x.h0123456789abcdefghij.hub.anima.ing",
        "h0123456789abcdefghij.hub.anima.ing.evil.example",
        "h0123456789abcdefghij.hub.anima.ing.",
        "h0123456789abcdefghij.hub.anima.ing:8443",
    ];
    for host in refused {
        let (_, answer) = post(
            &hub,
            &[("origin", OWN), ("host", host)],
            "{ __typename }",
            json!({}),
        )
        .await;
        assert_eq!(answer, Value::Null, "{host}: a bare 403");
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn hmd_061_a_sibling_hub_or_the_old_address_cannot_use_the_cookie_of_this_hub(pool: PgPool) {
    let (hub, _link) = hub_at(&pool, Some(OWN), fixed()).await;
    for origin in [SIBLING, SHARED] {
        let traded = signed_in(&hub, origin, OWN_HOST).await;
        assert!(traded.is_none(), "{origin} gets no session of this hub");
    }
    let cookie = signed_in(&hub, OWN, OWN_HOST)
        .await
        .expect("the cookie of the hub");
    assert_eq!(me(&hub, &cookie).await["data"]["me"]["username"], "owner");
    for origin in [
        SIBLING,
        SHARED,
        "null",
        "http://h0123456789abcdefghij.hub.anima.ing",
    ] {
        let from = [
            ("origin", origin),
            ("host", OWN_HOST),
            ("cookie", cookie.as_str()),
        ];
        let (_, answer) = post(&hub, &from, ME, json!({})).await;
        assert_eq!(answer, Value::Null, "{origin}: a bare 403");
    }
    let no_origin = [("host", OWN_HOST), ("cookie", cookie.as_str())];
    let (_, answer) = post(&hub, &no_origin, ME, json!({})).await;
    assert_eq!(
        code(&answer),
        "UNAUTHENTICATED",
        "no origin, no cookie: {answer}"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn hmd_069_a_hosted_hub_that_was_given_no_address_takes_no_browser(pool: PgPool) {
    let (hub, _link) = hub_at(&pool, None, fixed()).await;
    for (origin, host) in [(SHARED, "hub.anima.ing"), (SHARED, "127.0.0.1:8443")] {
        let traded = signed_in(&hub, origin, host).await;
        assert!(
            traded.is_none(),
            "{origin} at {host}: no session at the old shared address"
        );
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn hmd_062_a_browser_session_ends_after_30_minutes_with_no_request(pool: PgPool) {
    let clock = Arc::new(Moving(Mutex::new(start())));
    let (hub, _link) = hub_at(&pool, Some(OWN), clock.clone()).await;
    let cookie = signed_in(&hub, OWN, OWN_HOST).await.expect("a cookie");
    for _ in 0..3 {
        clock.pass(TimeDelta::minutes(29));
        assert!(
            me(&hub, &cookie).await["errors"].is_null(),
            "used within 30 minutes"
        );
    }
    clock.pass(TimeDelta::minutes(30));
    assert_eq!(code(&me(&hub, &cookie).await), "UNAUTHENTICATED");
}

#[sqlx::test(migrations = "./migrations")]
async fn hmd_062_a_browser_session_ends_12_hours_after_its_sign_in_however_it_is_used(
    pool: PgPool,
) {
    let clock = Arc::new(Moving(Mutex::new(start())));
    let (hub, _link) = hub_at(&pool, Some(OWN), clock.clone()).await;
    let cookie = signed_in(&hub, OWN, OWN_HOST).await.expect("a cookie");
    for _ in 0..28 {
        clock.pass(TimeDelta::minutes(25));
        assert!(
            me(&hub, &cookie).await["errors"].is_null(),
            "used every 25 minutes"
        );
    }
    clock.pass(TimeDelta::minutes(20));
    assert_eq!(
        code(&me(&hub, &cookie).await),
        "UNAUTHENTICATED",
        "12 hours"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn hmd_062_a_sign_out_ends_the_browser_session_and_clears_its_cookie(pool: PgPool) {
    let (hub, _link) = hub_at(&pool, Some(OWN), fixed()).await;
    let cookie = signed_in(&hub, OWN, OWN_HOST).await.expect("a cookie");
    let from = [
        ("origin", OWN),
        ("host", OWN_HOST),
        ("cookie", cookie.as_str()),
    ];
    let (headers, answer) = post(&hub, &from, "mutation { logout }", json!({})).await;
    assert_eq!(answer["data"]["logout"], true, "{answer}");
    let cleared = headers
        .get("set-cookie")
        .and_then(|value| value.to_str().ok());
    assert!(cleared.is_some_and(|value| value.starts_with("__Host-anima=; Max-Age=0")));
    assert_eq!(code(&me(&hub, &cookie).await), "UNAUTHENTICATED");
}
