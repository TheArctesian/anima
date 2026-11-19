//! A place with references is not deleted without a replacement (PLC-005).

use serde_json::json;
use sqlx::PgPool;

use super::{add, child, id, link, place, run, signed_in, top};
use crate::hub::error;

const DELETE: &str = "mutation($id: ID!, $r: ID) { deletePlace(id: $id, replacement: $r) }";

#[sqlx::test(migrations = "./migrations")]
async fn plc_005_a_place_with_no_reference_is_deleted(pool: PgPool) {
    let (hub, token) = signed_in(&pool).await;
    let home = id(&add(&hub, &token, top("Home")).await);
    let done = run(&hub, &token, DELETE, json!({ "id": home })).await;
    assert_eq!(done["data"]["deletePlace"], json!(home), "{done}");
    assert_eq!(place(&hub, &token, &home).await, json!(null));
}

#[sqlx::test(migrations = "./migrations")]
async fn plc_005_a_place_with_references_is_refused_with_the_count_of_each_sort(pool: PgPool) {
    let (hub, token) = signed_in(&pool).await;
    let home = id(&add(&hub, &token, top("Home")).await);
    add(&hub, &token, child("Room", &home)).await;
    add(&hub, &token, child("Attic", &home)).await;
    link(&pool, "book:1", &format!("place:{home}"), "stored_at").await;
    let refused = run(&hub, &token, DELETE, json!({ "id": home })).await;
    assert_eq!(error(&refused).1, "CONFLICT");
    let counts = &refused["errors"][0]["extensions"]["references"];
    assert_eq!(counts["links"], 1, "{refused}");
    assert_eq!(counts["children"], 2);
    assert_eq!(counts["sites"], 0);
    assert_eq!(
        place(&hub, &token, &home).await["id"],
        json!(home),
        "nothing was deleted"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn plc_005_a_delete_with_a_replacement_merges_into_it(pool: PgPool) {
    let (hub, token) = signed_in(&pool).await;
    let old = id(&add(&hub, &token, top("Old")).await);
    let new = id(&add(&hub, &token, top("New")).await);
    link(&pool, "book:1", &format!("place:{old}"), "stored_at").await;
    let done = run(&hub, &token, DELETE, json!({ "id": old, "r": new })).await;
    assert!(done["errors"].is_null(), "{done}");
    let moved: i64 = sqlx::query_scalar("SELECT count(*) FROM links.record_link WHERE target = $1")
        .bind(format!("place:{new}"))
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(moved, 1);
    assert_eq!(place(&hub, &token, &old).await["id"], json!(new));
    let actions: Vec<String> = sqlx::query_scalar(
        "SELECT action FROM events.audit_log WHERE target_type = 'place' ORDER BY at, id",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(actions.last().map(String::as_str), Some("delete"));
}
