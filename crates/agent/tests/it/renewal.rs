//! Renewal against the fake hub: inside and outside the last 30 days (MET-013, K8 Section 5).

use anima_agent::push::Event;
use anima_agent::token_store::TokenStore;
use chrono::TimeDelta;

use crate::fake_hub::{FakeHub, Reply};
use crate::kit::{at, points, rig, token};

fn renewed(seed: u8, expires_in: TimeDelta) -> Reply {
    let body = format!(
        r#"{{"token":"{}","expiresAt":"{}"}}"#,
        token(seed).expose(),
        (at(0) + expires_in).format("%Y-%m-%dT%H:%M:%SZ")
    );
    Reply::json(200, &body)
}

fn feed(rig: &mut crate::kit::Rig, first: i64, count: i64) {
    for point in points(first, count) {
        rig.pusher.accept(Event::Point(point));
    }
}

#[tokio::test]
async fn met_013_inside_the_last_30_days_the_agent_renews_stores_the_new_token_and_uses_it() {
    let hub = FakeHub::start().await;
    let mut rig = rig(&hub, 1, TimeDelta::days(10));
    hub.queue_renew(renewed(2, TimeDelta::days(365)));
    feed(&mut rig, 1, 2);
    rig.pusher.push_due(at(30)).await;
    let renewals = hub.seen("renew");
    assert_eq!(renewals.len(), 1);
    assert_eq!(
        renewals[0].bearer.as_deref(),
        Some(token(1).expose()),
        "renews with the current token"
    );
    assert_eq!(renewals[0].body, serde_json::json!({}));
    let stored = TokenStore::open(&rig.home)
        .load()
        .expect("reads")
        .expect("a token");
    assert_eq!(stored.token.expose(), token(2).expose());
    assert_eq!(
        stored.expires_at,
        at(0) + TimeDelta::days(365),
        "the expiry is stored with it"
    );
    assert_eq!(
        hub.seen("push")[0].bearer.as_deref(),
        Some(token(2).expose()),
        "the push uses the new token"
    );
}

#[tokio::test]
async fn met_013_outside_the_last_30_days_the_agent_does_not_renew() {
    let hub = FakeHub::start().await;
    let mut rig = rig(&hub, 1, TimeDelta::days(31));
    feed(&mut rig, 1, 2);
    rig.pusher.push_due(at(30)).await;
    assert!(hub.seen("renew").is_empty());
    assert_eq!(
        hub.seen("push")[0].bearer.as_deref(),
        Some(token(1).expose())
    );
}

#[tokio::test]
async fn k8_5_a_failed_renewal_is_tried_again_after_an_hour_not_every_round() {
    let hub = FakeHub::start().await;
    let mut rig = rig(&hub, 1, TimeDelta::days(10));
    feed(&mut rig, 1, 1);
    rig.pusher.push_due(at(20)).await;
    rig.pusher.push_due(at(30)).await;
    assert_eq!(
        hub.seen("renew").len(),
        1,
        "the 503 is not retried within the hour"
    );
    rig.pusher.push_due(at(30 + 3600)).await;
    assert_eq!(hub.seen("renew").len(), 2);
}

#[cfg(unix)]
#[tokio::test]
async fn k8_5_a_token_that_cannot_be_stored_is_not_used_and_the_current_one_stays() {
    use std::os::unix::fs::PermissionsExt as _;
    let hub = FakeHub::start().await;
    let mut rig = rig(&hub, 1, TimeDelta::days(10));
    hub.queue_renew(renewed(2, TimeDelta::days(365)));
    // A folder nobody can write to: the store cannot take the new token.
    std::fs::set_permissions(rig.home.dir(), std::fs::Permissions::from_mode(0o500))
        .expect("chmod");
    let writable = tempfile::tempfile_in(rig.home.dir()).is_ok();
    feed(&mut rig, 1, 2);
    rig.pusher.push_due(at(30)).await;
    std::fs::set_permissions(rig.home.dir(), std::fs::Permissions::from_mode(0o700))
        .expect("chmod");
    if writable {
        return; // Running as a user the mode does not bind (root): nothing to prove.
    }
    assert_eq!(
        hub.seen("push")[0].bearer.as_deref(),
        Some(token(1).expose())
    );
    let stored = TokenStore::open(&rig.home)
        .load()
        .expect("reads")
        .expect("a token");
    assert_eq!(stored.token.expose(), token(1).expose());
}

#[tokio::test]
async fn met_013_a_renewed_far_future_expiry_is_capped_and_the_hub_stays_with_the_token() {
    let hub = FakeHub::start().await;
    let mut rig = rig(&hub, 1, TimeDelta::days(1));
    let far = format!(
        r#"{{"token":"{}","expiresAt":"9999-12-31T23:59:59Z"}}"#,
        token(9).expose()
    );
    hub.queue_renew(Reply::json(200, &far));
    rig.pusher.push_due(at(0)).await;
    let stored = TokenStore::open(&rig.home)
        .load()
        .expect("reads")
        .expect("a token");
    assert_eq!(stored.token.expose(), token(9).expose());
    assert_eq!(stored.expires_at, at(0) + TimeDelta::days(366));
}
