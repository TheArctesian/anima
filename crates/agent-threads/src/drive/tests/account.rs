//! The account, the plan and the limits that the programs say (THR-051, THR-132).

use super::*;

fn facts(events: &[DriveEvent]) -> Vec<&AccountFacts> {
    events
        .iter()
        .filter_map(|event| match event {
            DriveEvent::Account { facts } => Some(facts.as_ref()),
            _ => None,
        })
        .collect()
}

#[test]
fn thr_132_the_answer_to_account_read_gives_the_account_and_the_reply_keeps_only_ids() {
    let events = codex(
        r#"{"id":3,"result":{"account":{"type":"chatgpt","email":"someone@example.com","planType":"pro"},"userId":"user-9","requiresOpenaiAuth":true,"accessToken":"TOKEN-VALUE"}}"#,
    );
    assert!(
        matches!(&events[0], DriveEvent::Reply { result, .. } if result.as_object().is_some_and(serde_json::Map::is_empty))
    );
    let found = facts(&events);
    assert_eq!(found.len(), 1);
    assert_eq!(
        found[0].name.as_ref().map(Private::reveal),
        Some("someone@example.com")
    );
    assert_eq!(found[0].id.as_ref().map(Private::reveal), Some("user-9"));
    assert_eq!(found[0].plan.as_deref(), Some("pro"));
    let shown = serde_json::to_string(&events).expect("events serialise");
    assert!(
        !shown.contains("TOKEN"),
        "no field of a token is read: {shown}"
    );
}

#[test]
fn log_007_the_debug_text_of_an_account_event_holds_no_name() {
    let events = codex(
        r#"{"id":3,"result":{"account":{"email":"someone@example.com","planType":"plus"},"userId":"user-9","requiresOpenaiAuth":true}}"#,
    );
    let debug = format!("{events:?}");
    assert!(
        !debug.contains("someone") && !debug.contains("user-9") && !debug.contains("plus"),
        "{debug}"
    );
}

#[test]
fn thr_132_a_codex_limit_notice_gives_its_two_windows() {
    let events = codex(
        r#"{"method":"account/rateLimits/updated","params":{"rateLimits":{"planType":"plus","primary":{"usedPercent":42,"windowDurationMins":300,"resetsAt":1795600000},"secondary":{"usedPercent":100,"windowDurationMins":10080,"resetsAt":null},"rateLimitReachedType":"rate_limit_reached"}}}"#,
    );
    let found = facts(&events);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].plan.as_deref(), Some("plus"));
    assert_eq!(
        found[0].limits,
        [
            Limit {
                window: "300 min".into(),
                used_percent: Some(42),
                resets_at: Some(1_795_600_000),
                reached: false
            },
            Limit {
                window: "10080 min".into(),
                used_percent: Some(100),
                resets_at: None,
                reached: true
            },
        ]
    );
}

