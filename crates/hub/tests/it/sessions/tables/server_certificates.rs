//! Migration 0610's `server_certificates` (in schema `trust` since 0920): its columns and
//! checks, the device delete that removes a certificate, and a runtime role that may upsert but
//! never delete one (K2, SEC-007, R-016).

use sqlx::{PgConnection, PgPool};

use super::{columns, expected};
use crate::sessions::{
    DOCKER_HOST, OPERATOR_ID, PVE_01, add_operator, constraint_of, delete_device, now,
    privileges::{as_runtime, is_refused},
};

/// SHA-256 as `FreeRDP` writes it: 32 lower-case hex pairs joined by colons.
fn fingerprint(pair: &str) -> String {
    vec![pair; 32].join(":")
}

/// Upserts `device`'s certificate as `acceptServerCertificate` will, through `conn`.
async fn put(
    conn: &mut PgConnection,
    device: &str,
    fingerprint: &str,
    subject: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "insert into trust.server_certificates (device_id, fingerprint_sha256, subject, \
                not_after, first_seen_at, accepted_by) \
         values ($1::text::uuid, $2, $3, $4 + interval '1 year', $4, $5::text::uuid) \
         on conflict (device_id) do update set fingerprint_sha256 = excluded.fingerprint_sha256, \
             subject = excluded.subject, not_after = excluded.not_after, \
             first_seen_at = excluded.first_seen_at, accepted_by = excluded.accepted_by",
    )
    .bind(device)
    .bind(fingerprint)
    .bind(subject)
    .bind(now())
    .bind(OPERATOR_ID)
    .execute(&mut *conn)
    .await
    .map(|_| ())
}

async fn certified_devices(pool: &PgPool) -> Vec<String> {
    sqlx::query_scalar("select device_id::text from trust.server_certificates order by device_id")
        .fetch_all(pool)
        .await
        .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn k2_server_certificates_has_the_contract_columns(pool: PgPool) {
    let (time, text, uuid) = ("timestamp with time zone", "text", "uuid");
    assert_eq!(
        columns(&pool, "trust", "server_certificates").await,
        expected(&[
            ("device_id", uuid, "NO"),
            ("fingerprint_sha256", text, "NO"),
            ("subject", text, "YES"),
            ("not_after", time, "YES"),
            ("first_seen_at", time, "NO"),
            ("accepted_by", uuid, "NO"),
        ])
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn sec_007_a_certificate_row_holds_a_sha_256_fingerprint_and_a_plain_subject(pool: PgPool) {
    add_operator(&pool).await;
    let mut conn = pool.acquire().await.unwrap();
    put(
        &mut conn,
        PVE_01,
        &fingerprint("0f"),
        Some("CN=desk-01, O=Home"),
    )
    .await
    .unwrap();
    put(&mut conn, PVE_01, &fingerprint("a9"), None)
        .await
        .unwrap();
    let long = "x".repeat(1024);
    put(&mut conn, PVE_01, &fingerprint("00"), Some(&long))
        .await
        .unwrap();

    let short = vec!["ab"; 31].join(":");
    let upper = fingerprint("AB");
    let unseparated = "ab".repeat(32);
    let openssh = "SHA256:4eECQSeWmB0ww/ND7ZkydOxBUAgciI4AwfhKt8R8Nbk";
    for bad in [short.as_str(), &upper, &unseparated, openssh, ""] {
        let error = put(&mut conn, PVE_01, bad, None).await.unwrap_err();
        let constraint = constraint_of(&error);
        assert_eq!(
            constraint,
            Some("server_certificates_fingerprint_check"),
            "{bad:?}"
        );
    }
    let too_long = "x".repeat(1025);
    for bad in ["", "CN=a\nCN=b", "CN=a\u{1b}[2J", too_long.as_str()] {
        let error = put(&mut conn, PVE_01, &fingerprint("01"), Some(bad))
            .await
            .unwrap_err();
        let constraint = constraint_of(&error);
        assert_eq!(
            constraint,
            Some("server_certificates_subject_check"),
            "{bad:?}"
        );
    }
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn sec_007_a_certificate_goes_with_its_device_even_for_the_runtime_role(pool: PgPool) {
    add_operator(&pool).await;
    let mut owner = pool.acquire().await.unwrap();
    put(&mut owner, PVE_01, &fingerprint("01"), None)
        .await
        .unwrap();
    put(&mut owner, DOCKER_HOST, &fingerprint("02"), None)
        .await
        .unwrap();
    drop(owner);

    let mut runtime = as_runtime(&pool).await;
    let deleted = sqlx::query("delete from devices.devices where id = $1::text::uuid")
        .bind(PVE_01)
        .execute(&mut runtime)
        .await
        .unwrap();
    assert_eq!(deleted.rows_affected(), 1);
    assert_eq!(
        certified_devices(&pool).await,
        [DOCKER_HOST],
        "the cascade runs with the owner's rights"
    );
    delete_device(&pool, DOCKER_HOST).await;
    assert_eq!(certified_devices(&pool).await, Vec::<String>::new());
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn sec_007_runtime_role_upserts_certificates_but_cannot_delete_or_move_them(pool: PgPool) {
    add_operator(&pool).await;
    let mut conn = as_runtime(&pool).await;
    put(&mut conn, PVE_01, &fingerprint("01"), Some("CN=pve-01"))
        .await
        .unwrap();
    put(&mut conn, PVE_01, &fingerprint("02"), None)
        .await
        .unwrap();
    for sql in [
        "delete from trust.server_certificates where device_id = $1::text::uuid",
        "delete from trust.server_certificates where device_id <> $1::text::uuid",
        "update trust.server_certificates set device_id = $1::text::uuid",
        "truncate trust.server_certificates",
    ] {
        let query = sqlx::query(sql);
        let query = if sql.contains("$1") {
            query.bind(PVE_01)
        } else {
            query
        };
        let error = query.execute(&mut conn).await.unwrap_err();
        assert!(is_refused(&error), "{sql}: {error}");
    }
    assert_eq!(certified_devices(&pool).await, [PVE_01]);
}
