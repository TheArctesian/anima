//! Sync runs: the run row (SRC-005), the slice that moves only with its stored page (SRC-007),
//! a rerun that writes nothing (SRC-012), a superseded claim that writes nothing, and a consumer
//! switched off that stops polling (EXT-002).

use anima_hub::engine::sources::{Outcome, start_due};
use chrono::TimeDelta;
use serde_json::json;
use sqlx::PgPool;
use tokio::task::JoinSet;

use super::{items, kit, last_run, set_projects, slice};
use crate::hub::{error, start};

const PAGES: &[&[(&str, &str)]] = &[&[("a", "1"), ("b", "1")], &[("c", "1")], &[("d", "1")]];

#[sqlx::test(migrations = "./migrations")]
async fn src_005_a_run_row_records_its_trigger_times_and_counts(pool: PgPool) {
    let kit = kit(&pool).await;
    let (_, id) = kit.connect().await;
    set_projects(&pool, true).await;
    kit.stub.set_pages(PAGES);
    assert_eq!(kit.run(id).await, Some(Outcome::Ok));
    assert_eq!(
        last_run(&pool, id).await,
        ("manual".to_owned(), Some("ok".to_owned()), 4, 0, 0, 0, None)
    );
    let query = "query { sourceAccounts { health lastSyncAt failureCount \
        runs(first: 1) { trigger startedAt endedAt outcome inserted changed unchanged removed problems { code } } } }";
    let response = kit.hub.post(Some(&kit.token), query, json!({})).await;
    let account = &response["data"]["sourceAccounts"][0];
    assert_eq!(account["health"], "OK", "{response}");
    assert_eq!(account["failureCount"], 0);
    let at = json!(start().to_rfc3339());
    assert_eq!(
        account["runs"][0],
        json!({ "trigger": "MANUAL", "startedAt": at, "endedAt": at, "outcome": "OK",
            "inserted": 4, "changed": 0, "unchanged": 0, "removed": 0, "problems": [] })
    );
    assert_eq!(account["lastSyncAt"], at);
}

