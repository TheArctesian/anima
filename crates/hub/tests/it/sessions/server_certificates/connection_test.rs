//! `testConnection` with `protocol: RDP` or `VNC` (DEV-013, SEC-007): the certificate is read
//! the hub's own way and reported by the accepted-certificate rule, nothing follows the TLS
//! handshake or the VNC banner, and the target rule refuses before any connection (R-032).

use anima_hub::sessions::Live;
use chrono::{DateTime, Utc};
use serde_json::{Value, json};
use sqlx::PgPool;

use super::{
    accept_request, instant, point_at, result,
    servers::{Answer, Certificate, rdp, vnc},
    test_request,
};
use crate::sessions::{
    PVE_01, add_operator,
    harness::error_json,
    host_keys::{loopback_live, schema},
    now,
};

/// The Connection Request for TLS, NLA and NLA with the early result, and no user name.
const ASKS_FOR_TLS: [u8; 19] = [
    3, 0, 0, 19, 14, 0xe0, 0, 0, 0, 0, 0, 1, 0, 8, 0, 0x0b, 0, 0, 0,
];

fn unsaved_rdp(port: u16, rdp: &Value) -> Value {
    let mut settings = json!({ "enabled": true, "port": port, "username": "operator" });
    settings
        .as_object_mut()
        .unwrap()
        .extend(rdp.as_object().unwrap().clone());
    json!({ "protocol": "RDP", "host": "127.0.0.1", "rdp": settings })
}

