//! `startSession` and the device's session limit (RA-036, K4 §1 step 1): a device with as many
//! live sessions as its limit refuses the next start with CONFLICT until one ends, two starts
//! at the limit cannot both pass, files tickets hold no session, and a device with
//! no limit, or another device, is never refused. A refused start tries no wake, an expired
//! unredeemed ticket frees its slot, and a limit lowered below the live count refuses.

use std::sync::Arc;

use anima_hub::{
    clock::{Clock, FixedClock},
    events::Bus,
    sessions::{
        FilesMutation, Live,
        graphql::{Mutation as SessionsMutation, Query, Subscription},
    },
};
use async_graphql::{MergedObject, Request, Response, Schema, Variables};
use chrono::TimeDelta;
use serde_json::{Value, json};
use sqlx::{PgPool, postgres::PgPoolOptions};

use crate::sessions::{
    DOCKER_HOST, PVE_01,
    harness::{TestSchema, error_json, schema_at, signed_in_until},
    now,
    starting::{
        display, end_request, event_count, ready, redeem, session_count, start_request, started,
    },
};

const LIMIT_REACHED: &str = "This device has reached its session limit.";

/// Sets `device`'s session limit; `None` is no limit.
async fn set_limit(pool: &PgPool, device: &str, limit: Option<i32>) {
    sqlx::query("update devices.devices set session_limit = $2 where id = $1::text::uuid")
        .bind(device)
        .bind(limit)
        .execute(pool)
        .await
        .unwrap();
}

/// Asserts the response is the limit's refusal: CONFLICT with K4's text, nothing else.
fn assert_limit_reached(response: &Response) {
    let error = error_json(response);
    assert_eq!(error["extensions"]["code"], "CONFLICT", "{error}");
    assert_eq!(error["message"], LIMIT_REACHED, "{error}");
}

/// The response to one `startSession` for `device`.
async fn start(schema: &TestSchema, device: &str) -> Response {
    schema.execute(start_request(device, &display())).await
}

