//! `createCheck` turns on the channels marked "on by default" in its own transaction, and
//! `Check.notifications` answers through the hub's schema from a loader that reads a batch of
//! checks in three statements (NOT-003, API-005).

use std::sync::atomic::Ordering;

use anima_hub::{checks::graphql::check_notifications_loader, ids::CheckId};
use serde_json::{Value, json};
use sqlx::PgPool;

use super::heartbeats::{Rig, rig};
use crate::devices::batching::counting_pool;

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
    fixtures(path = "../../../fixtures", scripts("devices"))
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
    fixtures(path = "../../../fixtures", scripts("devices"))
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
    fixtures(path = "../../../fixtures", scripts("devices"))
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

const CREATE: &str = "mutation($input: CreateCheckInput!) { createCheck(input: $input) { id } }";

/// Makes every insert into `table` fail with an SQL error.
async fn refuse_inserts(pool: &PgPool, table: &str) {
    for statement in [
        "CREATE FUNCTION public.refuse_insert() RETURNS trigger LANGUAGE plpgsql AS \
         $$ BEGIN RAISE EXCEPTION 'refused'; END $$"
            .to_owned(),
        format!(
            "CREATE TRIGGER refuse_insert BEFORE INSERT ON {table} \
             FOR EACH ROW EXECUTE FUNCTION public.refuse_insert()"
        ),
    ] {
        sqlx::query(&statement).execute(pool).await.unwrap();
    }
}

/// Creates the check `SSH` on pve-01, which must fail; returns how many checks and channel
/// links are stored afterwards.
async fn failed_create(rig: &Rig) -> (i64, i64) {
    let input = json!({ "deviceId": PVE_01, "name": "SSH", "config": { "tcp": { "port": 22 } } });
    let response = rig.run(CREATE, json!({ "input": input })).await;
    assert_eq!(response.errors.len(), 1, "{:?}", response.errors);
    let checks: i64 = sqlx::query_scalar("SELECT count(*) FROM checks.checks WHERE name = 'SSH'")
        .fetch_one(&rig.pool)
        .await
        .unwrap();
    let links: i64 = sqlx::query_scalar("SELECT count(*) FROM notifications.check_channels")
        .fetch_one(&rig.pool)
        .await
        .unwrap();
    (checks, links)
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn not_003_a_create_whose_audit_cannot_be_written_keeps_no_check_and_no_link(pool: PgPool) {
    let rig = rig(&pool).await;
    rig.channel("Pager", true).await;
    refuse_inserts(&pool, "events.audit_log").await;

    assert_eq!(failed_create(&rig).await, (0, 0));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn not_003_a_create_whose_channel_links_cannot_be_written_keeps_no_check(pool: PgPool) {
    let rig = rig(&pool).await;
    rig.channel("Pager", true).await;
    refuse_inserts(&pool, "notifications.check_channels").await;

    assert_eq!(failed_create(&rig).await, (0, 0));
}

/// Twelve TCP checks on pve-01, created while `Pager` is on by default.
async fn twelve_checks(rig: &Rig) -> Vec<String> {
    let mut checks = Vec::new();
    for port in 0..12 {
        let config = json!({ "tcp": { "port": 2000 + port } });
        checks.push(rig.check(PVE_01, &format!("Port {port}"), config).await);
    }
    checks
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn api_005_the_loader_reads_1_or_12_checks_in_the_same_3_statements(pool: PgPool) {
    let rig = rig(&pool).await;
    let pager = rig.channel("Pager", true).await;
    let checks: Vec<CheckId> = twelve_checks(&rig)
        .await
        .iter()
        .map(|id| id.parse().unwrap())
        .collect();
    let (counted, statements) = counting_pool(&pool).await;
    sqlx::query("SELECT 1").execute(&counted).await.unwrap();
    let loader = check_notifications_loader(counted);

    statements.store(0, Ordering::SeqCst);
    let one = loader.load_one(checks[0]).await.unwrap().unwrap();
    let for_one = statements.swap(0, Ordering::SeqCst);
    let all = loader.load_many(checks.iter().copied()).await.unwrap();
    let for_twelve = statements.load(Ordering::SeqCst);

    // `for_checks` reads the links, the settings and the channels once per batch. Read per
    // check, twelve checks would take 36.
    assert_eq!((for_one, for_twelve), (3, 3));
    assert_eq!(one.channels.len(), 1);
    assert_eq!(all.len(), 12);
    for found in all.values() {
        assert_eq!(found.channels[0].id.to_string(), pager);
    }
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn not_003_check_notifications_answers_for_each_of_many_checks_in_one_query(pool: PgPool) {
    let rig = rig(&pool).await;
    rig.channel("Pager", true).await;
    let checks = twelve_checks(&rig).await;
    let fields: Vec<String> = checks
        .iter()
        .enumerate()
        .map(|(index, id)| format!("c{index}: check(id: \"{id}\") {{ {NOTIFICATIONS} }}"))
        .collect();

    let read = rig
        .data(&format!("query {{ {} }}", fields.join(" ")), json!({}))
        .await;

    for index in 0..checks.len() {
        assert_eq!(
            names(&read[format!("c{index}")]["notifications"]),
            ["Pager"]
        );
    }
}
