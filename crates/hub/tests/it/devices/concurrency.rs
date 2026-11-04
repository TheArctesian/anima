//! Credential writes and device deletes on one device, at the same time, take their locks in
//! one order, so neither fails with a deadlock (DEV-012, DEV-015).

use std::time::Duration;

use serde_json::{Value, json};
use sqlx::PgPool;

use super::{
    credential::{PVE_01, SET, store_password},
    harness::{schema_for, with_variables},
};

/// Waits until `count` sessions of this database wait for a lock.
async fn lock_waiters(pool: &PgPool, count: i64) {
    for _ in 0..250 {
        let waiting: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM pg_stat_activity \
             WHERE wait_event_type = 'Lock' AND datname = current_database()",
        )
        .fetch_one(pool)
        .await
        .unwrap();
        if waiting >= count {
            return;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("never saw {count} sessions waiting for a lock");
}

fn codes(response: &async_graphql::Response) -> Vec<Value> {
    response
        .errors
        .iter()
        .map(|error| serde_json::to_value(error).unwrap()["extensions"]["code"].clone())
        .collect()
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn dev_015_a_set_credential_racing_a_device_delete_is_not_found_not_a_deadlock(pool: PgPool) {
    store_password(&pool, PVE_01, "an-earlier-password").await;
    let schema = schema_for(&pool);
    // A third session holds the device's SSH settings row, so the delete stops inside its
    // cascade after it has locked the device row.
    let mut blocker = pool.begin().await.unwrap();
    sqlx::query("SELECT 1 FROM devices.ssh_settings WHERE device_id = $1::text::uuid FOR UPDATE")
        .bind(PVE_01)
        .execute(&mut *blocker)
        .await
        .unwrap();
    let delete = tokio::spawn({
        let schema = schema.clone();
        async move {
            let query = "mutation($id: ID!) { deleteDevice(id: $id) }";
            with_variables(&schema, query, json!({ "id": PVE_01 })).await
        }
    });
    lock_waiters(&pool, 1).await;
    let set = tokio::spawn({
        let schema = schema.clone();
        async move {
            let credential = json!({ "password": "a-new-password" });
            with_variables(
                &schema,
                SET,
                json!({ "id": PVE_01, "credential": credential }),
            )
            .await
        }
    });
    lock_waiters(&pool, 2).await;
    blocker.rollback().await.unwrap();
    let (delete, set) = (delete.await.unwrap(), set.await.unwrap());

    assert_eq!(codes(&delete), Vec::<Value>::new());
    assert_eq!(
        delete.data.into_json().unwrap(),
        json!({ "deleteDevice": PVE_01 })
    );
    assert_eq!(codes(&set), [json!("NOT_FOUND")]);
    let secrets: i64 = sqlx::query_scalar("SELECT count(*) FROM credentials.secrets")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(secrets, 0);
}
