//! The runtime role writes heartbeats and aggregates and manages partitions only through the
//! owner's functions (K2 "Roles", R-016, R-023).

use super::{PVE_01_PING, UPTIME_TABLES, as_runtime, at, beat, ensure, execute, id, refusal};
use sqlx::{PgConnection, PgPool};

/// Statements the runtime role must not be able to run. The partition is the one
/// `runtime_writes` creates.
const OWNER_ONLY: [&str; 6] = [
    "create table checks.heartbeats_20300102 partition of checks.heartbeats \
     for values from ('2030-01-02T00:00:00Z') to ('2030-01-03T00:00:00Z')",
    "drop table checks.heartbeats",
    "drop table checks.heartbeats_20300101",
    "alter table checks.heartbeats detach partition checks.heartbeats_20300101",
    "truncate checks.heartbeats",
    "create table checks.extra (id int)",
];

/// Temporary objects that shadow every type name the partition functions could resolve
/// through `pg_temp`; each domain's CHECK records that it ran as the function owner.
const SHADOWED_TYPES: [&str; 9] = [
    "date",
    "text",
    "timestamptz",
    "timestamp",
    "interval",
    "regclass",
    "int4",
    "bool",
    "name",
];

const FUNCTIONS: [&str; 2] = [
    "checks.drop_day_partitions_before(date,timestamp with time zone)",
    "checks.ensure_day_partition(timestamp with time zone)",
];

/// Creates a partition, writes a heartbeat and upserts every aggregate twice, then reads
/// them back; returns the heartbeat count and each aggregate's up count.
async fn runtime_writes(conn: &mut PgConnection) -> Result<(i64, Vec<i32>), sqlx::Error> {
    let (check, instant) = (id(PVE_01_PING), at("2030-01-01T06:00:00Z"));
    ensure(conn, instant).await?;
    beat(conn, check, instant).await?;
    let beats = sqlx::query_scalar("select count(*) from checks.heartbeats where check_id = $1")
        .bind(check)
        .fetch_one(&mut *conn)
        .await?;
    let mut ups = Vec::new();
    for table in UPTIME_TABLES {
        let upsert = format!(
            "insert into checks.{table} (check_id, bucket, up_count, response_count, \
             avg_response_ms) values ($1, $2, 1, 1, 12) on conflict (check_id, bucket) do update \
             set up_count = checks.{table}.up_count + 1 returning up_count"
        );
        for _ in 0..2 {
            let up = sqlx::query_scalar(&upsert)
                .bind(check)
                .bind(instant)
                .fetch_one(&mut *conn)
                .await?;
            ups.push(up);
        }
    }
    sqlx::query("select checks.drop_day_partitions_before('2000-01-01', now())")
        .execute(&mut *conn)
        .await?;
    Ok((beats, ups))
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices", "checks"))
)]
async fn r_016_runtime_role_writes_heartbeats_and_aggregates_but_not_partitions(pool: PgPool) {
    let mut conn = pool.acquire().await.unwrap();

    let (wrote, refusals) = as_runtime(&mut conn, async |conn| {
        let wrote = runtime_writes(conn).await.map_err(|e| e.to_string());
        let mut refusals = Vec::new();
        for sql in OWNER_ONLY {
            refusals.push((sql, refusal(conn, sql).await));
        }
        (wrote, refusals)
    })
    .await;

    assert_eq!(wrote, Ok((1, vec![1, 2, 1, 2, 1, 2])));
    for (sql, code) in refusals {
        assert_eq!(code.as_deref(), Some("42501"), "{sql}");
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn r_023_only_the_runtime_role_may_execute_the_partition_functions(pool: PgPool) {
    let grants: Vec<(String, bool, bool, i64)> = sqlx::query_as(
        "select p.oid::pg_catalog.regprocedure::pg_catalog.text, \
         p.proacl is not null, \
         pg_catalog.has_function_privilege('anima_runtime', p.oid, 'EXECUTE'), \
         (select count(*) from pg_catalog.aclexplode(p.proacl) a \
          where a.grantee = 0 and a.privilege_type = 'EXECUTE') \
         from pg_catalog.pg_proc p \
         where p.pronamespace = 'checks'::pg_catalog.regnamespace order by 1",
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    let names: Vec<&str> = grants.iter().map(|(name, ..)| name.as_str()).collect();
    assert_eq!(names, FUNCTIONS);
    for (name, explicit, runtime, public) in grants {
        // A null ACL means the default one, which grants EXECUTE to PUBLIC.
        assert!(explicit, "{name}");
        assert!(runtime, "{name}");
        assert_eq!(public, 0, "{name}");
    }
}

/// The bodies are also schema-qualified, so the attack below needs both defences removed;
/// this pins the search path on its own.
#[sqlx::test(migrations = "./migrations")]
async fn r_016_partition_functions_run_as_owner_with_pg_temp_searched_last(pool: PgPool) {
    let functions: Vec<(String, bool, Vec<String>)> = sqlx::query_as(
        "select p.oid::pg_catalog.regprocedure::pg_catalog.text, p.prosecdef, p.proconfig \
         from pg_catalog.pg_proc p \
         where p.pronamespace = 'checks'::pg_catalog.regnamespace order by 1",
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    assert_eq!(functions.len(), FUNCTIONS.len());
    for (name, definer, config) in functions {
        assert!(definer, "{name}");
        assert_eq!(config, ["search_path=pg_catalog, pg_temp"], "{name}");
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn r_023_runtime_role_cannot_run_code_as_the_partition_owner_through_pg_temp(pool: PgPool) {
    let mut conn = pool.acquire().await.unwrap();

    let (outcome, injected) = as_runtime(&mut conn, async |conn| {
        execute(conn, "create temp table injected (via pg_catalog.text)").await;
        execute(
            conn,
            "create function pg_temp.evil(via pg_catalog.text) returns pg_catalog.bool \
             language sql as $$ insert into pg_temp.injected values (via); select true $$",
        )
        .await;
        for name in SHADOWED_TYPES {
            let domain = format!(
                "create domain pg_temp.{name} as pg_catalog.{name} check (pg_temp.evil('{name}'))"
            );
            execute(conn, &domain).await;
        }
        let ensured = ensure(conn, at("2001-07-07T00:00:00Z")).await;
        let dropped: Result<Vec<String>, _> =
            sqlx::query_scalar("select checks.drop_day_partitions_before('2002-01-01', now())")
                .fetch_all(&mut *conn)
                .await;
        let injected: Vec<String> = sqlx::query_scalar("select via from pg_temp.injected")
            .fetch_all(&mut *conn)
            .await
            .unwrap();
        ((ensured.is_ok(), dropped.ok()), injected)
    })
    .await;

    assert_eq!(
        outcome,
        (true, Some(vec!["heartbeats_20010707".to_owned()]))
    );
    assert_eq!(injected, Vec::<String>::new(), "code ran as the owner");
}
