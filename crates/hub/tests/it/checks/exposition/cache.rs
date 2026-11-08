//! The ten-second cache on the injected clock, and a failed build kept two seconds (D-24).

use axum::http::StatusCode;
use chrono::TimeDelta;
use sqlx::PgPool;

use super::{PVE_01_PING, body, database_now, of_check, rig, scrape, set_statuses};

macro_rules! fixture_test {
    ($(async fn $name:ident($pool:ident: PgPool) $body:block)+) => {$(
        #[sqlx::test(
            migrations = "./migrations",
            fixtures(path = "../../../fixtures", scripts("devices", "checks"))
        )]
        async fn $name($pool: PgPool) $body
    )+};
}

async fn set_config(pool: &PgPool, config: &str) {
    sqlx::query("UPDATE checks.checks SET config = $2::jsonb WHERE id = $1::uuid")
        .bind(PVE_01_PING)
        .bind(config)
        .execute(pool)
        .await
        .unwrap();
}

fixture_test! {
    async fn d_24_the_same_body_within_ten_seconds_and_a_new_one_after(pool: PgPool) {
        let (router, clock) = rig(&pool, database_now(&pool).await);
        let first = body(&router).await;

        set_statuses(&pool, "up", &[(PVE_01_PING, "down")]).await;
        clock.advance(TimeDelta::milliseconds(9_999));
        let cached = body(&router).await;
        clock.advance(TimeDelta::milliseconds(1));
        let rebuilt = body(&router).await;

        assert_eq!(cached, first);
        assert_eq!(of_check(&first, "anima_check_status", PVE_01_PING).as_deref(), Some("4"));
        assert_eq!(of_check(&rebuilt, "anima_check_status", PVE_01_PING).as_deref(), Some("0"));
    }

    async fn d_24_a_failed_build_answers_500_with_a_fixed_body_for_two_seconds(pool: PgPool) {
        set_config(&pool, r#"{"unknownKey": "SELECT secret"}"#).await;
        let (router, clock) = rig(&pool, database_now(&pool).await);

        let failed = scrape(&router).await;
        set_config(&pool, r#"{"timeoutMs":2000,"ipVersion":"V4"}"#).await;
        clock.advance(TimeDelta::milliseconds(1_999));
        let kept = scrape(&router).await;
        clock.advance(TimeDelta::milliseconds(1));
        let rebuilt = scrape(&router).await;

        for answer in [&failed, &kept] {
            assert_eq!(answer.status, StatusCode::INTERNAL_SERVER_ERROR);
            assert_eq!(answer.body, "The metrics cannot be read now.");
        }
        assert_eq!(rebuilt.status, StatusCode::OK, "the failure is kept two seconds only");
        assert!(rebuilt.body.contains("anima_check_status{"));
    }
}
