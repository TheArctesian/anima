//! The `agentUsage` member of a batch (K8 Section 4; R-135, AIS-018, AIS-019, AIS-024):
//! optional, so an older daemon's batch is taken as it was; held to fixed alphabets, counts and
//! limits; an entry of a source the hub does not know is dropped, not refused.

use chrono::NaiveDate;
use proptest::prelude::*;

use super::*;
use crate::agent_usage::{Source, UsageState};
use crate::wire::{BatchAgentUsage, BatchModel, BatchSessionDay};

/// The K8 Section 4 example of the member.
const AGENT_USAGE: &str = include_str!("k8/agent-usage.json");

fn example_usage() -> BatchAgentUsage {
    serde_json::from_str(AGENT_USAGE).unwrap()
}

fn with(change: impl FnOnce(&mut BatchAgentUsage)) -> impl FnOnce(&mut Batch) {
    move |batch: &mut Batch| {
        let mut usage = example_usage();
        change(&mut usage);
        batch.agent_usage = Some(usage);
    }
}

fn with_day(change: impl FnOnce(&mut BatchSessionDay)) -> impl FnOnce(&mut Batch) {
    with(|usage| change(&mut usage.session_days[0]))
}

fn model(name: &str) -> BatchModel {
    BatchModel {
        model: name.to_owned(),
        input_tokens: 0,
        output_tokens: 1,
        cache_read_tokens: 0,
        cache_write_tokens: 0,
    }
}

#[test]
fn ais_019_a_batch_without_agent_usage_is_taken_with_none() {
    assert!(
        !BATCH.contains("agentUsage"),
        "the example is an older daemon's"
    );
    assert_eq!(checked(|_| {}).unwrap().agent_usage, None);
}

#[test]
fn ais_019_the_k8_example_reaches_the_hub_as_sent() {
    let usage = checked(with(|_| {})).unwrap().agent_usage.unwrap();
    assert_eq!(usage.state, Some(UsageState::Reading));
    let day = &usage.session_days[0];
    assert_eq!(day.source, Source::ClaudeCode);
    assert_eq!(day.day, NaiveDate::from_ymd_opt(2026, 11, 8).unwrap());
    assert_eq!(day.repository.as_deref(), Some("github.com/example/anima"));
    assert_eq!((day.prompts, day.tool_calls), (6, 41));
    assert_eq!(day.models[0].tokens.cache_read, 2_210_450);
}

