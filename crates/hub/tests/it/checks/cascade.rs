//! Deleting a device removes its checks and their aggregates, but heartbeats have no foreign
//! key and stay until the retention job removes them (DEV-015).

use sqlx::PgPool;
use uuid::Uuid;

use super::{DOCKER_HOST, PVE_01, UPTIME_TABLES, at, beat, count, ensure, id, insert_check};

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices", "checks"))
)]
async fn dev_015_deleting_a_device_removes_its_checks_and_uptime_but_keeps_heartbeats(
    pool: PgPool,
) {
    let (group, child) = (Uuid::now_v7(), Uuid::now_v7());
    insert_check(&pool, (group, PVE_01), ("group", "All"), None)
        .await
        .unwrap();
    insert_check(&pool, (child, PVE_01), ("tcp", "Web"), None)
        .await
        .unwrap();
    sqlx::query("update checks.checks set parent_id = $1 where id = $2")
        .bind(group)
        .bind(child)
        .execute(&pool)
        .await
        .unwrap();
    let mut conn = pool.acquire().await.unwrap();
    ensure(&mut conn, at("2030-01-01T00:00:00Z")).await.unwrap();
    beat(&mut conn, child, at("2030-01-01T00:00:00Z"))
        .await
        .unwrap();
    for table in UPTIME_TABLES {
        sqlx::query(&format!(
            "insert into checks.{table} (check_id, bucket, up_count) values ($1, $2, 1)"
        ))
        .bind(child)
        .bind(at("2030-01-01T00:00:00Z"))
        .execute(&pool)
        .await
        .unwrap();
    }
    let others = count(
        &pool,
        "select count(*) from checks.checks where device_id = $1",
        id(DOCKER_HOST),
    )
    .await;

    sqlx::query("delete from devices.devices where id = $1")
        .bind(id(PVE_01))
        .execute(&pool)
        .await
        .unwrap();

    let device_checks = "select count(*) from checks.checks where device_id = $1";
    assert_eq!(count(&pool, device_checks, id(PVE_01)).await, 0);
    assert_eq!(count(&pool, device_checks, id(DOCKER_HOST)).await, others);
    for table in UPTIME_TABLES {
        let sql = format!("select count(*) from checks.{table} where check_id = $1");
        assert_eq!(count(&pool, &sql, child).await, 0, "{table}");
    }
    let beats = "select count(*) from checks.heartbeats where check_id = $1";
    assert_eq!(count(&pool, beats, child).await, 1);
}