/// Ends the session of `ticket` with `endSession`.
async fn end(schema: &TestSchema, ticket: &Value) {
    let id = ticket["sessionId"].as_str().unwrap();
    let response = schema.execute(end_request(id)).await;
    assert!(response.errors.is_empty(), "{:?}", response.errors);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn ra_036_a_device_at_its_limit_refuses_the_next_start_until_one_ends(pool: PgPool) {
    ready(&pool, &[PVE_01]).await;
    set_limit(&pool, PVE_01, Some(3)).await;
    let schema = schema_at(&pool, now(), &Live::default());
    let mut tickets = Vec::new();
    for _ in 0..3 {
        tickets.push(started(&schema, PVE_01).await);
    }

    assert_limit_reached(&start(&schema, PVE_01).await);
    assert_eq!(session_count(&pool).await, 3, "the refusal wrote no row");

    end(&schema, &tickets[0]).await;
    started(&schema, PVE_01).await;
    assert_limit_reached(&start(&schema, PVE_01).await);
    assert_eq!(session_count(&pool).await, 4);
}

/// A pool of its own with room for both starts' transactions at once.
async fn own_pool(pool: &PgPool) -> PgPool {
    PgPoolOptions::new()
        .max_connections(4)
        .connect_with((*pool.connect_options()).clone())
        .await
        .unwrap()
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn ra_036_two_parallel_starts_at_a_limit_of_one_give_exactly_one_ticket(pool: PgPool) {
    ready(&pool, &[PVE_01]).await;
    set_limit(&pool, PVE_01, Some(1)).await;
    let own = own_pool(&pool).await;
    let schema = schema_at(&own, now(), &Live::default());
    for round in 0..8 {
        let (first, second) = tokio::join!(start(&schema, PVE_01), start(&schema, PVE_01));
        let (won, lost) = match (first.errors.is_empty(), second.errors.is_empty()) {
            (true, false) => (first, second),
            (false, true) => (second, first),
            (first_ok, second_ok) => panic!("round {round}: {first_ok} and {second_ok}"),
        };
        assert_limit_reached(&lost);
        assert_eq!(session_count(&pool).await, round + 1, "round {round}");
        let ticket = won.data.into_json().unwrap()["startSession"].clone();
        end(&schema, &ticket).await;
    }
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn ra_036_a_device_with_no_limit_is_never_refused(pool: PgPool) {
    ready(&pool, &[PVE_01]).await;
    let schema = schema_at(&pool, now(), &Live::default());
    for _ in 0..12 {
        started(&schema, PVE_01).await;
    }

    set_limit(&pool, PVE_01, Some(12)).await;
    assert_limit_reached(&start(&schema, PVE_01).await);
    set_limit(&pool, PVE_01, None).await;
    started(&schema, PVE_01).await;
    assert_eq!(session_count(&pool).await, 13);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn ra_036_a_limit_on_one_device_leaves_another_device_alone(pool: PgPool) {
    ready(&pool, &[PVE_01, DOCKER_HOST]).await;
    set_limit(&pool, PVE_01, Some(1)).await;
    set_limit(&pool, DOCKER_HOST, Some(2)).await;
    let schema = schema_at(&pool, now(), &Live::default());
    started(&schema, PVE_01).await;
    assert_limit_reached(&start(&schema, PVE_01).await);

    started(&schema, DOCKER_HOST).await;
    started(&schema, DOCKER_HOST).await;
    assert_limit_reached(&start(&schema, DOCKER_HOST).await);
    assert_limit_reached(&start(&schema, PVE_01).await);
}

/// The sessions mutations with `openFiles`, merged as `schema.rs` merges them.
#[derive(MergedObject, Default)]
struct Mutation(SessionsMutation, FilesMutation);

type FilesSchema = Schema<Query, Mutation, Subscription>;

fn files_schema(pool: &PgPool, live: &Live) -> FilesSchema {
    let clock: Arc<dyn Clock> = Arc::new(FixedClock(now()));
    Schema::build(Query, Mutation::default(), Subscription)
        .data(pool.clone())
        .data(clock)
        .data(live.clone())
        .data(Bus::new(16))
        .finish()
}

/// The data of `query` with `variables`, which must have no errors.
async fn data(schema: &FilesSchema, query: &str, variables: Value) -> Value {
    let request: Request = signed_in_until(query, 24).variables(Variables::from_json(variables));
    let response = schema.execute(request).await;
    assert!(response.errors.is_empty(), "{:?}", response.errors);
    response.data.into_json().unwrap()
}

const OPEN_FILES: &str = "mutation($id: ID!) { openFiles(sessionId: $id) { tunnelPath } }";

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn ra_036_files_tickets_hold_no_session_and_never_count(pool: PgPool) {
    ready(&pool, &[PVE_01]).await;
    set_limit(&pool, PVE_01, Some(2)).await;
    let live = Live::default();
    let schema = schema_at(&pool, now(), &live);
    let files_schema = files_schema(&pool, &live);
    let owner = started(&schema, PVE_01).await;
    // The owner's terminal tunnel is open while it is held, as the tunnel task holds it.
    let _tunnel = redeem(&live, &owner, now()).expect("the ticket opens the tunnel");
    let id = json!({ "id": owner["sessionId"] });
    for _ in 0..3 {
        let files = data(&files_schema, OPEN_FILES, id.clone()).await;
        assert_eq!(files["openFiles"]["tunnelPath"], "/files");
    }
    assert_eq!(session_count(&pool).await, 1, "only the owner's row");

    started(&schema, PVE_01).await;
    assert_limit_reached(&start(&schema, PVE_01).await);
}

/// PVE-01 Down, with a MAC address and DOCKER-HOST (no accepted key) as its site's wake relay:
/// each wake tried writes one Error event.
async fn asleep_with_relay(pool: &PgPool) {
    for statement in [
        "update devices.devices set mac_address = 'aa:bb:cc:dd:ee:ff' where id = $1::text::uuid",
        "insert into checks.checks (id, device_id, type, name, config, status) \
         values (gen_random_uuid(), $1::text::uuid, 'icmp', 'Ping', \
                 '{\"timeoutMs\":2000,\"ipVersion\":\"V4\"}', 'down')",
    ] {
        sqlx::query(statement)
            .bind(PVE_01)
            .execute(pool)
            .await
            .unwrap();
    }
    sqlx::query(
        "update devices.sites set wake_relay_device_id = $2::text::uuid \
         where id = (select site_id from devices.devices where id = $1::text::uuid)",
    )
    .bind(PVE_01)
    .bind(DOCKER_HOST)
    .execute(pool)
    .await
    .unwrap();
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn ra_036_a_start_refused_at_the_limit_tries_no_wake(pool: PgPool) {
    ready(&pool, &[PVE_01]).await;
    asleep_with_relay(&pool).await;
    set_limit(&pool, PVE_01, Some(1)).await;
    let schema = schema_at(&pool, now(), &Live::default());
    started(&schema, PVE_01).await;
    assert_eq!(
        event_count(&pool).await,
        1,
        "the first start tried the wake"
    );

    assert_limit_reached(&start(&schema, PVE_01).await);
    assert_eq!(
        event_count(&pool).await,
        1,
        "the refused start tried no wake"
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn ra_036_an_expired_unredeemed_ticket_frees_its_slot(pool: PgPool) {
    ready(&pool, &[PVE_01]).await;
    set_limit(&pool, PVE_01, Some(1)).await;
    let live = Live::default();
    started(&schema_at(&pool, now(), &live), PVE_01).await;

    let after_expiry = schema_at(&pool, now() + TimeDelta::seconds(31), &live);
    started(&after_expiry, PVE_01).await;
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn ra_036_a_limit_lowered_below_the_live_count_refuses_the_next_start(pool: PgPool) {
    ready(&pool, &[PVE_01]).await;
    let schema = schema_at(&pool, now(), &Live::default());
    for _ in 0..3 {
        started(&schema, PVE_01).await;
    }

    set_limit(&pool, PVE_01, Some(2)).await;
    assert_limit_reached(&start(&schema, PVE_01).await);
}
