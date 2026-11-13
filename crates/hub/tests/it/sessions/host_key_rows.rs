//! Migration 0600 keeps only host key rows the hub's reader accepts (SEC-006, SEC-007).

use anima_hub::{
    fleet::trust::{self, Fingerprint},
    ids::DeviceId,
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use sqlx::PgPool;

use super::{
    ECDSA, ECDSA_FINGERPRINT, ED25519, ED25519_KEY, PVE_01, add_operator, constraint_of,
    put_host_key,
};

/// The 8190-byte blob cap of `trust::public_key::MAX_BLOB`.
const MAX_BLOB: usize = 8190;

/// An `ssh-rsa` key blob of `len` bytes in base64, with its fingerprint.
fn rsa_key(len: usize) -> (String, String) {
    let mut blob = 7_u32.to_be_bytes().to_vec();
    blob.extend_from_slice(b"ssh-rsa");
    blob.resize(len, 1);
    (
        STANDARD.encode(&blob),
        Fingerprint::of_key(&blob).to_string(),
    )
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn sec_006_a_host_key_row_must_be_one_consistent_openssh_key(pool: PgPool) {
    add_operator(&pool).await;
    let long_name = "a".repeat(65);
    let (over_cap, over_cap_fingerprint) = rsa_key(MAX_BLOB + 1);
    // `decode` ignores the set pad bit in `N`; canonical text ends `TDM=`.
    let non_canonical = ECDSA.replace("TDM=", "TDN=");
    assert_ne!(non_canonical, ECDSA);
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
            (long_name.as_str(), ED25519, ED25519_KEY.2),
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
            ("ssh-rsa", over_cap.as_str(), over_cap_fingerprint.as_str()),
            "host_keys_public_key_check",
        ),
        (
            (
                "ecdsa-sha2-nistp256",
                non_canonical.as_str(),
                ECDSA_FINGERPRINT,
            ),
            "host_keys_public_key_canonical",
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

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn sec_007_the_largest_key_the_table_takes_is_one_the_hub_reads(pool: PgPool) {
    add_operator(&pool).await;
    let (largest, fingerprint) = rsa_key(MAX_BLOB);
    assert_eq!(largest.len(), 10_920);
    put_host_key(&pool, PVE_01, ("ssh-rsa", &largest, &fingerprint))
        .await
        .unwrap();
    let key = trust::host_key(&pool, PVE_01.parse::<DeviceId>().unwrap())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(key.public_key.fingerprint().as_str(), fingerprint);
}

#[sqlx::test(migrations = "./migrations")]
async fn sec_006_a_host_key_needs_an_existing_device(pool: PgPool) {
    add_operator(&pool).await;
    let error = put_host_key(&pool, PVE_01, ED25519_KEY).await.unwrap_err();
    assert_eq!(constraint_of(&error), Some("host_keys_device_id_fkey"));
}
