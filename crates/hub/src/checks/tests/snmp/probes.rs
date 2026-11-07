//! The SNMP probe against the local agent, v2c and v3 authPriv (D-5, CHK-001, LOG-007).

use std::sync::atomic::Ordering;

use super::agent::{Agent, Setup, spawn};
use crate::{
    checks::{
        config::{SnmpConfig, SnmpSecrets, SnmpVersion},
        probe::{
            Failure, ProbeResult,
            snmp::{self, DRAWS_LEN, Draws, message::Ids},
        },
        tests::probe::support::{LOOPBACK, env, strict_env, target},
    },
    credentials::Secret,
};

const NAME: &str = "1.3.6.1.2.1.1.5.0";
const UPTIME: &str = "1.3.6.1.2.1.1.3.0";

fn config(agent: &Agent, version: SnmpVersion, oid: &str) -> SnmpConfig {
    SnmpConfig {
        port: agent.port,
        version,
        oid: oid.to_owned(),
        expected: None,
        username: (version == SnmpVersion::V3).then(|| "monitor".to_owned()),
        timeout_ms: 1000,
    }
}

fn community(text: &str) -> SnmpSecrets {
    SnmpSecrets::Community(Secret::from(text.to_owned()))
}

fn user(auth: &str, privacy: &str) -> SnmpSecrets {
    SnmpSecrets::User {
        auth: Secret::from(auth.to_owned()),
        privacy: Secret::from(privacy.to_owned()),
    }
}

fn right_user() -> SnmpSecrets {
    let setup = Setup::new();
    user(setup.auth, setup.privacy)
}

fn right_community() -> SnmpSecrets {
    community(Setup::new().community)
}

/// The attempt's random values, fixed so a run is deterministic (CQ-021).
fn draws() -> Draws {
    let mut bytes = [0_u8; DRAWS_LEN];
    for (byte, value) in bytes.iter_mut().zip(1_u8..) {
        *byte = value;
    }
    Draws::from_bytes(&bytes)
}

async fn run(config: &SnmpConfig, secrets: &SnmpSecrets) -> ProbeResult {
    snmp::observe(&target("127.0.0.1"), (config, secrets), &env(), draws()).await
}

/// The message as stored, which must name none of the planted secrets.
fn message(result: &ProbeResult) -> String {
    let text = result.message.clone().unwrap_or_default();
    let setup = Setup::new();
    for secret in [setup.community, setup.auth, setup.privacy, "SECRET"] {
        assert!(!text.contains(secret), "{text}");
        assert!(!format!("{result:?}").contains(secret));
    }
    text
}

fn failed_with(result: &ProbeResult, failure: &Failure) {
    assert!(!result.ok);
    assert_eq!(message(result), failure.to_string());
    assert_eq!(result.response_time, None);
}

#[tokio::test]
async fn d_5_v2c_get_with_the_community_is_up_with_a_response_time() {
    let agent = spawn(Setup::new()).await;
    let result = run(&config(&agent, SnmpVersion::V2c, NAME), &right_community()).await;
    assert!(result.ok, "{}", message(&result));
    assert!(result.response_time.is_some());
    assert_eq!(result.resolved_ip, Some(LOOPBACK));
}

