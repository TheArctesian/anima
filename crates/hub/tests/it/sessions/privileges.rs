//! The runtime role may append a session and end it, never delete or rewrite one (RA-008,
//! R-016, K2 "Roles"). Host keys keep the default privileges, because accepting one upserts.

use sqlx::{PgPool, Postgres, pool::PoolConnection};

use super::{ED25519_KEY, OPERATOR_ID, PVE_01, Row, add_operator, insert, now, session_id};

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

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn sec_006_runtime_role_upserts_and_deletes_host_keys(pool: PgPool) {
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
    let deleted = sqlx::query("delete from sessions.host_keys")
        .execute(&mut *conn)
        .await
        .unwrap();
    assert_eq!(deleted.rows_affected(), 1);
}
