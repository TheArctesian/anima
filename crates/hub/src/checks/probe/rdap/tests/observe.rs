//! The whole probe against a local bootstrap file and RDAP service (D-29, P3-003): the
//! bootstrap read once a day by the clock given, the expiry read and judged, and the fixed
//! results of a registry that cannot be reached or read.

use std::time::Duration;

use chrono::{DateTime, TimeDelta, TimeZone as _, Utc};

use super::server::{Server, reply, serve};
use crate::{
    checks::probe::{
        outcome::{Failure, Judgement, ProbeResult},
        rdap::{Lookup, Rdap},
    },
    targets::AddressPolicy,
};

/// What the RDAP service answers for a domain name.
type RdapAnswer = fn(&str) -> Vec<u8>;

const NOT_AFTER: &str = "2026-11-20T12:00:00Z";

fn now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 8, 12, 0, 0).unwrap()
}

/// A registry for `.test` whose bootstrap file lives at `/dns.json` and whose RDAP service
/// answers `/rdap/domain/<name>` with `answer(name)`.
async fn registry(answer: RdapAnswer) -> Server {
    serve(move |path, port| {
        if path == "/dns.json" {
            let file =
                format!(r#"{{"services": [[["test"], ["https://localhost:{port}/rdap/"]]]}}"#);
            reply("200 OK", file.as_bytes())
        } else {
            answer(path.trim_start_matches("/rdap/domain/"))
        }
    })
    .await
}

fn expiring(_: &str) -> Vec<u8> {
    let body =
        format!(r#"{{"events": [{{"eventAction": "expiration", "eventDate": "{NOT_AFTER}"}}]}}"#);
    reply("200 OK", body.as_bytes())
}

fn rdap(server: &Server) -> Rdap {
    Rdap::for_tests(
        AddressPolicy::default().allowing_loopback(),
        server.root.clone(),
        server.url("/dns.json"),
    )
}

async fn observe(rdap: &Rdap, domain: &str, at: DateTime<Utc>) -> ProbeResult {
    let lookup = Lookup {
        domain,
        warn_days: 30,
        timeout: Duration::from_secs(10),
    };
    rdap.observe(lookup, at).await
}

#[tokio::test]
async fn d_29_the_probe_reads_the_expiry_of_the_domain_in_lower_case() {
    let server = registry(expiring).await;

    let result = observe(&rdap(&server), "Example.TEST", now()).await;

    let not_after = DateTime::parse_from_rfc3339(NOT_AFTER).unwrap();
    assert_eq!(result.domain_not_after, Some(not_after.with_timezone(&Utc)));
    assert_eq!(
        result.judgement,
        Judgement::Expiring,
        "12 days left, 30 to warn"
    );
    assert_eq!(server.requests_for("/rdap/domain/example.test"), 1);
}

#[tokio::test]
async fn d_29_the_bootstrap_file_is_read_once_a_day_by_the_clock_given() {
    let server = registry(expiring).await;
    let rdap = rdap(&server);
    let almost_a_day = TimeDelta::hours(24) - TimeDelta::seconds(1);

    observe(&rdap, "a.test", now()).await;
    observe(&rdap, "b.test", now() + TimeDelta::hours(1)).await;
    observe(&rdap, "a.test", now() + almost_a_day).await;
    assert_eq!(server.requests_for("/dns.json"), 1);

    observe(&rdap, "a.test", now() + TimeDelta::hours(24)).await;
    assert_eq!(server.requests_for("/dns.json"), 2);

    // A clock moved back by a day spends the file too.
    observe(&rdap, "a.test", now()).await;
    assert_eq!(server.requests_for("/dns.json"), 3);
    assert_eq!(server.requests_for("/rdap/domain/a.test"), 4);
}

#[tokio::test]
async fn d_29_a_failed_bootstrap_read_is_not_kept() {
    let server = serve(|_, _| reply("503 Service Unavailable", b"")).await;
    let rdap = rdap(&server);

    let first = observe(&rdap, "a.test", now()).await;
    let second = observe(&rdap, "a.test", now()).await;

    assert_eq!(first.judgement, Judgement::Unjudged);
    assert_eq!(first.message, Some(Failure::Status(503).to_string()));
    assert_eq!(second, first);
    assert_eq!(server.requests_for("/dns.json"), 2);
}

#[tokio::test]
async fn d_29_a_registry_that_cannot_be_read_gives_unknown_with_a_fixed_message() {
    let cases: [(RdapAnswer, Failure); 4] = [
        (
            |_| reply("200 OK", br#"{"events": []}"#),
            Failure::NoExpiryDate,
        ),
        (|_| reply("200 OK", b"<html>"), Failure::RdapUnreadable),
        (
            |_| reply("500 Internal Server Error", b""),
            Failure::Status(500),
        ),
        (
            |_| reply("200 OK", &vec![b' '; 256 * 1024 + 1]),
            Failure::RdapTooLong,
        ),
    ];
    for (answer, failure) in cases {
        let server = registry(answer).await;

        let result = observe(&rdap(&server), "a.test", now()).await;

        assert_eq!(result.judgement, Judgement::Unjudged, "{failure}");
        assert_eq!(result.message, Some(failure.to_string()));
        assert_eq!(result.domain_not_after, None);
    }
}

#[tokio::test]
async fn d_29_a_domain_the_registry_does_not_know_fails() {
    let server = registry(|_| reply("404 Not Found", br#"{"errorCode": 404}"#)).await;

    let result = observe(&rdap(&server), "gone.test", now()).await;

    assert!(!result.ok);
    assert_eq!(result.judgement, Judgement::Answered);
    assert_eq!(result.message, Some(Failure::DomainNotFound.to_string()));
}

#[tokio::test]
async fn d_29_a_tld_without_a_service_is_unknown_and_asks_nothing_more() {
    let server = registry(expiring).await;

    let result = observe(&rdap(&server), "example.org", now()).await;

    assert_eq!(result.judgement, Judgement::Unjudged);
    assert_eq!(result.message, Some(Failure::NoRdapService.to_string()));
    assert_eq!(server.lines(), ["GET /dns.json HTTP/1.1"]);
}

#[tokio::test]
async fn p3_003_a_registry_at_a_refused_address_is_never_reached() {
    let server = registry(expiring).await;
    let rdap = Rdap::for_tests(
        AddressPolicy::default(),
        server.root.clone(),
        server.url("/dns.json"),
    );

    let result = observe(&rdap, "a.test", now()).await;

    assert_eq!(result.judgement, Judgement::Unjudged);
    assert_eq!(result.message, Some(Failure::NotPermitted.to_string()));
    assert_eq!(server.connections().await, 0);
}

#[tokio::test]
async fn d_29_the_time_limit_bounds_the_whole_probe() {
    let server = registry(expiring).await;
    let silent = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = silent.local_addr().unwrap().port();
    let rdap = Rdap::for_tests(
        AddressPolicy::default().allowing_loopback(),
        server.root.clone(),
        reqwest::Url::parse(&format!("https://localhost:{port}/dns.json")).unwrap(),
    );
    let lookup = Lookup {
        domain: "a.test",
        warn_days: 30,
        timeout: Duration::from_millis(300),
    };

    let started = std::time::Instant::now();
    let result = rdap.observe(lookup, now()).await;

    assert!(started.elapsed() < Duration::from_secs(5));
    assert_eq!(result.judgement, Judgement::Unjudged);
    assert_eq!(result.message, Some(Failure::TimedOut.to_string()));
    drop(silent);
}