fn says_no_sign_in(test: &Value) -> bool {
    test["message"]
        .as_str()
        .is_some_and(|text| text.contains("does not sign in"))
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn sec_007_an_rdp_test_reads_the_certificate_and_sends_nothing_after_the_handshake(
    pool: PgPool,
) {
    let certificate = Certificate::new("DESKTOP-01");
    let mut server = rdp(Answer::Tls, &certificate).await;
    let schema = schema(&pool, &loopback_live());
    let mut input = unsaved_rdp(server.port, &json!({}));
    input["password"] = json!({ "password": "a-correct-horse-password" });

    let test = result(schema.execute(test_request(&input)).await);
    let visit = server.visit().await;

    assert_eq!(test["reachable"], true);
    assert_eq!(test["authenticated"], false);
    assert!(says_no_sign_in(&test), "{test}");
    assert_eq!(test["hostKey"], Value::Null);
    let presented = &test["serverCertificate"];
    assert_eq!(presented["fingerprintSha256"], certificate.fingerprint());
    assert_eq!(presented["subject"], "CN=DESKTOP-01");
    assert_eq!(
        instant(&presented["notAfter"]),
        "2031-01-01T00:00:00Z".parse::<DateTime<Utc>>().unwrap()
    );
    assert_eq!(instant(&presented["firstSeenAt"]), now());
    assert_eq!(test["certificateChanged"], false);
    assert_eq!(visit.request, ASKS_FOR_TLS);
    assert!(visit.handshake);
    assert_eq!(visit.after, Vec::<u8>::new(), "no credential, no CredSSP");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn sec_007_a_saved_device_reports_a_new_an_equal_and_a_changed_certificate(pool: PgPool) {
    add_operator(&pool).await;
    let (first, second) = (
        Certificate::new("DESKTOP-01"),
        Certificate::new("DESKTOP-01"),
    );
    let server = rdp(Answer::Tls, &first).await;
    point_at(&pool, server.port, "nla", false).await;
    let schema = schema(&pool, &loopback_live());
    let saved = json!({ "protocol": "RDP", "deviceId": PVE_01 });

    let new = result(schema.execute(test_request(&saved)).await);
    let accepted = schema
        .execute(accept_request(PVE_01, &first.fingerprint()))
        .await;
    let equal = result(schema.execute(test_request(&saved)).await);
    let replaced = rdp(Answer::Tls, &second).await;
    point_at(&pool, replaced.port, "nla", false).await;
    let changed = result(schema.execute(test_request(&saved)).await);

    assert_eq!(
        new["serverCertificate"]["fingerprintSha256"],
        first.fingerprint()
    );
    assert_eq!(new["certificateChanged"], false);
    assert!(accepted.errors.is_empty(), "{:?}", accepted.errors);
    assert_eq!(equal["serverCertificate"], Value::Null);
    assert_eq!(equal["certificateChanged"], false);
    assert_eq!(equal["reachable"], true);
    assert!(says_no_sign_in(&equal), "{equal}");
    assert_eq!(
        changed["serverCertificate"]["fingerprintSha256"],
        second.fingerprint()
    );
    assert_eq!(changed["certificateChanged"], true);
    assert_eq!(changed["authenticated"], false);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn sec_007_an_ignored_certificate_is_still_read_and_reported(pool: PgPool) {
    let certificate = Certificate::new("DESKTOP-01");
    let server = rdp(Answer::Tls, &certificate).await;
    point_at(&pool, server.port, "any", true).await;
    let schema = schema(&pool, &loopback_live());

    let saved = json!({ "protocol": "RDP", "deviceId": PVE_01 });
    let test = result(schema.execute(test_request(&saved)).await);

    let fingerprint = &test["serverCertificate"]["fingerprintSha256"];
    assert_eq!(fingerprint, &json!(certificate.fingerprint()));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn sec_007_a_server_without_tls_is_a_result_with_no_certificate(pool: PgPool) {
    let certificate = Certificate::new("DESKTOP-01");
    let schema = schema(&pool, &loopback_live());
    let answers = [Answer::Standard, Answer::Failure(2)];
    for answer in answers {
        let mut server = rdp(answer.clone(), &certificate).await;
        let input = unsaved_rdp(server.port, &json!({}));

        let test = result(schema.execute(test_request(&input)).await);
        let visit = server.visit().await;

        assert_eq!(test["reachable"], true, "{answer:?}");
        assert_eq!(test["serverCertificate"], Value::Null, "{answer:?}");
        assert!(says_no_sign_in(&test), "{test}");
        assert!(!visit.handshake, "{answer:?}");
    }
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn sec_007_standard_security_asks_for_no_tls(pool: PgPool) {
    let certificate = Certificate::new("DESKTOP-01");
    let mut server = rdp(Answer::Standard, &certificate).await;
    let schema = schema(&pool, &loopback_live());
    let input = unsaved_rdp(server.port, &json!({ "security": "RDP" }));

    let test = result(schema.execute(test_request(&input)).await);
    let visit = server.visit().await;

    assert_eq!(test["reachable"], true);
    assert_eq!(test["serverCertificate"], Value::Null);
    assert_eq!(visit.request[15..], [0, 0, 0, 0], "PROTOCOL_RDP only");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn sec_007_a_reply_that_is_not_rdp_is_an_unreachable_result(pool: PgPool) {
    let certificate = Certificate::new("DESKTOP-01");
    let schema = schema(&pool, &loopback_live());
    let replies = [
        b"HTTP/1.1 400 Bad Request\r\n\r\n".to_vec(),
        vec![3, 0],
        vec![3, 0, 0xff, 0xff, 0, 0],
        vec![3, 0, 0, 19, 14, 0xd0, 0, 0],
        vec![3, 0, 0, 19, 14, 0xd0, 0, 0, 0, 0, 0, 2, 0, 8, 0, 4, 0, 0, 0],
    ];
    for reply in replies {
        let server = rdp(Answer::Raw(reply.clone()), &certificate).await;
        let input = unsaved_rdp(server.port, &json!({}));

        let test = result(schema.execute(test_request(&input)).await);

        assert_eq!(test["reachable"], false, "{reply:?}");
        assert_eq!(test["serverCertificate"], Value::Null, "{reply:?}");
        assert_eq!(
            test["message"],
            "The device did not answer as an RDP server."
        );
    }
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn dev_013_a_vnc_test_reads_the_banner_and_sends_nothing(pool: PgPool) {
    let mut server = vnc(b"RFB 003.008\n").await;
    let not_vnc = vnc(b"SSH-2.0-OpenSSH_9.6\r\n").await;
    let schema = schema(&pool, &loopback_live());
    let input = |port: u16| {
        json!({ "protocol": "VNC", "host": "127.0.0.1", "vnc": { "enabled": true, "port": port },
                "password": { "password": "a-correct-horse-password" } })
    };

    let test = result(schema.execute(test_request(&input(server.port))).await);
    let visit = server.visit().await;
    let other = result(schema.execute(test_request(&input(not_vnc.port))).await);

    assert_eq!(test["reachable"], true);
    assert_eq!(test["authenticated"], false);
    assert_eq!(test["serverCertificate"], Value::Null);
    assert!(says_no_sign_in(&test), "{test}");
    assert_eq!(
        visit.after,
        Vec::<u8>::new(),
        "no reply to the banner, no credential"
    );
    assert_eq!(other["reachable"], false);
    assert_eq!(
        other["message"],
        "The device did not answer as a VNC server."
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn r_032_the_target_rule_refuses_rdp_and_vnc_before_any_connection(pool: PgPool) {
    let certificate = Certificate::new("DESKTOP-01");
    let mut rdp_server = rdp(Answer::Tls, &certificate).await;
    let mut vnc_server = vnc(b"RFB 003.008\n").await;
    point_at(&pool, rdp_server.port, "nla", false).await;
    let strict = schema(&pool, &Live::default());
    let inputs = [
        unsaved_rdp(rdp_server.port, &json!({})),
        json!({ "protocol": "RDP", "deviceId": PVE_01 }),
        json!({ "protocol": "VNC", "host": "127.0.0.1",
                "vnc": { "enabled": true, "port": vnc_server.port } }),
    ];
    for input in inputs {
        let response = strict.execute(test_request(&input)).await;
        let error = error_json(&response);
        assert_eq!(error["extensions"]["code"], "VALIDATION", "{input}");
        assert_eq!(error["extensions"]["fields"][0]["field"], "host", "{input}");
    }
    assert!(!rdp_server.was_visited().await);
    assert!(!vnc_server.was_visited().await);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn dev_013_settings_of_another_protocol_or_with_a_device_are_refused(pool: PgPool) {
    let schema = schema(&pool, &loopback_live());
    let cases = [
        (
            json!({ "host": "10.10.0.2", "rdp": { "enabled": true, "username": "u" } }),
            "rdp",
        ),
        (
            json!({ "host": "10.10.0.2", "password": { "password": "p" } }),
            "password",
        ),
        (
            json!({ "protocol": "RDP", "host": "10.10.0.2", "ssh": { "enabled": true,
                 "username": "u", "auth": "PASSWORD" } }),
            "ssh",
        ),
        (
            json!({ "protocol": "VNC", "deviceId": PVE_01, "credential": { "password": "p" } }),
            "credential",
        ),
        (
            json!({ "protocol": "RDP", "host": "10.10.0.2", "vnc": { "enabled": true } }),
            "vnc",
        ),
    ];
    let with_device = json!({ "protocol": "RDP", "deviceId": PVE_01,
                              "rdp": { "enabled": true, "username": "u" } });
    let cases = cases.into_iter().chain([(with_device, "deviceId")]);
    for (input, field) in cases {
        let error = error_json(&schema.execute(test_request(&input)).await);
        assert_eq!(error["extensions"]["code"], "VALIDATION", "{input}");
        assert_eq!(error["extensions"]["fields"][0]["field"], field, "{input}");
    }
}
