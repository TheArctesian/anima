//! `certificateMatched` through the schema (SEC-007): true only when the hub read the RDP
//! server certificate and it equals the accepted pin. A null `serverCertificate` is not proof
//! of a match, so a device that answers without TLS after a pin is accepted is no match.

use serde_json::json;
use sqlx::PgPool;

use super::{
    accept_request, point_at, result,
    servers::{Answer, Certificate, rdp},
    test_request,
};
use crate::sessions::{
    PVE_01, add_operator,
    host_keys::{loopback_live, schema},
};

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn sec_007_only_the_accepted_certificate_is_matched(pool: PgPool) {
    add_operator(&pool).await;
    let (first, second) = (
        Certificate::new("DESKTOP-01"),
        Certificate::new("DESKTOP-01"),
    );
    let schema = schema(&pool, &loopback_live());
    let saved = json!({ "protocol": "RDP", "deviceId": PVE_01 });
    let server = rdp(Answer::Tls, &first).await;
    point_at(&pool, server.port, "nla", false).await;

    let new = result(schema.execute(test_request(&saved)).await);
    let accepted = schema
        .execute(accept_request(PVE_01, &first.fingerprint()))
        .await;
    let equal = result(schema.execute(test_request(&saved)).await);
    let replaced = rdp(Answer::Tls, &second).await;
    point_at(&pool, replaced.port, "nla", false).await;
    let changed = result(schema.execute(test_request(&saved)).await);
    let standard = rdp(Answer::Standard, &first).await;
    point_at(&pool, standard.port, "nla", false).await;
    let downgraded = result(schema.execute(test_request(&saved)).await);

    assert!(accepted.errors.is_empty(), "{:?}", accepted.errors);
    assert_eq!(new["certificateMatched"], false, "first certificate");
    assert_eq!(equal["certificateMatched"], true, "accepted certificate");
    assert_eq!(changed["certificateMatched"], false, "changed certificate");
    assert_eq!(downgraded["serverCertificate"], json!(null));
    assert_eq!(downgraded["certificateMatched"], false, "standard only");
}
