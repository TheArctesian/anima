//! `createCheck` turns on the channels marked "on by default", and `Check.notifications`
//! answers through the hub's schema in a bounded number of statements however many checks a
//! query selects (NOT-003, API-005).

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use serde_json::{Value, json};
use sqlx::PgPool;
use tracing::{Event, Subscriber, subscriber::set_default};
use tracing_subscriber::{Layer, layer::Context, layer::SubscriberExt as _, registry::Registry};

use super::heartbeats::{Rig, rig};

const PVE_01: &str = "0190a000-0000-7000-8002-000000000001";

const NOTIFICATIONS: &str = "notifications { resendEveryBeats channels { id name } }";

/// The names of the channels a `CheckNotifications` lists.
fn names(notifications: &Value) -> Vec<&str> {
    notifications["channels"]
        .as_array()
        .unwrap()
        .iter()
        .map(|channel| channel["name"].as_str().unwrap())
        .collect()
}

async fn notifications_of(rig: &Rig, check: &str) -> Value {
    let query = format!("query($id: ID!) {{ check(id: $id) {{ {NOTIFICATIONS} }} }}");
    let read = rig.data(&query, json!({ "id": check })).await;
    read["check"]["notifications"].clone()
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn not_003_a_new_check_notifies_the_channels_on_by_default(pool: PgPool) {
    let rig = rig(&pool).await;
    rig.channel("Pager", true).await;
    rig.channel("Quiet", false).await;
    rig.channel("Phone", true).await;

    let id = rig
        .check(PVE_01, "SSH", json!({ "tcp": { "port": 22 } }))
        .await;

    let notifications = notifications_of(&rig, &id).await;
    assert_eq!(names(&notifications), ["Pager", "Phone"]);
    assert_eq!(notifications["resendEveryBeats"], 0);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn not_003_with_no_channel_on_by_default_a_new_check_notifies_none(pool: PgPool) {
    let rig = rig(&pool).await;
    rig.channel("Quiet", false).await;

    let id = rig
        .check(PVE_01, "SSH", json!({ "tcp": { "port": 22 } }))
        .await;

    let notifications = notifications_of(&rig, &id).await;
    assert_eq!(notifications["channels"], json!([]));
    let links: i64 = sqlx::query_scalar("SELECT count(*) FROM notifications.check_channels")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(links, 0);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn not_003_check_notifications_answers_the_choices_set_for_the_check(pool: PgPool) {
    let rig = rig(&pool).await;
    let pager = rig.channel("Pager", false).await;
    let id = rig
        .check(PVE_01, "SSH", json!({ "tcp": { "port": 22 } }))
        .await;
    let set = "mutation($check: ID!, $input: CheckNotificationsInput!) { \
        setCheckNotifications(checkId: $check, input: $input) { resendEveryBeats } }";
    let input = json!({ "channelIds": [pager], "resendEveryBeats": 3 });
    rig.data(set, json!({ "check": id, "input": input })).await;

    let notifications = notifications_of(&rig, &id).await;

    assert_eq!(
        notifications["channels"],
        json!([{ "id": pager, "name": "Pager" }])
    );
    assert_eq!(notifications["resendEveryBeats"], 3);
}

/// Counts the statements sqlx runs on this thread.
struct Statements(Arc<AtomicUsize>);

impl<S: Subscriber> Layer<S> for Statements {
    fn on_event(&self, event: &Event<'_>, _: Context<'_, S>) {
        if event.metadata().target() == "sqlx::query" {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }
}

/// The statements one query reading `selection` on each of `checks` takes.
async fn statements(rig: &Rig, checks: &[String], selection: &str) -> usize {
    let fields: Vec<String> = checks
        .iter()
        .enumerate()
        .map(|(index, id)| format!("c{index}: check(id: \"{id}\") {{ id {selection} }}"))
        .collect();
    let query = format!("query {{ {} }}", fields.join(" "));
    let count = Arc::new(AtomicUsize::new(0));
    let guard = set_default(Registry::default().with(Statements(count.clone())));
    tracing::callsite::rebuild_interest_cache();
    let read = rig.data(&query, json!({})).await;
    drop(guard);
    assert_eq!(read.as_object().unwrap().len(), checks.len());
    count.load(Ordering::SeqCst)
}

/// What `Check.notifications` adds to a query of `checks`.
async fn notifications_cost(rig: &Rig, checks: &[String]) -> usize {
    let with = statements(rig, checks, NOTIFICATIONS).await;
    let without = statements(rig, checks, "").await;
    with.saturating_sub(without)
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn api_005_check_notifications_cost_the_same_statements_for_1_or_12_checks(pool: PgPool) {
    let rig = rig(&pool).await;
    let pager = rig.channel("Pager", true).await;
    let mut checks = Vec::new();
    for port in 0..12 {
        let config = json!({ "tcp": { "port": 2000 + port } });
        checks.push(rig.check(PVE_01, &format!("Port {port}"), config).await);
    }

    let one = notifications_cost(&rig, &checks[..1]).await;
    let twelve = notifications_cost(&rig, &checks).await;

    // `for_checks` reads the links, the settings and the channels: three statements per batch.
    // One per check would be 36 for twelve.
    assert!(one <= 3, "1 check took {one} statements");
    assert!(twelve <= 3, "12 checks took {twelve} statements");
    let last = notifications_of(&rig, &checks[11]).await;
    assert_eq!(last["channels"][0]["id"], json!(pager));
}