#[test]
fn thr_132_the_plan_of_account_updated_is_kept_and_unknown_is_not_a_plan() {
    let plan = |line: &str| {
        facts(&codex(line))
            .first()
            .and_then(|found| found.plan.clone())
    };
    assert_eq!(
        plan(r#"{"method":"account/updated","params":{"authMode":"chatgpt","planType":"team"}}"#)
            .as_deref(),
        Some("team")
    );
    assert_eq!(
        plan(
            r#"{"method":"account/updated","params":{"authMode":"chatgpt","planType":"unknown"}}"#
        ),
        None
    );
}

#[test]
fn thr_132_claude_code_gives_its_limits_and_no_account() {
    let events = claude(
        r#"{"type":"rate_limit_event","rate_limit_info":{"status":"allowed","resetsAt":1795608000,"rateLimitType":"seven_day","isUsingOverage":false,"unifiedWindows":{"seven_day":{"utilization":0.46,"resetsAt":1795608000,"status":"allowed"},"five_hour":{"utilization":1.2,"status":"rejected"}}}}"#,
    );
    let found = facts(&events);
    assert_eq!(found.len(), 1);
    assert!(found[0].name.is_none() && found[0].id.is_none() && found[0].plan.is_none());
    let mut limits = found[0].limits.clone();
    limits.sort_by(|a, b| a.window.cmp(&b.window));
    assert_eq!(
        limits,
        [
            Limit {
                window: "five_hour".into(),
                used_percent: Some(100),
                resets_at: None,
                reached: true
            },
            Limit {
                window: "seven_day".into(),
                used_percent: Some(46),
                resets_at: Some(1_795_608_000),
                reached: false
            },
        ]
    );
    assert_eq!(
        claude(r#"{"type":"rate_limit_event","rate_limit_info":{"status":"allowed"}}"#),
        []
    );
}

#[test]
fn thr_132_a_newer_report_replaces_only_what_it_gives() {
    let limit = |window: &str, used: u8| Limit {
        window: window.into(),
        used_percent: Some(used),
        resets_at: None,
        reached: false,
    };
    let mut kept = facts(&codex(
        r#"{"id":3,"result":{"account":{"email":"a@example.com","planType":"pro"},"requiresOpenaiAuth":true}}"#,
    ))[0]
    .clone();
    kept.merge(AccountFacts {
        limits: vec![limit("300 min", 10), limit("10080 min", 5)],
        ..AccountFacts::default()
    });
    kept.merge(AccountFacts {
        plan: Some("plus".into()),
        limits: vec![limit("300 min", 60)],
        ..AccountFacts::default()
    });
    assert_eq!(
        kept.name.as_ref().map(Private::reveal),
        Some("a@example.com")
    );
    assert_eq!(kept.plan.as_deref(), Some("plus"));
    assert_eq!(kept.limits, [limit("10080 min", 5), limit("300 min", 60)]);
}

#[test]
fn thr_132_a_codex_sign_out_clears_the_account() {
    let mut kept = facts(&codex(
        r#"{"id":3,"result":{"account":{"email":"a@example.com","planType":"pro"},"requiresOpenaiAuth":true}}"#,
    ))[0]
    .clone();
    let out = facts(&codex(
        r#"{"method":"account/updated","params":{"authMode":null,"planType":null}}"#,
    ))[0]
        .clone();
    kept.merge(out);
    assert!(kept.name.is_none() && kept.plan.is_none() && kept.signed_out);
    let events = codex(r#"{"id":3,"result":{"account":null,"requiresOpenaiAuth":true}}"#);
    assert!(facts(&events)[0].signed_out);
}

#[test]
fn thr_132_claude_warning_is_not_reached_and_a_report_without_windows_still_counts() {
    let events = claude(
        r#"{"type":"rate_limit_event","rate_limit_info":{"unifiedWindows":{"five_hour":{"utilization":0.9,"status":"allowed_warning"}}}}"#,
    );
    assert!(!facts(&events)[0].limits[0].reached);
    let events = claude(
        r#"{"type":"rate_limit_event","rate_limit_info":{"status":"rejected","resetsAt":1795608000,"rateLimitType":"five_hour"}}"#,
    );
    assert_eq!(
        facts(&events)[0].limits,
        [Limit {
            window: "five_hour".into(),
            used_percent: None,
            resets_at: Some(1_795_608_000),
            reached: true
        }]
    );
}

#[test]
fn thr_132_a_codex_window_with_no_length_keeps_its_own_name() {
    let events = codex(
        r#"{"method":"account/rateLimits/updated","params":{"rateLimits":{"primary":{"usedPercent":1},"secondary":{"usedPercent":2}}}}"#,
    );
    let windows: Vec<&str> = facts(&events)[0]
        .limits
        .iter()
        .map(|limit| limit.window.as_str())
        .collect();
    assert_eq!(windows, ["primary", "secondary"]);
}

#[test]
fn thr_125_a_plan_with_hidden_marks_is_shown_without_them() {
    let plan = facts(&codex(
        r#"{"method":"account/updated","params":{"authMode":"chatgpt","planType":"pr\u202eo\u0007"}}"#,
    ))[0]
    .plan
    .clone();
    assert_eq!(plan.as_deref(), Some("pro"));
}
