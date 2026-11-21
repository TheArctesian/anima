//! The account of a program stays with the runner: never in an update of a thread (THR-051,
//! THR-132).

use agent_threads::drive::{AccountFacts, Limit};

use super::super::accounts::Accounts;
use super::*;

fn accounts_of(steps: &[Step]) -> Vec<&AccountFacts> {
    steps
        .iter()
        .filter_map(|step| match step {
            Step::Account(facts) => Some(facts.as_ref()),
            _ => None,
        })
        .collect()
}

#[test]
fn thr_132_codex_is_asked_for_its_account_and_limits_after_it_starts() {
    let mut thread = codex();
    thread.begin("go");
    let sent = writes(&thread.line(r#"{"id":1,"result":{}}"#, &mut counter()));
    let methods: Vec<&str> = sent
        .iter()
        .filter_map(|line| line["method"].as_str())
        .collect();
    assert_eq!(
        methods,
        [
            "initialized",
            "thread/start",
            "account/read",
            "account/rateLimits/read"
        ]
    );
}

#[test]
fn thr_051_an_account_report_is_a_step_for_the_runner_and_no_update() {
    let mut thread = codex();
    let mut ids = counter();
    thread.begin("go");
    thread.line(r#"{"id":1,"result":{}}"#, &mut ids);
    let steps = thread.line(
        r#"{"id":3,"result":{"account":{"type":"chatgpt","email":"someone@example.com","planType":"pro"},"requiresOpenaiAuth":true}}"#,
        &mut ids,
    );
    assert_eq!(accounts_of(&steps).len(), 1);
    let updates: Vec<&Change> = steps
        .iter()
        .filter_map(|step| match step {
            Step::Change(change) => Some(change),
            _ => None,
        })
        .collect();
    let shown = serde_json::to_string(&updates).expect("updates serialise");
    assert!(!shown.contains("someone"), "{shown}");
    let limits = thread.line(
        r#"{"method":"account/rateLimits/updated","params":{"rateLimits":{"primary":{"usedPercent":5,"windowDurationMins":300}}}}"#,
        &mut ids,
    );
    assert_eq!(accounts_of(&limits)[0].limits[0].used_percent, Some(5));
    assert!(
        !limits
            .iter()
            .any(|step| matches!(step, Step::Change(Change::Said { .. })))
    );
}

#[test]
fn thr_132_the_runner_keeps_the_newest_report_of_each_program() {
    let at = |seconds| chrono::DateTime::from_timestamp(seconds, 0).expect("a time");
    let limit = |used: u8| Limit {
        window: "seven_day".into(),
        used_percent: Some(used),
        resets_at: None,
        reached: false,
    };
    let mut accounts = Accounts::default();
    let plan = AccountFacts {
        plan: Some("pro".into()),
        ..AccountFacts::default()
    };
    accounts.note(Program::Codex, plan, at(1));
    let first = AccountFacts {
        limits: vec![limit(10)],
        ..AccountFacts::default()
    };
    accounts.note(Program::ClaudeCode, first, at(2));
    let second = AccountFacts {
        limits: vec![limit(20)],
        ..AccountFacts::default()
    };
    accounts.note(Program::ClaudeCode, second, at(3));
    let rows = accounts.rows();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].facts.plan.as_deref(), Some("pro"));
    assert_eq!(rows[1].facts.limits, [limit(20)]);
    assert_eq!(rows[1].at, at(3));
}