#[sqlx::test(migrations = "./migrations")]
async fn src_012_a_rerun_over_the_same_records_writes_nothing(pool: PgPool) {
    let kit = kit(&pool).await;
    let (_, id) = kit.connect().await;
    set_projects(&pool, true).await;
    kit.stub.set_pages(PAGES);
    kit.run(id).await;
    let first = items(&pool, id).await;
    assert_eq!(kit.run(id).await, Some(Outcome::Ok));
    assert_eq!(items(&pool, id).await, first, "no row written again");
    assert!(first.iter().all(|(_, _, writes)| *writes == 1));
    assert_eq!(
        last_run(&pool, id).await,
        ("manual".to_owned(), Some("ok".to_owned()), 0, 0, 4, 0, None)
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn src_007_a_page_whose_records_fail_to_store_does_not_move_the_slice(pool: PgPool) {
    let kit = kit(&pool).await;
    let (_, id) = kit.connect().await;
    set_projects(&pool, true).await;
    kit.stub.set_pages(PAGES);
    kit.intake.quirks.lock().unwrap().fail_after_storing = Some(2);
    assert_eq!(kit.run(id).await, Some(Outcome::Failed));
    assert_eq!(
        slice(&pool, id).await,
        json!({ "v": 1, "page": 2, "done": false })
    );
    let stored: Vec<String> = items(&pool, id)
        .await
        .into_iter()
        .map(|(id, _, _)| id)
        .collect();
    assert_eq!(stored, ["a", "b"], "page 2 rolled back with its slice");
    assert_eq!(
        last_run(&pool, id).await.6.as_deref(),
        Some("The hub could not store what it read.")
    );

    assert_eq!(kit.run(id).await, Some(Outcome::Ok));
    assert_eq!(
        kit.stub.gets(),
        ["/pages/1", "/pages/2", "/pages/2", "/pages/3"]
    );
    let stored = items(&pool, id).await;
    let ids: Vec<&str> = stored.iter().map(|(id, _, _)| id.as_str()).collect();
    assert_eq!(ids, ["a", "b", "c", "d"]);
    assert!(
        stored.iter().all(|(_, _, writes)| *writes == 1),
        "{stored:?}"
    );
    assert_eq!(
        slice(&pool, id).await,
        json!({ "v": 1, "page": 3, "done": true })
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn src_007_a_run_whose_claim_was_taken_writes_nothing(pool: PgPool) {
    let kit = kit(&pool).await;
    let (_, id) = kit.connect().await;
    set_projects(&pool, true).await;
    kit.stub.set_pages(PAGES);
    kit.stub.hook(
        "/pages/1",
        "UPDATE sources.accounts SET run_generation = run_generation + 1",
    );
    assert_eq!(kit.run(id).await, None);
    assert_eq!(items(&pool, id).await, []);
    assert_eq!(slice(&pool, id).await, serde_json::Value::Null);
    let (sync, outcome): (Option<chrono::DateTime<chrono::Utc>>, Option<String>) = sqlx::query_as(
        "SELECT a.last_sync_at, r.outcome FROM sources.accounts a JOIN sources.sync_runs r ON r.account_id = a.id",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!((sync, outcome), (None, None), "the stale run ended nothing");
}

#[sqlx::test(migrations = "./migrations")]
async fn ext_002_with_the_consumer_off_no_request_leaves_the_hub(pool: PgPool) {
    let kit = kit(&pool).await;
    let (_, id) = kit.connect().await;
    kit.stub.set_pages(PAGES);
    let mints = kit.stub.paths();
    let mut running = JoinSet::new();
    let later = kit.at(start() + TimeDelta::days(1));
    assert_eq!(start_due(&later, &mut running).await.unwrap(), 0);
    let sync = "mutation($id: ID!) { syncSourceAccount(id: $id) { id } }";
    let response = kit
        .hub
        .post(Some(&kit.token), sync, json!({ "id": id }))
        .await;
    assert_eq!(error(&response).1, "CONFLICT", "{response}");
    assert_eq!(kit.stub.paths(), mints, "only the connect's proving mint");

    set_projects(&pool, true).await;
    assert_eq!(start_due(&later, &mut running).await.unwrap(), 1);
    while running.join_next().await.is_some() {}
    assert_eq!(kit.stub.gets(), ["/pages/1", "/pages/2", "/pages/3"]);
}

#[sqlx::test(migrations = "./migrations")]
async fn ext_002_switching_the_consumer_off_mid_run_stops_it_and_on_resumes_it(pool: PgPool) {
    let kit = kit(&pool).await;
    let (_, id) = kit.connect().await;
    set_projects(&pool, true).await;
    kit.stub.set_pages(PAGES);
    kit.stub.hook(
        "/pages/2",
        "UPDATE extensions.switches SET is_on = false WHERE key = 'projects'",
    );
    assert_eq!(kit.run(id).await, Some(Outcome::Stopped));
    assert_eq!(
        kit.stub.gets(),
        ["/pages/1", "/pages/2"],
        "no request after the switch"
    );
    assert_eq!(
        slice(&pool, id).await,
        json!({ "v": 1, "page": 3, "done": false })
    );
    assert_eq!(last_run(&pool, id).await.1.as_deref(), Some("stopped"));

    set_projects(&pool, true).await;
    assert_eq!(kit.run(id).await, Some(Outcome::Ok));
    assert_eq!(kit.stub.gets(), ["/pages/1", "/pages/2", "/pages/3"]);
    assert_eq!(items(&pool, id).await.len(), 4);
}

#[sqlx::test(migrations = "./migrations")]
async fn src_007_a_run_sends_at_most_two_thousand_requests_and_the_next_resumes(pool: PgPool) {
    let kit = kit(&pool).await;
    let (_, id) = kit.connect().await;
    set_projects(&pool, true).await;
    let empty: Vec<&[(&str, &str)]> = vec![&[]; 2001];
    kit.stub.set_pages(&empty);

    assert_eq!(kit.run(id).await, Some(Outcome::Stopped));
    assert_eq!(kit.stub.gets().len(), 2000, "the cap");
    assert_eq!(
        slice(&pool, id).await,
        json!({ "v": 1, "page": 2001, "done": false })
    );

    assert_eq!(kit.run(id).await, Some(Outcome::Ok));
    assert_eq!(
        kit.stub.gets().last().map(String::as_str),
        Some("/pages/2001")
    );
}
