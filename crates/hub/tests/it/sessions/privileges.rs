//! The runtime role may append a live session and end it once, never delete, reopen or
//! rewrite one (RA-008, R-016, K2 "Roles"). It may upsert a host key, because accepting one
//! upserts, but not delete one or move it to another device.

use chrono::TimeDelta;
use sqlx::{PgPool, Postgres, pool::PoolConnection};

use super::{
    DOCKER_HOST, ED25519_KEY, OPERATOR_ID, PVE_01, Row, add_operator, constraint_of, insert, now,
    put_host_key, session_id,
};

async fn as_runtime(pool: &PgPool) -> PoolConnection<Postgres> {
    let mut conn = pool.acquire().await.unwrap();
    sqlx::query("set role anima_runtime")
        .execute(&mut *conn)
        .await
        .unwrap();
    conn
}

/// SQLSTATE 42501, insufficient privilege.
fn is_refused(error: &sqlx::Error) -> bool {
    error
        .as_database_error()
        .and_then(sqlx::error::DatabaseError::code)
        .is_some_and(|code| code == "42501")
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn ra_008_runtime_role_can_end_a_session_but_not_delete_or_rewrite_it(pool: PgPool) {
    insert(&pool, &Row::live(1, PVE_01, 0)).await.unwrap();
    let mut conn = as_runtime(&pool).await;

    for sql in [
        "delete from sessions.sessions where id = $1",
        "update sessions.sessions set device_name = 'other' where id = $1",
        "update sessions.sessions set started_at = started_at - interval '1 day' where id = $1",
        "update sessions.sessions set operator_name = 'someone' where id = $1",
        "truncate sessions.sessions",
    ] {
        let query = sqlx::query(sql);
        let query = if sql.contains("$1") {
            query.bind(session_id(1))
        } else {
            query
        };
        let error = query.execute(&mut *conn).await.unwrap_err();
        assert!(is_refused(&error), "{sql}: {error}");
    }

    let ended = sqlx::query(
        "update sessions.sessions set ended_at = $2, end_reason = 'operator' \
         where id = $1 and ended_at is null",
    )
    .bind(session_id(1))
    .bind(now())
    .execute(&mut *conn)
    .await
    .unwrap();
    assert_eq!(ended.rows_affected(), 1);
}

/// Runs `sql` on session row `n` and returns the database's refusal.
async fn refused_update(conn: &mut PoolConnection<Postgres>, sql: &str, n: u128) -> sqlx::Error {
    sqlx::query(sql)
        .bind(session_id(n))
        .execute(&mut **conn)
        .await
        .unwrap_err()
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn ra_008_an_ended_session_cannot_be_reopened_or_rewritten(pool: PgPool) {
    insert(&pool, &Row::ended(1, PVE_01, 0, "operator"))
        .await
        .unwrap();
    let mut conn = as_runtime(&pool).await;
    for sql in [
        "update sessions.sessions set ended_at = null, end_reason = null where id = $1",
        "update sessions.sessions set end_reason = 'timeout' where id = $1",
        "update sessions.sessions set ended_at = ended_at + interval '1 hour' where id = $1",
        "update sessions.sessions set ended_at = ended_at, end_reason = end_reason \
         where id = $1",
    ] {
        let error = refused_update(&mut conn, sql, 1).await;
        assert_eq!(
            constraint_of(&error),
            Some("sessions_end_once"),
            "{sql}: {error}"
        );
    }
    drop(conn);
    // The owner is refused too: the rule is the table's, not a privilege.
    let error = refused_update(
        &mut pool.acquire().await.unwrap(),
        "update sessions.sessions set ended_at = null, end_reason = null where id = $1",
        1,
    )
    .await;
    assert_eq!(constraint_of(&error), Some("sessions_end_once"), "{error}");

    // The hub's end paths filter on `ended_at is null`: ending again is a no-op, not an error.
    let again = sqlx::query(
        "update sessions.sessions set ended_at = $2, end_reason = 'timeout' \
         where id = $1 and ended_at is null",
    )
    .bind(session_id(1))
    .bind(now() + TimeDelta::hours(1))
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(again.rows_affected(), 0);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn ra_008_a_live_session_ends_whole_and_not_before_it_started(pool: PgPool) {
    insert(&pool, &Row::live(1, PVE_01, 0)).await.unwrap();
    let mut conn = as_runtime(&pool).await;
    let before = sqlx::query(
        "update sessions.sessions set ended_at = $2, end_reason = 'operator' where id = $1",
    )
    .bind(session_id(1))
    .bind(now() - TimeDelta::microseconds(1))
    .execute(&mut *conn)
    .await
    .unwrap_err();
    assert_eq!(
        constraint_of(&before),
        Some("sessions_end_after_start"),
        "{before}"
    );
    // A half end is the 0600 pair check's to refuse; an end at no real instant, C3's.
    for (sql, constraint) in [
        (
            "update sessions.sessions set ended_at = started_at where id = $1",
            "sessions_end_pair_check",
        ),
        (
            "update sessions.sessions set end_reason = 'operator' where id = $1",
            "sessions_end_pair_check",
        ),
        (
            "update sessions.sessions set ended_at = 'infinity', end_reason = 'operator' \
             where id = $1",
            "sessions_times_finite",
        ),
    ] {
        let error = refused_update(&mut conn, sql, 1).await;
        assert_eq!(constraint_of(&error), Some(constraint), "{sql}: {error}");
    }
    let at_start = sqlx::query(
        "update sessions.sessions set ended_at = started_at, end_reason = 'operator' \
         where id = $1 and ended_at is null",
    )
    .bind(session_id(1))
    .execute(&mut *conn)
    .await
    .unwrap();
    assert_eq!(
        at_start.rows_affected(),
        1,
        "an end at the start instant is allowed"
    );
}

/// Inserts session row 1 on PVE-01 with the given times and end, through `conn`.
async fn insert_times(
    conn: &mut PoolConnection<Postgres>,
    started_at: &str,
    end: Option<(&str, &str)>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "insert into sessions.sessions (id, device_id, device_name, protocol, operator_id, \
                operator_name, client_addr, started_at, ended_at, end_reason) \
         values ($1, $2::text::uuid, 'pve-01', 'ssh', $3::text::uuid, 'operator', \
                 '10.10.0.77', $4::timestamptz, $5::timestamptz, $6)",
    )
    .bind(session_id(1))
    .bind(PVE_01)
    .bind(OPERATOR_ID)
    .bind(started_at)
    .bind(end.map(|(at, _)| at))
    .bind(end.map(|(_, reason)| reason))
    .execute(&mut **conn)
    .await
    .map(|_| ())
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn ra_008_a_session_is_inserted_live_with_real_times(pool: PgPool) {
    let start = "2026-11-04T09:00:00Z";
    let mut runtime = as_runtime(&pool).await;
    let ended = insert_times(&mut runtime, start, Some((start, "operator"))).await;
    assert!(
        is_refused(&ended.unwrap_err()),
        "the runtime role inserts live rows only"
    );
    drop(runtime);

    // The checks hold for the owner too, on insert as on update.
    let mut owner = pool.acquire().await.unwrap();
    for (started_at, end, constraint) in [
        (
            start,
            Some(("2026-11-04T08:59:59Z", "operator")),
            "sessions_end_after_start",
        ),
        ("infinity", None, "sessions_times_finite"),
        ("-infinity", None, "sessions_times_finite"),
        (
            start,
            Some(("infinity", "operator")),
            "sessions_times_finite",
        ),
    ] {
        let error = insert_times(&mut owner, started_at, end).await.unwrap_err();
        assert_eq!(
            constraint_of(&error),
            Some(constraint),
            "{started_at} {end:?}: {error}"
        );
    }
    insert_times(&mut owner, start, Some((start, "operator")))
        .await
        .unwrap();
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn sec_006_runtime_role_upserts_host_keys_but_cannot_delete_them(pool: PgPool) {
    add_operator(&pool).await;
    let mut conn = as_runtime(&pool).await;
    let (algorithm, key, fingerprint) = ED25519_KEY;
    for _ in 0..2 {
        sqlx::query(
            "insert into sessions.host_keys (device_id, algorithm, fingerprint_sha256, \
                    public_key, first_seen_at, accepted_at, accepted_by) \
             values ($1::text::uuid, $2, $3, $4, $5, $5, $6::text::uuid) \
             on conflict (device_id) do update set public_key = excluded.public_key, \
                 fingerprint_sha256 = excluded.fingerprint_sha256, \
                 accepted_at = excluded.accepted_at, accepted_by = excluded.accepted_by",
        )
        .bind(PVE_01)
        .bind(algorithm)
        .bind(fingerprint)
        .bind(key)
        .bind(now())
        .bind(OPERATOR_ID)
        .execute(&mut *conn)
        .await
        .unwrap();
    }
    for sql in [
        "delete from sessions.host_keys where device_id <> $1::text::uuid",
        "update sessions.host_keys set device_id = $1::text::uuid",
    ] {
        let error = sqlx::query(sql)
            .bind(DOCKER_HOST)
            .execute(&mut *conn)
            .await
            .unwrap_err();
        assert!(is_refused(&error), "{sql}: {error}");
    }
    assert_eq!(host_key_count(&pool).await, 1);
}

async fn host_key_count(pool: &PgPool) -> i64 {
    sqlx::query_scalar("select count(*) from sessions.host_keys")
        .fetch_one(pool)
        .await
        .unwrap()
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn sec_006_deleting_a_device_as_the_runtime_role_still_removes_its_host_key(pool: PgPool) {
    add_operator(&pool).await;
    put_host_key(&pool, PVE_01, ED25519_KEY).await.unwrap();
    let mut conn = as_runtime(&pool).await;
    let deleted = sqlx::query("delete from devices.devices where id = $1::text::uuid")
        .bind(PVE_01)
        .execute(&mut *conn)
        .await
        .unwrap();
    assert_eq!(deleted.rows_affected(), 1);
    assert_eq!(
        host_key_count(&pool).await,
        0,
        "the cascade runs with the owner's rights"
    );
}