#[tokio::test]
async fn d_5_v2c_with_a_wrong_community_times_out_like_a_real_agent() {
    let agent = spawn(Setup::new()).await;
    let result = run(&config(&agent, SnmpVersion::V2c, NAME), &community("wrong")).await;
    failed_with(&result, &Failure::TimedOut);
    assert_eq!(agent.requests.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn d_5_v3_discovers_the_engine_then_gets_with_auth_priv() {
    let agent = spawn(Setup::new()).await;
    let result = run(&config(&agent, SnmpVersion::V3, NAME), &right_user()).await;
    assert!(result.ok, "{}", message(&result));
    assert!(result.response_time.is_some());
    assert_eq!(agent.requests.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn d_5_v3_with_a_wrong_auth_password_is_down_with_the_wrong_digest_report() {
    let agent = spawn(Setup::new()).await;
    let secrets = user("not-the-auth-password", Setup::new().privacy);
    let result = run(&config(&agent, SnmpVersion::V3, NAME), &secrets).await;
    failed_with(&result, &Failure::SnmpReport("wrong digest"));
}

#[tokio::test]
async fn d_5_v3_with_an_unknown_user_is_down_with_the_unknown_user_report() {
    let agent = spawn(Setup::new()).await;
    let mut config = config(&agent, SnmpVersion::V3, NAME);
    config.username = Some("intruder".to_owned());
    let result = run(&config, &right_user()).await;
    failed_with(&result, &Failure::SnmpReport("unknown user name"));
}

#[tokio::test]
async fn d_5_v3_answer_with_a_bad_digest_is_never_read() {
    let agent = spawn(Setup {
        tamper: true,
        ..Setup::new()
    })
    .await;
    let result = run(&config(&agent, SnmpVersion::V3, NAME), &right_user()).await;
    failed_with(&result, &Failure::SnmpAuthentication);
}

#[tokio::test]
async fn d_5_an_oid_the_agent_lacks_is_down_with_no_such_object() {
    let agent = spawn(Setup::new()).await;
    for (version, secrets) in [
        (SnmpVersion::V2c, right_community()),
        (SnmpVersion::V3, right_user()),
    ] {
        let config = config(&agent, version, "1.3.6.1.4.1.99999.1.0");
        let result = run(&config, &secrets).await;
        failed_with(&result, &Failure::SnmpNoValue("noSuchObject"));
    }
}

#[tokio::test]
async fn d_5_an_expected_value_must_equal_the_text_form() {
    let agent = spawn(Setup::new()).await;
    for (version, secrets) in [
        (SnmpVersion::V2c, right_community()),
        (SnmpVersion::V3, right_user()),
    ] {
        let mut config = config(&agent, version, NAME);
        config.expected = Some("nas-01".to_owned());
        assert!(run(&config, &secrets).await.ok, "{version:?}");
        config.expected = Some("nas-02".to_owned());
        failed_with(&run(&config, &secrets).await, &Failure::SnmpMismatch);
        let mut ticks = self::config(&agent, version, UPTIME);
        ticks.expected = Some("12345".to_owned());
        assert!(run(&ticks, &secrets).await.ok, "{version:?}");
    }
}

#[tokio::test]
async fn d_5_a_silent_agent_times_out() {
    let agent = spawn(Setup {
        silent: true,
        ..Setup::new()
    })
    .await;
    let result = run(&config(&agent, SnmpVersion::V3, NAME), &right_user()).await;
    failed_with(&result, &Failure::TimedOut);
}

#[tokio::test]
async fn d_5_a_reply_from_another_port_is_ignored() {
    let agent = spawn(Setup {
        stray_first: true,
        ..Setup::new()
    })
    .await;
    for (version, secrets) in [
        (SnmpVersion::V2c, right_community()),
        (SnmpVersion::V3, right_user()),
    ] {
        let result = run(&config(&agent, version, NAME), &secrets).await;
        assert!(result.ok, "{version:?}: {}", message(&result));
    }
}

#[tokio::test]
async fn d_5_a_refused_address_sends_nothing() {
    let agent = spawn(Setup::new()).await;
    let config = config(&agent, SnmpVersion::V2c, NAME);
    let pair = (&config, &right_community());
    let result = snmp::observe(&target("127.0.0.1"), pair, &strict_env(), draws()).await;
    failed_with(&result, &Failure::NotPermitted);
    assert_eq!(agent.requests.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn d_5_secrets_of_the_other_version_send_nothing() {
    let agent = spawn(Setup::new()).await;
    let result = run(&config(&agent, SnmpVersion::V2c, NAME), &right_user()).await;
    failed_with(&result, &Failure::SecretsUnbound);
    let result = run(&config(&agent, SnmpVersion::V3, NAME), &right_community()).await;
    failed_with(&result, &Failure::SecretsUnbound);
    assert_eq!(agent.requests.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn d_5_an_invalid_configuration_sends_nothing() {
    let agent = spawn(Setup::new()).await;
    let config = config(&agent, SnmpVersion::V2c, "1");
    failed_with(&run(&config, &right_community()).await, &Failure::BadConfig);
    assert_eq!(agent.requests.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn d_5_given_draws_are_used_for_the_request() {
    // The core takes its randomness as a parameter: a fixed salt and fixed ids still work.
    let agent = spawn(Setup::new()).await;
    let draws = Draws {
        discovery: Ids {
            message: 1,
            request: 2,
        },
        get: Ids {
            message: 3,
            request: 4,
        },
        salt: [7; 8],
    };
    let config = config(&agent, SnmpVersion::V3, NAME);
    let pair = (&config, &right_user());
    let result = snmp::observe(&target("127.0.0.1"), pair, &env(), draws).await;
    assert!(result.ok, "{}", message(&result));
}

#[test]
fn d_5_fresh_draws_differ_in_every_id_and_the_salt() {
    // Each value has at least 31 random bits, so a repeat here means the draw is not fresh.
    let (first, second) = (Draws::fresh().unwrap(), Draws::fresh().unwrap());
    assert_ne!(first.discovery.message, second.discovery.message);
    assert_ne!(first.discovery.request, second.discovery.request);
    assert_ne!(first.get.message, second.get.message);
    assert_ne!(first.get.request, second.get.request);
    assert_ne!(first.salt, second.salt);
}

#[test]
fn cq_021_draws_come_from_the_given_bytes() {
    // Each id is a big-endian word shifted right once, so it is never negative.
    let draws = draws();
    assert_eq!(draws.discovery.message, 0x0102_0304 >> 1);
    assert_eq!(draws.discovery.request, 0x0506_0708 >> 1);
    assert_eq!(draws.get.message, 0x090a_0b0c >> 1);
    assert_eq!(draws.get.request, 0x0d0e_0f10 >> 1);
    assert_eq!(draws.salt, [17, 18, 19, 20, 21, 22, 23, 24]);
    let high = Draws::from_bytes(&[0xff; DRAWS_LEN]);
    assert_eq!(high.discovery.message, i32::MAX);
    assert_eq!(high.get.request, i32::MAX);
}
