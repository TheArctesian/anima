//! `acceptServerCertificate` (SEC-007, LOG-005): only the certificate a test saw in the last
//! 15 minutes, by its fingerprint; stored with an audit entry in the same transaction.

use async_graphql::Response;
use chrono::{DateTime, TimeDelta, Utc};
use serde_json::{Value, json};
use sqlx::PgPool;

use super::{
    accept_request, instant, point_at,
    servers::{Answer, Certificate, rdp},
    test_request,
};
use crate::sessions::{
    OPERATOR_ID, PVE_01, add_operator,
    harness::{error_json, schema_at},
    host_keys::{loopback_live, schema},
    now,
};

/// The stored certificate row of PVE-01.
#[derive(Debug, PartialEq, sqlx::FromRow)]
struct Row {
    fingerprint_sha256: String,
    subject: Option<String>,
    not_after: Option<DateTime<Utc>>,
    first_seen_at: DateTime<Utc>,
    accepted_by: String,
}

async fn row(pool: &PgPool) -> Option<Row> {
    sqlx::query_as(
        "select fingerprint_sha256, subject, not_after, first_seen_at, accepted_by::text \
         from trust.server_certificates where device_id = $1::text::uuid",
    )
    .bind(PVE_01)
    .fetch_optional(pool)
    .await
    .unwrap()
}

/// The audit entries: action, target type, target id and change, oldest first.
async fn audits(pool: &PgPool) -> Vec<(String, String, String, Value)> {
    sqlx::query_as(
        "select action, target_type, target_id, change from events.audit_log order by at",
    )
    .fetch_all(pool)
    .await
    .unwrap()
}

fn code(response: &Response) -> Value {
    error_json(response)["extensions"]["code"].clone()
}