#[test]
fn ais_019_the_member_round_trips_in_the_k8_spelling() {
    let usage = example_usage();
    let written = serde_json::to_value(&usage).unwrap();
    let expected: serde_json::Value = serde_json::from_str(AGENT_USAGE).unwrap();
    assert_eq!(written, expected);
    let batch = Batch {
        agent_usage: Some(usage),
        ..example()
    };
    let text = serde_json::to_string(&batch).unwrap();
    assert!(text.contains(r#""agentUsage":{"state":"reading","sessionDays":[{"#));
}

#[test]
fn ais_019_a_source_or_state_the_hub_does_not_know_is_not_refused() {
    let usage = checked(with(|usage| {
        usage.state = "paused_by_newer".to_owned();
        let mut newer = usage.session_days[0].clone();
        newer.source = "gemini_cli".to_owned();
        usage.session_days.push(newer);
    }))
    .unwrap()
    .agent_usage
    .unwrap();
    assert_eq!(usage.state, None, "the hub keeps the state it has");
    assert_eq!(usage.session_days.len(), 1, "the unknown source is dropped");
}

#[test]
fn ais_024_a_daemon_that_cannot_read_sends_its_state_and_no_session_days() {
    for state in [UsageState::SystemUnit, UsageState::TurnedOff] {
        let usage = checked(|batch| batch.agent_usage = Some(BatchAgentUsage::new(state, &[])))
            .unwrap()
            .agent_usage
            .unwrap();
        assert_eq!(usage.state, Some(state));
        assert_malformed(
            "counts with a silent state",
            with(|usage| {
                usage.state = state.as_str().to_owned();
            }),
        );
    }
}

#[test]
fn ais_019_limits_at_their_edge_are_taken() {
    assert_accepted(
        "16 session days",
        with(|usage| {
            let first = usage.session_days[0].clone();
            usage.session_days = (0..16)
                .map(|n| BatchSessionDay {
                    id: format!("s{n}"),
                    ..first.clone()
                })
                .collect();
        }),
    );
    assert_accepted(
        "16 models",
        with_day(|day| {
            day.models = (0..16).map(|n| model(&format!("m{n}"))).collect();
        }),
    );
    assert_accepted(
        "the largest count",
        with_day(|day| day.prompts = (1 << 53) - 1),
    );
    assert_accepted("no repository", with_day(|day| day.repository = None));
    assert_accepted("no models", with_day(|day| day.models.clear()));
    assert_accepted("a 64-byte ID", with_day(|day| day.id = "a".repeat(64)));
    assert_accepted(
        "the start day",
        with_day(|day| day.day = NaiveDate::from_ymd_opt(2026, 11, 7).unwrap()),
    );
}

#[test]
fn ais_019_a_member_that_breaks_a_list_rule_is_malformed() {
    assert_malformed(
        "17 session days",
        with(|usage| {
            let first = usage.session_days[0].clone();
            usage.session_days = (0..17)
                .map(|n| BatchSessionDay {
                    id: format!("s{n}"),
                    ..first.clone()
                })
                .collect();
        }),
    );
    assert_malformed(
        "two equal keys",
        with(|usage| usage.session_days.push(usage.session_days[0].clone())),
    );
    assert_malformed(
        "a capital",
        with(|usage| usage.state = "Reading".to_owned()),
    );
    assert_malformed("an empty state", with(|usage| usage.state = String::new()));
    let models = |count: usize| (0..count).map(|n| model(&format!("m{n}"))).collect();
    assert_malformed("17 models", with_day(|day| day.models = models(17)));
    let twice = vec![model("m"), model("m")];
    assert_malformed("two equal models", with_day(|day| day.models = twice));
}

#[test]
fn ais_019_a_session_day_that_breaks_a_text_rule_is_malformed() {
    let source = |text: &str| {
        let text = text.to_owned();
        with_day(move |day| day.source = text)
    };
    assert_malformed("a source with a dash", source("claude-code"));
    for id in ["../x", &"a".repeat(65), ""] {
        let id = id.to_owned();
        assert_malformed("an ID", with_day(move |day| day.id = id));
    }
    for name in ["/Users/me/app", "me:pw@github.com/a/b", "github.com/app"] {
        let name = name.to_owned();
        assert_malformed(
            "a repository",
            with_day(move |day| day.repository = Some(name)),
        );
    }
    assert_malformed(
        "a model with a space",
        with_day(|day| day.models = vec![model("gpt 5")]),
    );
}

#[test]
fn ais_019_a_session_day_that_breaks_a_count_or_time_rule_is_malformed() {
    assert_malformed(
        "the end before the start",
        with_day(|day| std::mem::swap(&mut day.started_at, &mut day.ended_at)),
    );
    assert_malformed(
        "a day after the end",
        with_day(|day| day.day = NaiveDate::from_ymd_opt(2026, 11, 9).unwrap()),
    );
    assert_malformed(
        "a count past 2^53 - 1",
        with_day(|day| day.tool_calls = 1 << 53),
    );
    assert_malformed(
        "tokens past 2^53 - 1",
        with_day(|day| day.models[0].cache_read_tokens = 1 << 53),
    );
}

#[test]
fn ais_019_a_day_or_time_not_spelled_as_k8_is_not_a_batch() {
    for (from, to) in [
        (r#""day": "2026-11-08""#, r#""day": "2026-11-8""#),
        (r#""day": "2026-11-08""#, r#""day": "2026-02-30""#),
        (
            r#""startedAt": "2026-11-07T22:14:03Z""#,
            r#""startedAt": "2026-11-07T22:14:03.5Z""#,
        ),
        (r#""prompts": 6"#, r#""prompts": -6"#),
        (r#""prompts": 6"#, r#""prompts": 6.5"#),
    ] {
        let text = AGENT_USAGE.replace(from, to);
        assert_ne!(text, AGENT_USAGE, "{to}");
        assert!(
            serde_json::from_str::<BatchAgentUsage>(&text).is_err(),
            "{to}"
        );
    }
}

proptest! {
    /// Any text in a session day's fields gives a verdict, never a panic; an accepted one
    /// keeps the K8 alphabets.
    #[test]
    fn ais_019_any_texts_are_checked(source in "\\PC{0,40}", id in "\\PC{0,70}", repository in "\\PC{0,60}", name in "\\PC{0,70}") {
        let mut usage = example_usage();
        let day = &mut usage.session_days[0];
        day.source = source;
        day.id = id;
        day.repository = Some(repository);
        day.models[0].model = name;
        if let Some(accepted) = usage.accepted() {
            for day in accepted.session_days {
                prop_assert!(crate::agent_usage::is_session_id(&day.id));
                prop_assert!(day.repository.as_deref().is_none_or(crate::agent_usage::is_repository));
            }
        }
    }
}
