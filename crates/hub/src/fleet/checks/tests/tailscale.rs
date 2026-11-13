//! Tailscale ping: configuration, and the probe against a fake `LocalAPI` proxy on loopback
//! (D-4, CHK-009, LOG-007). The proxy is not judged by the policy, so the strict policy
//! that refuses loopback still reaches it.

mod proxy;

use std::{net::IpAddr, sync::atomic::Ordering, time::Duration};

use self::proxy::{Reply, Seen, start};
use crate::fleet::{
    checks::{
        config::{CheckConfig, CheckType, ConfigError, Reason},
        probe::{Judgement, ProbeEnv, ProbeResult, ProbeTarget, probe},
    },
    targets::AddressPolicy,
};

fn ping(target: Option<&str>, timeout_ms: u32) -> CheckConfig {
    CheckConfig::TailscalePing {
        target: target.map(str::to_owned),
        timeout_ms,
    }
}

fn error(config: &CheckConfig) -> Option<(&'static str, Reason)> {
    config
        .validate()
        .err()
        .map(|ConfigError { field, reason }| (field, reason))
}

fn env(proxy: Option<&proxy::Proxy>) -> ProbeEnv {
    let mut env = ProbeEnv::new(AddressPolicy::default(), chrono::Utc::now())
        .with_localapi(proxy.map(proxy::Proxy::address));
    let hosts = [("tail-nas".to_owned(), vec!["100.64.0.9".parse().unwrap()])];
    env.overrides.hosts = std::sync::Arc::new(hosts.into_iter().collect());
    env
}

async fn run(config: &CheckConfig, host: &str, env: &ProbeEnv) -> ProbeResult {
    let target = ProbeTarget {
        host: host.to_owned(),
    };
    probe(&target, config, None, env).await
}

fn down(result: &ProbeResult) -> &str {
    assert!(!result.ok);
    assert_eq!(result.judgement, Judgement::Answered);
    result.message.as_deref().unwrap()
}

