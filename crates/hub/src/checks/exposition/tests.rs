//! Unit tests of the exposition's pure parts: the renderer, the label values and the cache
//! (INT-001, INT-002, D-24). The route against the database is tested in
//! `tests/it/checks/exposition`.

use std::sync::atomic::{AtomicUsize, Ordering};

use chrono::{DateTime, TimeDelta, Utc};
use proptest::prelude::*;
use uuid::Uuid;

use super::{
    cache::{Cache, FRESH_FOR},
    labels::{LABEL_MAX, escaped, label_value},
    model::{CheckSample, DeviceLabels, DeviceSample, Snapshot},
    render::{render, status_value},
};
use crate::{
    checks::config::CheckType,
    status::Status::{self, Degraded, Down, Maintenance, Unknown, Up},
};

const FAMILIES: [&str; 7] = [
    "anima_check_status",
    "anima_check_up",
    "anima_check_response_time_seconds",
    "anima_check_uptime_ratio",
    "anima_check_certificate_days_left",
    "anima_check_domain_days_left",
    "anima_device_status",
];

fn device() -> DeviceLabels {
    DeviceLabels {
        id: Uuid::from_u128(0xd1).into(),
        name: "pve-01".to_owned(),
        site: Uuid::from_u128(0x51).into(),
    }
}

fn check(name: &str, status: Status) -> CheckSample {
    CheckSample {
        id: Uuid::from_u128(0xc1).into(),
        name: name.to_owned(),
        kind: CheckType::Tls,
        device: device(),
        status,
        response_time_ms: None,
        uptime: [None; 3],
        certificate_days_left: None,
        domain_days_left: None,
    }
}

fn one(check: CheckSample) -> Snapshot {
    Snapshot {
        checks: vec![check],
        devices: vec![DeviceSample {
            device: device(),
            status: Down,
        }],
    }
}

/// The sample lines of one family.
fn samples<'a>(text: &'a str, family: &str) -> Vec<&'a str> {
    text.lines()
        .filter(|line| {
            line.strip_prefix(family)
                .is_some_and(|rest| rest.starts_with('{'))
        })
        .collect()
}

const CHECK_LABELS: &str = "check_id=\"00000000-0000-0000-0000-0000000000c1\",\
    check_name=\"TLS\",check_type=\"tls\",device_id=\"00000000-0000-0000-0000-0000000000d1\",\
    device_name=\"pve-01\",site=\"00000000-0000-0000-0000-000000000051\"";

#[test]
fn int_001_renders_every_family_with_help_type_and_values() {
    let mut sample = check("TLS", Degraded);
    sample.response_time_ms = Some(12);
    sample.uptime = [Some(1.0), None, Some(0.5)];
    sample.certificate_days_left = Some(-3);
    sample.domain_days_left = Some(40);

    let text = render(&one(sample));

    let device_labels = "device_id=\"00000000-0000-0000-0000-0000000000d1\",\
        device_name=\"pve-01\",site=\"00000000-0000-0000-0000-000000000051\"";
    let expected = [
        format!("anima_check_status{{{CHECK_LABELS}}} 2"),
        format!("anima_check_up{{{CHECK_LABELS}}} 1"),
        format!("anima_check_response_time_seconds{{{CHECK_LABELS}}} 0.012"),
        format!("anima_check_uptime_ratio{{{CHECK_LABELS},window=\"1d\"}} 1"),
        format!("anima_check_uptime_ratio{{{CHECK_LABELS},window=\"30d\"}} 0.5"),
        format!("anima_check_certificate_days_left{{{CHECK_LABELS}}} -3"),
        format!("anima_check_domain_days_left{{{CHECK_LABELS}}} 40"),
        format!("anima_device_status{{{device_labels}}} 0"),
    ];
    let lines: Vec<&str> = text.lines().filter(|line| !line.starts_with('#')).collect();
    assert_eq!(lines, expected);
    for family in FAMILIES {
        assert!(text.contains(&format!("# HELP {family} ")), "{family}");
        assert!(
            text.contains(&format!("# TYPE {family} gauge\n")),
            "{family}"
        );
    }
    assert!(text.ends_with('\n'));
}

#[test]
fn int_001_status_values_follow_k7_and_paused_is_unknown() {
    let expected = [
        (Down, 0, 0),
        (Up, 1, 1),
        (Degraded, 2, 1),
        (Maintenance, 3, 0),
        (Unknown, 4, 0),
    ];
    for (status, value, up) in expected {
        assert_eq!(status_value(status), value);
        let text = render(&one(check("TLS", status)));
        assert_eq!(
            samples(&text, "anima_check_status"),
            [format!("anima_check_status{{{CHECK_LABELS}}} {value}")]
        );
        assert_eq!(
            samples(&text, "anima_check_up"),
            [format!("anima_check_up{{{CHECK_LABELS}}} {up}")]
        );
    }
}

#[test]
fn int_001_optional_series_are_absent_without_a_value() {
    let text = render(&one(check("TLS", Up)));

    for family in [
        "anima_check_response_time_seconds",
        "anima_check_uptime_ratio",
        "anima_check_certificate_days_left",
        "anima_check_domain_days_left",
    ] {
        assert!(samples(&text, family).is_empty(), "{family}");
        assert!(
            text.contains(&format!("# TYPE {family} gauge\n")),
            "{family}"
        );
    }
}

#[test]
fn int_001_an_empty_hub_gives_only_help_and_type_lines() {
    let text = render(&Snapshot::default());

    assert_eq!(text.lines().count(), 2 * FAMILIES.len());
    assert!(text.lines().all(|line| line.starts_with("# ")));
}

