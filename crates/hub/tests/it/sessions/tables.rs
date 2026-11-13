//! Migrations 0600 and 0610: columns, session checks and what a device delete does (K2,
//! RA-008, SEC-006); the server certificate table is `server_certificates`. Migration 0920
//! moved `host_keys` and `server_certificates` to schema `trust` (D-31). Migrations 0121, 0662
//! and 0901 removed what recording left (R-113).

mod server_certificates;

use sqlx::PgPool;

use super::{
    DOCKER_HOST, ED25519_KEY, PVE_01, Row, add_operator, constraint_of, delete_device, insert,
    insert_all, put_host_key, session_id,
};
use anima_hub::fleet::sessions::EndReason;

pub(super) async fn columns(
    pool: &PgPool,
    schema: &str,
    table: &str,
) -> Vec<(String, String, String)> {
    sqlx::query_as(
        "select column_name::text, data_type::text, is_nullable::text \
         from information_schema.columns where table_schema = $1 and table_name = $2 \
         order by ordinal_position",
    )
    .bind(schema)
    .bind(table)
    .fetch_all(pool)
    .await
    .unwrap()
}

pub(super) fn expected(columns: &[(&str, &str, &str)]) -> Vec<(String, String, String)> {
    columns
        .iter()
        .map(|(a, b, c)| ((*a).to_owned(), (*b).to_owned(), (*c).to_owned()))
        .collect()
}

#[sqlx::test(migrations = "./migrations")]
async fn k2_session_tables_have_the_contract_columns(pool: PgPool) {
    let (time, text, uuid) = ("timestamp with time zone", "text", "uuid");
    assert_eq!(
        columns(&pool, "sessions", "sessions").await,
        expected(&[
            ("id", uuid, "NO"),
            ("device_id", uuid, "YES"),
            ("device_name", text, "NO"),
            ("protocol", text, "NO"),
            ("operator_id", uuid, "NO"),
            ("operator_name", text, "NO"),
            ("client_addr", "inet", "NO"),
            ("started_at", time, "NO"),
            ("ended_at", time, "YES"),
            ("end_reason", text, "YES"),
        ])
    );
    assert_eq!(
        columns(&pool, "trust", "host_keys").await,
        expected(&[
            ("device_id", uuid, "NO"),
            ("algorithm", text, "NO"),
            ("fingerprint_sha256", text, "NO"),
            ("public_key", text, "NO"),
            ("first_seen_at", time, "NO"),
            ("accepted_at", time, "NO"),
            ("accepted_by", uuid, "NO"),
        ])
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn ra_008_end_reason_takes_every_k1_value_and_refuses_other_text(pool: PgPool) {
    for (n, reason) in (1..).zip(EndReason::ALL) {
        insert(&pool, &Row::ended(n, PVE_01, 0, reason.as_str()))
            .await
            .unwrap();
    }
    for reason in [
        "crashed",
        "OPERATOR",
        "",
        "operator ",
        "SERVER_CERTIFICATE_MISMATCH",
    ] {
        let error = insert(&pool, &Row::ended(99, PVE_01, 0, reason))
            .await
            .unwrap_err();
        assert_eq!(
            constraint_of(&error),
            Some("sessions_end_reason_check"),
            "{reason:?}"
        );
    }
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn ra_008_ended_at_and_end_reason_are_set_together(pool: PgPool) {
    insert(&pool, &Row::live(1, PVE_01, 0)).await.unwrap();
    for set in ["end_reason = 'operator'", "ended_at = started_at"] {
        let error = update(&pool, set).await.unwrap_err();
        assert_eq!(
            constraint_of(&error),
            Some("sessions_end_pair_check"),
            "{set}"
        );
    }
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn ra_030_protocol_is_ssh_rdp_or_vnc(pool: PgPool) {
    insert(&pool, &Row::live(1, PVE_01, 0)).await.unwrap();
    for protocol in ["rdp", "vnc", "ssh"] {
        update(&pool, &format!("protocol = '{protocol}'"))
            .await
            .unwrap();
    }
    for protocol in ["telnet", "RDP", "SSH", "vnc ", ""] {
        let error = update(&pool, &format!("protocol = '{protocol}'"))
            .await
            .unwrap_err();
        assert_eq!(
            constraint_of(&error),
            Some("sessions_protocol_check"),
            "{protocol:?}"
        );
    }
}

/// Runs `update … set {set}` on session 1.
async fn update(pool: &PgPool, set: &str) -> Result<(), sqlx::Error> {
    sqlx::query(&format!("update sessions.sessions set {set} where id = $1"))
        .bind(session_id(1))
        .execute(pool)
        .await
        .map(|_| ())
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn ra_008_session_history_survives_device_delete_and_host_key_does_not(pool: PgPool) {
    add_operator(&pool).await;
    insert_all(
        &pool,
        &[
            Row::ended(1, PVE_01, 0, "operator"),
            Row::live(2, DOCKER_HOST, 1),
        ],
    )
    .await;
    put_host_key(&pool, PVE_01, ED25519_KEY).await.unwrap();
    put_host_key(&pool, DOCKER_HOST, ED25519_KEY).await.unwrap();

    delete_device(&pool, PVE_01).await;

    let kept: (String, String) =
        sqlx::query_as("select device_id::text, device_name from sessions.sessions where id = $1")
            .bind(session_id(1))
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(kept, (PVE_01.to_owned(), "pve-01".to_owned()));
    let keys: Vec<String> = sqlx::query_scalar("select device_id::text from trust.host_keys")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(keys, [DOCKER_HOST]);
}

/// R-113 (RA-008, RA-040 deferred): after every migration the hub has no table for session
/// content, no recording flag on a device and no recordings retention period.
#[sqlx::test(migrations = "./migrations")]
async fn ra_008_no_recording_table_flag_or_period_is_left_after_migrate(pool: PgPool) {
    let table: Option<String> =
        sqlx::query_scalar("select to_regclass('sessions.recordings')::text")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(table, None, "sessions.recordings");
    let flag = columns(&pool, "devices", "devices")
        .await
        .into_iter()
        .filter(|(name, _, _)| name == "record_sessions")
        .count();
    assert_eq!(flag, 0, "devices.record_sessions");
    let period: i64 =
        sqlx::query_scalar("select count(*) from retention.periods where subject = 'recordings'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(period, 0, "retention.periods recordings");
    let refused =
        sqlx::query("insert into retention.periods (subject, days) values ('recordings', 30)")
            .execute(&pool)
            .await;
    assert!(
        refused.is_err(),
        "the subject check no longer takes recordings"
    );
}
