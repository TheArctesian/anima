//! A device row held for another domain's transaction: kept, it cannot be deleted; locked, it
//! cannot be changed either; a missing device is `NotFound`.

use sqlx::{Connection, PgConnection, PgPool};

use super::*;

const PVE_01: &str = "0190a000-0000-7000-8002-000000000001";

/// Whether `sql` on the device runs on another connection while the row is held, waiting at
/// most a moment for the lock.
async fn runs_beside(pool: &PgPool, sql: &str) -> bool {
    let mut other = pool.acquire().await.unwrap();
    let mut tx = other.begin().await.unwrap();
    sqlx::query("SET LOCAL lock_timeout = '200ms'")
        .execute(&mut *tx)
        .await
        .unwrap();
    let ran = sqlx::query(sql)
        .bind(PVE_01)
        .execute(&mut *tx)
        .await
        .is_ok();
    tx.rollback().await.ok();
    ran
}

async fn held(pool: &PgPool, hold_as: Hold) -> sqlx::Transaction<'static, sqlx::Postgres> {
    let mut tx = pool.begin().await.unwrap();
    let conn: &mut PgConnection = &mut tx;
    hold(conn, device_id(PVE_01), hold_as).await.unwrap();
    tx
}

const CHANGE: &str =
    "UPDATE devices.devices SET maintenance = NOT maintenance WHERE id = $1::text::uuid";
const DELETE: &str = "DELETE FROM devices.devices WHERE id = $1::text::uuid";

store_test! {
    async fn met_011_a_kept_device_may_change_but_not_be_deleted(pool: PgPool) {
        let tx = held(&pool, Hold::Kept).await;
        assert!(runs_beside(&pool, CHANGE).await);
        assert!(!runs_beside(&pool, DELETE).await);
        tx.rollback().await.unwrap();
        assert!(runs_beside(&pool, DELETE).await);
    }

    async fn met_013_a_locked_device_may_neither_change_nor_be_deleted(pool: PgPool) {
        let tx = held(&pool, Hold::Locked).await;
        assert!(!runs_beside(&pool, CHANGE).await);
        assert!(!runs_beside(&pool, DELETE).await);
        tx.rollback().await.unwrap();
        assert!(runs_beside(&pool, CHANGE).await);
    }

    async fn met_011_holding_a_missing_device_is_not_found(pool: PgPool) {
        let missing = device_id("0190a000-0000-7000-8002-0000000000ff");
        let mut conn = pool.acquire().await.unwrap();
        for hold_as in [Hold::Kept, Hold::Locked] {
            let error = hold(&mut conn, missing, hold_as).await.unwrap_err();
            assert!(matches!(error, Error::NotFound(Missing::Device(id)) if id == missing));
        }
    }
}
