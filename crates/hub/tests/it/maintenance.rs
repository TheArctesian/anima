mod active;
mod bounds;
mod guard;
mod harness;
mod k1;
mod read;
mod trail;
mod writes;

use serde_json::Value;
use sqlx::PgPool;

macro_rules! fixture_test {
    ($(async fn $name:ident($pool:ident: PgPool) $body:block)+) => {$(
        #[sqlx::test(
            migrations = "./migrations",
            fixtures(path = "../fixtures", scripts("maintenance"))
        )]
        async fn $name($pool: PgPool) $body
    )+};
}

const SUNDAY_UPDATES: &str = "0190a000-0000-7000-8004-000000000001";
const OVERNIGHT_BACKUP: &str = "0190a000-0000-7000-8004-000000000002";

fn phase1() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../contracts/fixtures/phase1.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn constraint_of(error: &sqlx::Error) -> Option<&str> {
    error.as_database_error().and_then(|e| e.constraint())
}

async fn insert_window(
    pool: &PgPool,
    name: &str,
    (kind, rule_kind): (&str, &str),
    active_manual: bool,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO maintenance.windows (id, name, kind, timezone, rule, active_manual) \
         VALUES (gen_random_uuid(), $1, $2, 'UTC', jsonb_build_object('kind', $3::text), $4)",
    )
    .bind(name)
    .bind(kind)
    .bind(rule_kind)
    .bind(active_manual)
    .execute(pool)
    .await
    .map(|_| ())
}

fixture_test! {
    async fn k6_maintenance_fixture_matches_phase1_json(pool: PgPool) {
        let json = phase1();
        let windows = json["maintenanceWindows"].as_array().unwrap();
        assert_eq!(windows.len(), 2);
        for window in windows {
            let id = window["id"].as_str().unwrap();
            let row: (String, String, String, Value) = sqlx::query_as(
                "SELECT name, kind, timezone, rule FROM maintenance.windows WHERE id = $1::text::uuid",
            )
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
            assert_eq!(row.0, window["name"].as_str().unwrap());
            assert_eq!(row.1, window["kind"].as_str().unwrap().to_lowercase());
            assert_eq!(row.2, window["timezone"].as_str().unwrap());
            assert_eq!(row.3["durationMinutes"], window["durationMinutes"]);
            assert_eq!(row.3["startTime"].as_str().unwrap()[..5], *window["startTime"].as_str().unwrap());
            let days: Vec<String> = window["weekdays"].as_array().unwrap().iter()
                .map(|d| d.as_str().unwrap().to_lowercase()).collect();
            assert_eq!(row.3["weekdays"], serde_json::json!(days));

            let targets: Vec<(String, String)> = sqlx::query_as(
                "SELECT target_type, target FROM maintenance.window_targets \
                 WHERE window_id = $1::text::uuid ORDER BY target",
            )
            .bind(id)
            .fetch_all(&pool)
            .await
            .unwrap();
            let expected: Vec<(String, String)> = window["targets"].as_array().unwrap().iter()
                .map(|t| (t["type"].as_str().unwrap().to_lowercase(), t["target"].as_str().unwrap().to_owned()))
                .collect();
            assert_eq!(targets, expected);
        }
    }

    async fn k2_a_duplicate_window_name_fails_on_the_named_constraint(pool: PgPool) {
        let error = insert_window(&pool, "Sunday updates", ("manual", "manual"), false).await.unwrap_err();
        assert_eq!(constraint_of(&error), Some("windows_name_key"));
    }

    async fn k2_an_unknown_kind_fails_on_the_named_check(pool: PgPool) {
        let error = insert_window(&pool, "x", ("cron", "cron"), false).await.unwrap_err();
        assert_eq!(constraint_of(&error), Some("windows_kind_check"));
    }

    async fn mw_004_the_rule_must_carry_the_kind_of_its_row(pool: PgPool) {
        let error = insert_window(&pool, "x", ("manual", "single"), false).await.unwrap_err();
        assert_eq!(constraint_of(&error), Some("windows_rule_kind_check"));
    }

    async fn mw_004_only_a_manual_window_can_be_switched_on(pool: PgPool) {
        let error = insert_window(&pool, "x", ("single", "single"), true).await.unwrap_err();
        assert_eq!(constraint_of(&error), Some("windows_active_manual_check"));
        insert_window(&pool, "y", ("manual", "manual"), true).await.unwrap();

        let error = sqlx::query(
            "UPDATE maintenance.windows SET active_manual = true WHERE id = $1::text::uuid",
        )
        .bind(SUNDAY_UPDATES)
        .execute(&pool)
        .await
        .unwrap_err();
        assert_eq!(constraint_of(&error), Some("windows_active_manual_check"));
    }

    async fn db_011_defaults_live_in_the_schema(pool: PgPool) {
        sqlx::query(
            "INSERT INTO maintenance.windows (id, name, kind, timezone, rule) \
             VALUES (gen_random_uuid(), 'defaults', 'manual', 'UTC', '{\"kind\": \"manual\"}')",
        )
        .execute(&pool)
        .await
        .unwrap();
        let row: (bool, bool, bool) = sqlx::query_as(
            "SELECT active_manual, created_at <= now(), updated_at <= now() \
             FROM maintenance.windows WHERE name = 'defaults'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(row, (false, true, true));
    }

    async fn mw_003_a_target_has_a_known_type_a_window_and_no_repeat(pool: PgPool) {
        let insert = |window: &'static str, target_type: &'static str, target: &'static str| {
            let pool = pool.clone();
            async move {
                sqlx::query(
                    "INSERT INTO maintenance.window_targets (window_id, target_type, target) \
                     VALUES ($1::text::uuid, $2, $3)",
                )
                .bind(window)
                .bind(target_type)
                .bind(target)
                .execute(&pool)
                .await
                .unwrap_err()
            }
        };
        let bad_type = insert(SUNDAY_UPDATES, "site", "home").await;
        assert_eq!(constraint_of(&bad_type), Some("window_targets_target_type_check"));
        let repeat = insert(SUNDAY_UPDATES, "tag", "infra").await;
        assert_eq!(constraint_of(&repeat), Some("window_targets_pkey"));
        let orphan = insert("0190a000-0000-7000-8004-0000000000ff", "tag", "infra").await;
        assert_eq!(constraint_of(&orphan), Some("window_targets_window_id_fkey"));
    }

    async fn mw_003_deleting_a_window_removes_its_targets_only(pool: PgPool) {
        let count = |window: &'static str| {
            let pool = pool.clone();
            async move {
                sqlx::query_scalar::<_, i64>(
                    "SELECT count(*) FROM maintenance.window_targets WHERE window_id = $1::text::uuid",
                )
                .bind(window)
                .fetch_one(&pool)
                .await
                .unwrap()
            }
        };
        assert_eq!((count(SUNDAY_UPDATES).await, count(OVERNIGHT_BACKUP).await), (1, 1));
        sqlx::query("DELETE FROM maintenance.windows WHERE id = $1::text::uuid")
            .bind(SUNDAY_UPDATES)
            .execute(&pool)
            .await
            .unwrap();
        assert_eq!((count(SUNDAY_UPDATES).await, count(OVERNIGHT_BACKUP).await), (0, 1));
    }
}