#[test]
fn chk_009_the_tailscale_ping_id_defaults_and_stored_form_are_fixed() {
    assert_eq!(CheckType::TailscalePing.as_sql(), "tailscale_ping");
    assert_eq!(
        CheckType::from_sql("tailscale_ping"),
        Some(CheckType::TailscalePing)
    );
    assert_eq!(ping(None, 1).kind(), CheckType::TailscalePing);
    let bare: CheckConfig = serde_json::from_str(r#"{"type":"tailscale_ping"}"#).unwrap();
    assert_eq!(bare, ping(None, 5000));
    assert_eq!(bare.timeout(), Duration::from_secs(5));
    assert_eq!(
        serde_json::to_value(ping(Some("nas"), 3000)).unwrap(),
        serde_json::json!({"type": "tailscale_ping", "target": "nas", "timeoutMs": 3000})
    );
    let extra = r#"{"type":"tailscale_ping","port":1}"#;
    assert!(serde_json::from_str::<CheckConfig>(extra).is_err());
}

#[test]
fn chk_009_a_tailscale_ping_target_is_a_host_name_or_an_address_and_the_timeout_is_ranged() {
    for target in [
        "nas",
        "nas.tail1234.ts.net",
        "100.64.0.7",
        "fd7a:115c:a1e0::1",
        "[::1]",
    ] {
        assert_eq!(error(&ping(Some(target), 5000)), None, "{target}");
    }
    assert_eq!(error(&ping(None, 100)), None);
    assert_eq!(error(&ping(None, 60_000)), None);
    for target in [
        "nas; id",
        "a b",
        "-x",
        "http://nas",
        "nas/x",
        "nas:80",
        "n\nas",
    ] {
        let found = error(&ping(Some(target), 5000)).map(|(field, _)| field);
        assert_eq!(found, Some("target"), "{target:?}");
    }
    assert_eq!(
        error(&ping(Some(""), 5000)),
        Some(("target", Reason::Empty))
    );
    for ms in [0, 99, 60_001] {
        let found = error(&ping(None, ms));
        assert_eq!(
            found,
            Some((
                "timeoutMs",
                Reason::OutOfRange {
                    min: 100,
                    max: 60_000
                }
            ))
        );
    }
}

#[tokio::test]
async fn d_4_the_probe_posts_the_disco_ping_with_the_address_and_no_body() {
    let proxy = start(|| Reply::json(200, r#"{"Err":"","LatencySeconds":0.0123}"#)).await;

    run(
        &ping(Some("100.64.0.7"), 5000),
        "ignored",
        &env(Some(&proxy)),
    )
    .await;

    let [Seen { head, body_len }] = proxy.seen().try_into().unwrap();
    assert_eq!(
        head.lines().next(),
        Some("POST /localapi/v0/ping?ip=100.64.0.7&type=disco HTTP/1.1")
    );
    assert_eq!(body_len, 0);
    assert!(!head.to_ascii_lowercase().contains("transfer-encoding"));
    assert!(!head.to_ascii_lowercase().contains("authorization"));
}

#[tokio::test]
async fn d_4_an_empty_err_with_a_latency_is_up_and_the_latency_is_the_response_time() {
    let proxy = start(|| {
        Reply::json(
            200,
            r#"{"IP":"100.64.0.7","Err":"","LatencySeconds":0.0123}"#,
        )
    })
    .await;

    let result = run(&ping(Some("100.64.0.7"), 5000), "x", &env(Some(&proxy))).await;

    assert!(result.ok, "{result:?}");
    assert_eq!(result.response_time, Some(Duration::from_secs_f64(0.0123)));
    assert_eq!(
        result.resolved_ip,
        Some("100.64.0.7".parse::<IpAddr>().unwrap())
    );
    assert_eq!(result.judgement, Judgement::Answered);
}

#[tokio::test]
async fn d_4_the_device_host_is_pinged_when_the_check_names_no_target() {
    let proxy = start(|| Reply::json(200, r#"{"Err":"","LatencySeconds":0.002}"#)).await;

    let result = run(&ping(None, 5000), "Tail-NAS", &env(Some(&proxy))).await;

    assert!(result.ok);
    assert!(proxy.seen()[0].head.contains("ip=100.64.0.9&type=disco"));
}

#[tokio::test]
async fn d_4_an_err_is_down_with_a_fixed_message_that_keeps_no_proxy_text() {
    let reply = r#"{"Err":"no reply from secret-peer 100.64.0.7","LatencySeconds":0.5}"#;
    let proxy = start(move || Reply::json(200, reply)).await;

    let result = run(&ping(Some("100.64.0.7"), 5000), "x", &env(Some(&proxy))).await;

    assert_eq!(down(&result), "the Tailscale ping got no answer");
    assert!(result.response_time.is_none());
}

#[tokio::test]
async fn d_4_each_bad_proxy_answer_is_down_with_its_own_fixed_message() {
    let big = format!(
        r#"{{"Err":"","LatencySeconds":0.1,"Pad":"{}"}}"#,
        "a".repeat(70 * 1024)
    );
    let cases = [
        (
            Reply::json(403, "forbidden secret-detail"),
            "the LocalAPI proxy refused the ping",
        ),
        (
            Reply::json(500, "boom secret-detail"),
            "unexpected status 500",
        ),
        (Reply::json(302, ""), "unexpected status 302"),
        (
            Reply::json(200, "garbage secret-detail"),
            "the LocalAPI proxy reply cannot be read",
        ),
        (
            Reply::json(200, r#"{"Err":""}"#),
            "the LocalAPI proxy reply cannot be read",
        ),
        (
            Reply::json(200, r#"{"Err":"","LatencySeconds":-1}"#),
            "the LocalAPI proxy reply cannot be read",
        ),
        (
            Reply::json(200, &big),
            "the LocalAPI proxy reply was longer than the hub reads",
        ),
    ];
    for (reply, message) in cases {
        let proxy = start(move || reply.clone()).await;
        let result = run(&ping(Some("100.64.0.7"), 5000), "x", &env(Some(&proxy))).await;
        assert_eq!(down(&result), message);
        assert!(!result.message.unwrap().contains("secret"));
    }
}

#[tokio::test]
async fn d_4_a_proxy_that_never_answers_times_out_within_the_check_limit() {
    let proxy = start(|| Reply::Hang).await;
    let config = ping(Some("100.64.0.7"), 200);

    let started = std::time::Instant::now();
    let result = run(&config, "x", &env(Some(&proxy))).await;
    // `probe` applies the limit; this is the same call the scheduler makes.
    assert_eq!(down(&result), "the check timed out");
    assert!(started.elapsed() < Duration::from_secs(3));
}

#[tokio::test]
async fn d_4_a_refused_connection_to_the_proxy_is_down() {
    let proxy = start(|| Reply::Hang).await;
    let mut env = env(Some(&proxy));
    drop(proxy);
    env.localapi = Some("127.0.0.1:1".to_owned());

    let result = run(&ping(Some("100.64.0.7"), 2000), "x", &env).await;

    assert_eq!(down(&result), "the request failed");
}

#[tokio::test]
async fn d_4_without_a_proxy_the_result_is_unjudged_and_nothing_is_sent() {
    let proxy = start(|| Reply::json(200, "{}")).await;

    let result = run(&ping(Some("100.64.0.7"), 5000), "x", &env(None)).await;

    assert_eq!(result.judgement, Judgement::Unjudged);
    assert!(!result.ok);
    assert_eq!(
        result.message.as_deref(),
        Some("Tailscale ping needs the LocalAPI proxy (ANIMA_TAILSCALE_LOCALAPI)")
    );
    assert_eq!(proxy.connections.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn d_4_a_target_outside_the_tailnet_range_or_refused_by_the_policy_sends_nothing() {
    let proxy = start(|| Reply::json(200, r#"{"Err":"","LatencySeconds":0.1}"#)).await;
    let targets = [
        "8.8.8.8",
        "100.63.255.255",
        "100.128.0.1",
        "10.0.0.5",
        "127.0.0.1",
        "fd7a:115c:a1e0::1",
        // Inside 100.64.0.0/10, but Tailscale's own quad-100 range, which the policy refuses.
        "100.100.100.100",
    ];
    for target in targets {
        let result = run(&ping(Some(target), 5000), "x", &env(Some(&proxy))).await;
        assert_eq!(
            down(&result),
            "the target has no Tailscale address the hub may ping",
            "{target}"
        );
    }
    let mut denied = env(Some(&proxy));
    denied.policy = std::sync::Arc::new(AddressPolicy::new(vec!["100.64.9.0/24".parse().unwrap()]));
    let result = run(&ping(Some("100.64.9.4"), 5000), "x", &denied).await;
    assert_eq!(
        down(&result),
        "the target has no Tailscale address the hub may ping"
    );
    assert_eq!(proxy.connections.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn d_4_a_name_that_does_not_resolve_is_down_and_sends_nothing() {
    let proxy = start(|| Reply::json(200, "{}")).await;

    let result = run(
        &ping(Some("nowhere.invalid"), 5000),
        "x",
        &env(Some(&proxy)),
    )
    .await;

    assert!(!result.ok);
    assert_eq!(proxy.connections.load(Ordering::SeqCst), 0);
}