/// An RDP test of the saved PVE-01.
fn saved() -> Value {
    json!({ "protocol": "RDP", "deviceId": PVE_01 })
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn sec_007_accept_without_a_test_or_with_another_fingerprint_is_conflict(pool: PgPool) {
    add_operator(&pool).await;
    let (presented, other) = (
        Certificate::new("DESKTOP-01"),
        Certificate::new("DESKTOP-01"),
    );
    let server = rdp(Answer::Tls, &presented).await;
    point_at(&pool, server.port, "nla", false).await;
    let live = loopback_live();
    let schema = schema(&pool, &live);

    let unseen = schema
        .execute(accept_request(PVE_01, &presented.fingerprint()))
        .await;
    let tested = schema.execute(test_request(&saved())).await;
    let wrong = schema
        .execute(accept_request(PVE_01, &other.fingerprint()))
        .await;
    let malformed = schema
        .execute(accept_request(
            PVE_01,
            "SHA256:4eECQSeWmB0ww/ND7ZkydOxBUAgciI4AwfhKt8R8Nbk",
        ))
        .await;
    let later = schema_at(&pool, now() + TimeDelta::minutes(15), &live);
    let expired = later
        .execute(accept_request(PVE_01, &presented.fingerprint()))
        .await;

    assert!(tested.errors.is_empty(), "{:?}", tested.errors);
    assert_eq!(code(&unseen), "CONFLICT");
    assert_eq!(code(&wrong), "CONFLICT");
    assert_eq!(code(&expired), "CONFLICT");
    assert_eq!(code(&malformed), "VALIDATION");
    assert_eq!(
        error_json(&malformed)["extensions"]["fields"][0]["field"],
        "fingerprintSha256"
    );
    assert_eq!(row(&pool).await, None);
    assert_eq!(audits(&pool).await, []);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn sec_007_log_005_accepting_the_seen_certificate_stores_it_with_an_audit_entry(
    pool: PgPool,
) {
    add_operator(&pool).await;
    let certificate = Certificate::new("DESKTOP-01");
    let server = rdp(Answer::Tls, &certificate).await;
    point_at(&pool, server.port, "nla", false).await;
    let live = loopback_live();
    let schema = schema(&pool, &live);
    let tested = schema.execute(test_request(&saved())).await;
    assert!(tested.errors.is_empty(), "{:?}", tested.errors);

    // Accepted a minute after the test, typed in upper case: first seen stays the test's time.
    let later = schema_at(&pool, now() + TimeDelta::minutes(1), &live);
    let typed = certificate.fingerprint().to_ascii_uppercase();
    let response = later.execute(accept_request(PVE_01, &typed)).await;
    let again = later
        .execute(accept_request(PVE_01, &certificate.fingerprint()))
        .await;

    assert!(response.errors.is_empty(), "{:?}", response.errors);
    assert!(again.errors.is_empty(), "{:?}", again.errors);
    let accepted = response.data.into_json().unwrap()["acceptServerCertificate"].clone();
    assert_eq!(accepted["fingerprintSha256"], certificate.fingerprint());
    assert_eq!(accepted["subject"], "CN=DESKTOP-01");
    assert_eq!(instant(&accepted["firstSeenAt"]), now());
    let expected = Row {
        fingerprint_sha256: certificate.fingerprint(),
        subject: Some("CN=DESKTOP-01".to_owned()),
        not_after: Some("2031-01-01T00:00:00Z".parse().unwrap()),
        first_seen_at: now(),
        accepted_by: OPERATOR_ID.to_owned(),
    };
    assert_eq!(row(&pool).await, Some(expected));
    let change = json!({
        "deviceName": "pve-01",
        "serverCertificate": { "oldFingerprint": null, "newFingerprint": certificate.fingerprint() },
    });
    let entry = (
        "accept".to_owned(),
        "device".to_owned(),
        PVE_01.to_owned(),
        change,
    );
    assert_eq!(
        audits(&pool).await,
        [entry],
        "accepting it again writes nothing"
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn sec_007_a_server_that_sends_its_issuer_too_is_reported_and_accepted_by_its_leaf(
    pool: PgPool,
) {
    add_operator(&pool).await;
    let certificate = Certificate::issued("DESKTOP-01");
    let server = rdp(Answer::Tls, &certificate).await;
    point_at(&pool, server.port, "nla", false).await;
    let live = loopback_live();
    let schema = schema(&pool, &live);

    let tested = schema.execute(test_request(&saved())).await;
    let response = schema
        .execute(accept_request(PVE_01, &certificate.fingerprint()))
        .await;

    assert!(tested.errors.is_empty(), "{:?}", tested.errors);
    let presented = tested.data.into_json().unwrap()["testConnection"]["serverCertificate"].clone();
    assert_eq!(presented["fingerprintSha256"], certificate.fingerprint());
    assert_eq!(presented["subject"], "CN=DESKTOP-01");
    assert!(response.errors.is_empty(), "{:?}", response.errors);
    let stored = row(&pool).await.unwrap();
    assert_eq!(stored.fingerprint_sha256, certificate.fingerprint());
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn q2_10_the_fingerprint_may_be_typed_as_64_digits_and_is_stored_as_colon_pairs(
    pool: PgPool,
) {
    add_operator(&pool).await;
    let certificate = Certificate::new("DESKTOP-01");
    let server = rdp(Answer::Tls, &certificate).await;
    point_at(&pool, server.port, "nla", false).await;
    let live = loopback_live();
    let schema = schema(&pool, &live);
    let tested = schema.execute(test_request(&saved())).await;

    // As a web interface shows it: no colons, upper case, with space around it.
    let digits = certificate
        .fingerprint()
        .replace(':', "")
        .to_ascii_uppercase();
    let response = schema
        .execute(accept_request(PVE_01, &format!(" {digits}\n")))
        .await;

    assert!(tested.errors.is_empty(), "{:?}", tested.errors);
    assert!(response.errors.is_empty(), "{:?}", response.errors);
    let accepted = response.data.into_json().unwrap()["acceptServerCertificate"].clone();
    assert_eq!(accepted["fingerprintSha256"], certificate.fingerprint());
    let stored = row(&pool).await.unwrap();
    assert_eq!(stored.fingerprint_sha256, certificate.fingerprint());
}
