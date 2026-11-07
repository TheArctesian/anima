//! The live feed of session changes (RA-034): every client sees each start once its row is
//! committed and each end with its reason, and the subscription needs a signed-in operator.

use std::time::Duration;

use anima_hub::sessions::{EndReason, Live, SessionChange};
use async_graphql::{
    Request, Response, Variables,
    futures_util::{FutureExt as _, Stream, StreamExt as _},
};
use chrono::TimeDelta;
use serde_json::{Value, json};
use sqlx::PgPool;
use tokio::sync::broadcast::{Receiver, error::TryRecvError};

use super::{
    DOCKER_HOST, PVE_01,
    harness::{CLIENT_ADDR, error_json, schema_at, signed_in_from, signed_in_until},
    now,
    starting::{display, end_request, ready, start_request, started, stored},
};

const SUBSCRIPTION: &str = "subscription { sessionChanged { session { id clientAddr endedAt \
                            endReason } } }";

/// The change already published, failing when there is none.
fn published(changes: &mut Receiver<SessionChange>) -> SessionChange {
    changes.try_recv().expect("a change was published")
}

fn nothing_more(changes: &mut Receiver<SessionChange>) {
    assert_eq!(changes.try_recv().unwrap_err(), TryRecvError::Empty);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn ra_034_a_start_is_published_once_committed_then_its_end_with_the_reason(pool: PgPool) {
    ready(&pool, &[PVE_01]).await;
    let live = Live::default();
    let schema = schema_at(&pool, now(), &live);
    let mut changes = live.changes();

    let ticket = started(&schema, PVE_01).await;
    let id = ticket["sessionId"].as_str().unwrap();

    let start = published(&mut changes).session;
    assert_eq!(start.id.to_string(), id);
    assert_eq!(start.client.to_string(), CLIENT_ADDR);
    assert_eq!(start.end, None, "live after a start");
    assert_eq!(stored(&pool, id).await.end_reason, None, "the row is committed");
    nothing_more(&mut changes);

    let later = schema_at(&pool, now() + TimeDelta::seconds(5), &live);
    let response = later.execute(end_request(id)).await;
    assert!(response.errors.is_empty(), "{:?}", response.errors);

    let end = published(&mut changes).session;
    assert_eq!(end.id.to_string(), id);
    let ended = end.end.expect("ended after an end");
    assert_eq!(ended.reason, EndReason::Operator);
    assert_eq!(ended.at, now() + TimeDelta::seconds(5));
    nothing_more(&mut changes);

    // Ending it again changes nothing, so nothing is published.
    let response = later.execute(end_request(id)).await;
    assert!(response.errors.is_empty(), "{:?}", response.errors);
    nothing_more(&mut changes);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn ra_034_a_refused_start_publishes_nothing(pool: PgPool) {
    let live = Live::default();
    let schema = schema_at(&pool, now(), &live);
    let mut changes = live.changes();

    // No accepted host key: CONFLICT. A display outside the bounds: validation.
    let no_key = schema.execute(start_request(PVE_01, &display())).await;
    assert_eq!(error_json(&no_key)["extensions"]["code"], "CONFLICT");
    ready(&pool, &[PVE_01]).await;
    let too_wide = json!({ "widthPx": 9000, "heightPx": 800 });
    let invalid = schema.execute(start_request(PVE_01, &too_wide)).await;
    assert_eq!(error_json(&invalid)["extensions"]["code"], "VALIDATION");
    // The hub is shutting down: UNAVAILABLE, and the row rolls back.
    anima_hub::sessions::shut_down(&pool, &live, now())
        .await
        .unwrap();
    let unavailable = schema.execute(start_request(PVE_01, &display())).await;
    assert_eq!(error_json(&unavailable)["extensions"]["code"], "UNAVAILABLE");

    nothing_more(&mut changes);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn ra_034_shutdown_publishes_each_pending_session_s_hub_shutdown_end(pool: PgPool) {
    ready(&pool, &[PVE_01, DOCKER_HOST]).await;
    let live = Live::default();
    let schema = schema_at(&pool, now(), &live);
    let first = started(&schema, PVE_01).await;
    let second = started(&schema, DOCKER_HOST).await;
    let mut changes = live.changes();

    anima_hub::sessions::shut_down(&pool, &live, now() + TimeDelta::seconds(5))
        .await
        .unwrap();

    let mut ended: Vec<(String, Option<EndReason>)> = (0..2)
        .map(|_| {
            let session = published(&mut changes).session;
            (session.id.to_string(), session.end.map(|end| end.reason))
        })
        .collect();
    ended.sort();
    let mut expected: Vec<(String, Option<EndReason>)> = [first, second]
        .iter()
        .map(|ticket| {
            let id = ticket["sessionId"].as_str().unwrap().to_owned();
            (id, Some(EndReason::HubShutdown))
        })
        .collect();
    expected.sort();
    assert_eq!(ended, expected);
    nothing_more(&mut changes);
}

/// The subscription from the operator's own address, polled once so it has subscribed.
fn subscribed(stream: &mut (impl Stream<Item = Response> + Unpin)) {
    assert!(
        stream.next().now_or_never().is_none(),
        "no change before anything happened"
    );
}

/// The next change the subscription yields, as JSON.
async fn next_change(stream: &mut (impl Stream<Item = Response> + Unpin)) -> Value {
    let response = tokio::time::timeout(Duration::from_secs(5), stream.next())
        .await
        .expect("a change arrived")
        .expect("the stream is open");
    assert!(response.errors.is_empty(), "{:?}", response.errors);
    response.data.into_json().unwrap()["sessionChanged"]["session"].clone()
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn ra_034_every_client_sees_a_start_and_an_end_session_from_another_address(pool: PgPool) {
    ready(&pool, &[PVE_01]).await;
    let live = Live::default();
    let schema = schema_at(&pool, now(), &live);
    let mut stream = Box::pin(schema.execute_stream(signed_in_until(SUBSCRIPTION, 24)));
    subscribed(&mut stream);

    let ticket = started(&schema, PVE_01).await;
    let id = ticket["sessionId"].as_str().unwrap();
    let start = next_change(&mut stream).await;
    assert_eq!(
        start,
        json!({ "id": id, "clientAddr": CLIENT_ADDR, "endedAt": null, "endReason": null })
    );

    let end = "mutation($id: ID!) { endSession(id: $id) { endReason } }";
    let elsewhere = signed_in_from(end, "10.10.0.99", 24)
        .variables(Variables::from_json(json!({ "id": id })));
    let response = schema.execute(elsewhere).await;
    assert!(response.errors.is_empty(), "{:?}", response.errors);

    let ended = next_change(&mut stream).await;
    assert_eq!(ended["id"], id);
    assert_eq!(ended["clientAddr"], CLIENT_ADDR, "the address that started it");
    assert_eq!(ended["endReason"], "OPERATOR");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn api_003_session_changed_refuses_a_request_without_an_operator(pool: PgPool) {
    ready(&pool, &[PVE_01]).await;
    let live = Live::default();
    let schema = schema_at(&pool, now(), &live);
    // No identity, and an identity that expired before the schema's clock.
    for request in [Request::new(SUBSCRIPTION), signed_in_until(SUBSCRIPTION, -1)] {
        let mut stream = Box::pin(schema.execute_stream(request));
        let first = tokio::time::timeout(Duration::from_secs(5), stream.next())
            .await
            .expect("the subscription ran without an operator")
            .expect("one response");
        assert_eq!(error_json(&first)["extensions"]["code"], "UNAUTHENTICATED");
        started(&schema, PVE_01).await;
        assert!(stream.next().await.is_none(), "the stream ended unsubscribed");
    }
}