#[test]
fn int_002_escaping_covers_backslash_quote_and_line_feed() {
    assert_eq!(escaped(r#"a\b"c"#), r#"a\\b\"c"#);
    assert_eq!(escaped("one\ntwo"), r"one\ntwo");
    assert_eq!(escaped("plain } { = ,"), "plain } { = ,");
}

#[test]
fn int_002_hostile_label_values_cannot_break_out() {
    let hostile = "web\"} anima_check_up{check_id=\"x\"} 1\n# TYPE x counter\r\\";
    assert_eq!(
        label_value(hostile),
        r#"web\"} anima_check_up{check_id=\"x\"} 1# TYPE x counter\\"#
    );
    let invisible = "a\u{202E}b\u{200B}c\u{0007}d\u{2028}e\u{FEFF}f\u{0000}g";
    assert_eq!(label_value(invisible), "abcdefg");
    assert_eq!(label_value("Zürich ✓ 東京"), "Zürich ✓ 東京");
    assert_eq!(
        label_value(&"é".repeat(LABEL_MAX + 50)).chars().count(),
        LABEL_MAX
    );
}

/// Reads one quoted label value from the start of `text`, undoing the escapes. Returns the
/// value and the rest after the closing quote, or `None` when the quoting is broken.
fn unquote(text: &str) -> Option<(String, &str)> {
    let mut chars = text.strip_prefix('"')?.char_indices();
    let mut value = String::new();
    while let Some((index, c)) = chars.next() {
        match c {
            '"' => return Some((value, &text[index + 2..])),
            '\\' => match chars.next()?.1 {
                'n' => value.push('\n'),
                escaped @ ('\\' | '"') => value.push(escaped),
                _ => return None,
            },
            '\n' => return None,
            other => value.push(other),
        }
    }
    None
}

proptest! {
    /// A check name of any characters renders as one well-formed status line whose
    /// `check_name` reads back as the cleaned name (INT-002).
    #[test]
    fn int_002_any_check_name_renders_one_well_formed_line(name in any::<String>()) {
        let text = render(&one(check(&name, Up)));
        let lines = samples(&text, "anima_check_status");
        prop_assert_eq!(lines.len(), 1);
        let after = lines[0].split_once(",check_name=").map(|(_, rest)| rest);
        let (value, rest) = unquote(after.unwrap_or_default()).expect("quoted value");
        prop_assert!(rest.starts_with(",check_type=\"tls\","));
        let closed = lines[0].ends_with("} 1");
        prop_assert!(closed);
        prop_assert!(value.chars().count() <= LABEL_MAX);
        prop_assert!(!value.chars().any(char::is_control));
        prop_assert_eq!(text.lines().count(), 2 * FAMILIES.len() + 3);
    }
}

fn at(seconds: i64) -> DateTime<Utc> {
    DateTime::from_timestamp(1_800_000_000 + seconds, 0).unwrap()
}

/// A build that counts its runs and names the run in its text.
async fn counted(builds: &AtomicUsize) -> Option<String> {
    let run = builds.fetch_add(1, Ordering::SeqCst) + 1;
    Some(format!("build {run}"))
}

#[tokio::test]
async fn d_24_the_body_is_kept_for_ten_seconds() {
    let (cache, builds) = (Cache::default(), AtomicUsize::new(0));
    let fresh_until = TimeDelta::seconds(FRESH_FOR.num_seconds() - 1);

    let first = cache.body(at(0), || counted(&builds)).await;
    let within = cache.body(at(0) + fresh_until, || counted(&builds)).await;
    let after = cache.body(at(0) + FRESH_FOR, || counted(&builds)).await;

    assert_eq!(first.as_deref(), Some(&b"build 1"[..]));
    assert_eq!(within, first);
    assert_eq!(after.as_deref(), Some(&b"build 2"[..]));
    assert_eq!(builds.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn d_24_a_failed_build_is_not_kept() {
    let (cache, builds) = (Cache::default(), AtomicUsize::new(0));

    let failed = cache
        .body(at(0), || async {
            builds.fetch_add(1, Ordering::SeqCst);
            None
        })
        .await;
    let next = cache.body(at(1), || counted(&builds)).await;

    assert_eq!(failed, None);
    assert_eq!(next.as_deref(), Some(&b"build 2"[..]));
}

#[tokio::test]
async fn d_24_scrapes_during_a_build_share_it() {
    let (cache, builds) = (Cache::default(), AtomicUsize::new(0));
    let (release, held) = tokio::sync::oneshot::channel::<()>();
    let slow = || async {
        held.await.unwrap();
        counted(&builds).await
    };

    let (first, second, ()) = tokio::join!(
        cache.body(at(0), slow),
        cache.body(at(1), || counted(&builds)),
        async { release.send(()).unwrap() },
    );

    assert_eq!(builds.load(Ordering::SeqCst), 1);
    assert_eq!(first.as_deref(), Some(&b"build 1"[..]));
    assert_eq!(second, first);
}

#[tokio::test]
async fn d_24_a_clock_moved_back_ten_seconds_rebuilds() {
    let (cache, builds) = (Cache::default(), AtomicUsize::new(0));

    cache.body(at(100), || counted(&builds)).await;
    let back = cache.body(at(100) - FRESH_FOR, || counted(&builds)).await;

    assert_eq!(back.as_deref(), Some(&b"build 2"[..]));
}
