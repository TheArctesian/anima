//! The distribution read from a device: only a short lower-case token is kept, it is stored
//! without touching what the operator set, and an equal value writes nothing (DEV-016).

use super::*;

/// The row's `updated_at` and the transaction id that last wrote it.
async fn written(pool: &sqlx::PgPool, id: &str) -> (chrono::DateTime<chrono::Utc>, String) {
    sqlx::query_as("SELECT updated_at, xmin::text FROM devices.devices WHERE id = $1::text::uuid")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap()
}

#[test]
fn dev_016_a_distribution_is_a_short_lower_case_token() {
    for id in [
        "pop",
        "ubuntu",
        "arch",
        "linuxmint",
        "opensuse-tumbleweed",
        "rhel_9.4",
    ] {
        assert_eq!(Distribution::new(id).unwrap().as_str(), id);
    }
    assert!(Distribution::new(&"a".repeat(32)).is_some());
    for junk in [
        "",
        "Ubuntu",
        "pop os",
        "\"pop\"",
        "pop;rm",
        "../etc",
        "pöp",
        "pop\n",
        &"a".repeat(33),
    ] {
        assert_eq!(Distribution::new(junk), None, "{junk:?}");
    }
}

store_test! {
    async fn dev_016_a_recorded_distribution_is_read_back_and_an_equal_one_writes_nothing(pool: PgPool) {
        let id = device_id(PVE_01);
        assert_eq!(get(&pool, id).await.unwrap().distribution, None, "null until read");
        let before = written(&pool, PVE_01).await;
        let pop = Distribution::new("pop").unwrap();
        record_distribution(&pool, id, &pop).await.unwrap();
        assert_eq!(get(&pool, id).await.unwrap().distribution, Some(pop.clone()));
        let after = written(&pool, PVE_01).await;
        assert_eq!(after.0, before.0, "the operator did not change the device");
        assert_ne!(after.1, before.1);
        record_distribution(&pool, id, &pop).await.unwrap();
        assert_eq!(written(&pool, PVE_01).await, after, "an equal value is not written");
        let arch = Distribution::new("arch").unwrap();
        record_distribution(&pool, id, &arch).await.unwrap();
        let listed = list(&pool).await.unwrap();
        let listed = listed.iter().find(|device| device.id == id).unwrap();
        assert_eq!(listed.distribution, Some(arch));
    }

    async fn dev_016_the_database_refuses_a_distribution_that_is_not_a_token(pool: PgPool) {
        let refused = sqlx::query(
            "UPDATE devices.devices SET distribution = 'Pop!_OS' WHERE id = $1::text::uuid",
        )
        .bind(PVE_01)
        .execute(&pool)
        .await
        .unwrap_err();
        let constraint = refused.as_database_error().and_then(|error| error.constraint());
        assert_eq!(constraint, Some("devices_distribution_check"));
    }
}
