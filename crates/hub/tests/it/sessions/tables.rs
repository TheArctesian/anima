//! Migration 0600: columns, named checks and what a device delete does (K2, RA-008, SEC-006).

use sqlx::PgPool;

use super::{
    DOCKER_HOST, ECDSA_FINGERPRINT, ED25519, ED25519_KEY, PVE_01, Row, add_operator, constraint_of,
    delete_device, insert, insert_all, put_host_key, session_id,
};
use anima_hub::sessions::EndReason;

async fn columns(pool: &PgPool, table: &str) -> Vec<(String, String, String)> {
    sqlx::query_as(
        "select column_name::text, data_type::text, is_nullable::text \
         from information_schema.columns where table_schema = 'sessions' and table_name = $1 \
         order by ordinal_position",
    )
    .bind(table)
    .fetch_all(pool)
    .await
    .unwrap()
}

fn expected(columns: &[(&str, &str, &str)]) -> Vec<(String, String, String)> {
    columns
        .iter()
        .map(|(a, b, c)| ((*a).to_owned(), (*b).to_owned(), (*c).to_owned()))
        .collect()
}

#[sqlx::test(migrations = "./migrations")]
async fn k2_session_tables_have_the_contract_columns(pool: PgPool) {
    let (time, text, uuid) = ("timestamp with time zone", "text", "uuid");
    assert_eq!(
        columns(&pool, "sessions").await,
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
        columns(&pool, "host_keys").await,
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
    for reason in ["crashed", "OPERATOR", "", "operator "] {
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
    for set in [
        "end_reason = 'operator'",
        "ended_at = started_at",
        "protocol = 'rdp'",
    ] {
        let error = sqlx::query(&format!("update sessions.sessions set {set} where id = $1"))
            .bind(session_id(1))
            .execute(&pool)
            .await
            .unwrap_err();
        let expected = if set.starts_with("protocol") {
            "sessions_protocol_check"
        } else {
            "sessions_end_pair_check"
        };
        assert_eq!(constraint_of(&error), Some(expected), "{set}");
    }
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
    let keys: Vec<String> = sqlx::query_scalar("select device_id::text from sessions.host_keys")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(keys, [DOCKER_HOST]);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn sec_006_a_host_key_row_must_be_one_consistent_openssh_key(pool: PgPool) {
    add_operator(&pool).await;
    let cases = [
        (
            ("ssh-ed25519", ED25519, ECDSA_FINGERPRINT),
            "host_keys_fingerprint_matches_key",
        ),
        (
            ("ssh ed25519", ED25519, ED25519_KEY.2),
            "host_keys_algorithm_check",
        ),
        (
            ("ssh-ed25519", "AAAA\nAAAA", ED25519_KEY.2),
            "host_keys_public_key_check",
        ),
        (
            ("ssh-ed25519", "AAAAA", ED25519_KEY.2),
            "host_keys_public_key_check",
        ),
        (
            ("ssh-ed25519", ED25519, "SHA256:short"),
            "host_keys_fingerprint_check",
        ),
    ];
    for (key, constraint) in cases {
        let error = put_host_key(&pool, PVE_01, key).await.unwrap_err();
        assert_eq!(constraint_of(&error), Some(constraint), "{key:?}");
    }
    sqlx::query("delete from access.operators")
        .execute(&pool)
        .await
        .unwrap();
    let error = put_host_key(&pool, PVE_01, ED25519_KEY).await.unwrap_err();
    assert_eq!(constraint_of(&error), Some("host_keys_accepted_by_fkey"));
}

#[sqlx::test(migrations = "./migrations")]
async fn sec_006_a_host_key_needs_an_existing_device(pool: PgPool) {
    add_operator(&pool).await;
    let error = put_host_key(&pool, PVE_01, ED25519_KEY).await.unwrap_err();
    assert_eq!(constraint_of(&error), Some("host_keys_device_id_fkey"));
}
