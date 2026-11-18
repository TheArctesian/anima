//! The Codex reader (AIS-018, AIS-022): the thread and its parent, prompts of one kind, tool
//! calls, the tokens of each response once, a sub-agent's copied history not counted again;
//! the account fields of the first line never kept.

use proptest::prelude::*;

use super::{assert_no_marker, days_of, tally};
use crate::agent_usage::{Source, Tokens, root_thread};

/// A synthetic rollout: the thread's meta with account fields, a turn context, a prompt as an
/// item, an injected user message, tool calls, one token event written twice, a model change,
/// rate limits, and a line that is not JSON.
const ROLLOUT: &str = r#"{"timestamp":"2026-11-08T10:00:00.000Z","type":"session_meta","payload":{"id":"thread-1","cwd":"/work/MARKER-7f3a9c-SECRET","creator_account_id":"MARKER-7f3a9c-SECRET","base_instructions":{"text":"MARKER-7f3a9c-SECRET"},"git":{"repository_url":"https://MARKER-7f3a9c-SECRET@example.com/a/b"}}}
{"timestamp":"2026-11-08T10:00:01Z","type":"turn_context","payload":{"model":"gpt-5.1-codex","cwd":"/work/other"}}
{"timestamp":"2026-11-08T10:00:02Z","type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"<environment_context>MARKER-7f3a9c-SECRET"}]}}
{"timestamp":"2026-11-08T10:00:03Z","type":"event_msg","payload":{"type":"item_completed","item":{"type":"UserMessage","content":"MARKER-7f3a9c-SECRET"}}}
{"timestamp":"2026-11-08T10:00:04Z","type":"event_msg","payload":{"type":"user_message","message":"MARKER-7f3a9c-SECRET"}}
{"timestamp":"2026-11-08T10:00:05Z","type":"response_item","payload":{"type":"function_call","name":"shell","arguments":"MARKER-7f3a9c-SECRET"}}
{"timestamp":"2026-11-08T10:00:06Z","type":"response_item","payload":{"type":"custom_tool_call","input":"MARKER-7f3a9c-SECRET"}}
{"timestamp":"2026-11-08T10:00:07Z","type":"response_item","payload":{"type":"function_call_output","output":"MARKER-7f3a9c-SECRET"}}
{"timestamp":"2026-11-08T10:00:08Z","type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":1000,"cached_input_tokens":600,"output_tokens":50,"total_tokens":1050},"last_token_usage":{"input_tokens":1000,"cached_input_tokens":600,"cache_write_input_tokens":100,"output_tokens":50,"reasoning_output_tokens":20,"total_tokens":1050}},"rate_limits":{"primary":{"used_percent":12.0}}}}
{"timestamp":"2026-11-08T10:00:09Z","type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":1000,"cached_input_tokens":600,"output_tokens":50,"total_tokens":1050},"last_token_usage":{"input_tokens":1000,"cached_input_tokens":600,"cache_write_input_tokens":100,"output_tokens":50,"total_tokens":1050}}}}
{"timestamp":"2026-11-08T10:00:10Z","type":"event_msg","payload":{"type":"token_count","info":null,"rate_limits":{"primary":{"used_percent":13.0}}}}
{"timestamp":"2026-11-08T10:00:11Z","type":"turn_context","payload":{"model":"gpt-5.1-codex-max"}}
{"timestamp":"2026-11-08T10:00:12Z","type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":1300,"output_tokens":60},"last_token_usage":{"input_tokens":300,"output_tokens":10}}}}
{"timestamp":"2026-11-08T10:00:13Z","type":"session_meta","payload":{"id":"thread-0"}}
MARKER-7f3a9c-SECRET
"#;

#[test]
fn ais_018_a_codex_rollout_gives_prompts_tool_calls_and_tokens_by_model() {
    let tally = tally(Source::Codex, ROLLOUT);
    assert_eq!(tally.thread(), Some(("thread-1", None)), "the first meta");
    let days = days_of(&tally, "thread-1");
    assert_eq!(days.len(), 1);
    let day = &days[0];
    assert_eq!(
        day.prompts, 1,
        "items counted; the older event kind not added"
    );
    assert_eq!(day.tool_calls, 2);
    let first = Tokens {
        input: 300,
        output: 50,
        cache_read: 600,
        cache_write: 100,
    };
    assert_eq!(day.models[0].model, "gpt-5.1-codex");
    assert_eq!(
        day.models[0].tokens, first,
        "an event written twice counts once"
    );
    assert_eq!(day.models[1].model, "gpt-5.1-codex-max");
    assert_eq!(day.models[1].tokens.input, 300);
    assert_eq!(day.models[1].tokens.output, 10);
}

#[test]
fn ais_022_no_text_or_account_of_a_codex_rollout_is_kept_or_sent() {
    let tally = tally(Source::Codex, ROLLOUT);
    assert_no_marker(&tally, "thread-1");
}

#[test]
fn ais_018_an_older_codex_counts_its_user_message_events() {
    let lines = r#"{"timestamp":"2026-11-08T10:00:00Z","type":"session_meta","payload":{"id":"t"}}
{"timestamp":"2026-11-08T10:00:01Z","type":"event_msg","payload":{"type":"user_message","message":"hi"}}
{"timestamp":"2026-11-08T10:00:02Z","type":"event_msg","payload":{"type":"user_message","message":"again"}}"#;
    assert_eq!(days_of(&tally(Source::Codex, lines), "t")[0].prompts, 2);
}

#[test]
fn ais_018_a_codex_thread_takes_its_model_from_its_meta_then_its_settings() {
    let lines = r#"{"timestamp":"2026-11-08T10:00:00Z","type":"session_meta","payload":{"id":"t","base_instructions":{"text":"MARKER-7f3a9c-SECRET","provenance":{"model":"gpt-5.2"}}}}
{"timestamp":"2026-11-08T10:00:01Z","type":"event_msg","payload":{"type":"token_count","info":{"last_token_usage":{"output_tokens":3}}}}
{"timestamp":"2026-11-08T10:00:02Z","type":"event_msg","payload":{"type":"thread_settings_applied","thread_settings":{"model":"gpt-5.3","cwd":"/w"}}}
{"timestamp":"2026-11-08T10:00:03Z","type":"event_msg","payload":{"type":"token_count","info":{"last_token_usage":{"output_tokens":4}}}}"#;
    let tally = tally(Source::Codex, lines);
    let day = &days_of(&tally, "t")[0];
    let models: Vec<(&str, u64)> = day
        .models
        .iter()
        .map(|one| (one.model.as_str(), one.tokens.output))
        .collect();
    assert_eq!(models, [("gpt-5.2", 3), ("gpt-5.3", 4)]);
    assert_no_marker(&tally, "t");
}

#[test]
fn ais_018_a_codex_sub_agent_does_not_count_its_parent_s_copied_history() {
    let lines = r#"{"timestamp":"2026-11-08T11:00:00Z","ordinal":0,"type":"session_meta","payload":{"id":"child","parent_thread_id":"parent","subagent_history_start_ordinal":3}}
{"timestamp":"2026-11-08T10:00:00Z","ordinal":1,"type":"session_meta","payload":{"id":"parent"}}
{"timestamp":"2026-11-08T10:00:01Z","ordinal":2,"type":"response_item","payload":{"type":"function_call"}}
{"timestamp":"2026-11-08T10:00:02Z","ordinal":2,"type":"turn_context","payload":{"model":"gpt-5.1-codex"}}
{"timestamp":"2026-11-08T11:00:01Z","ordinal":3,"type":"response_item","payload":{"type":"function_call"}}
{"timestamp":"2026-11-08T11:00:02Z","ordinal":4,"type":"event_msg","payload":{"type":"token_count","info":{"last_token_usage":{"input_tokens":5,"output_tokens":1}}}}"#;
    let tally = tally(Source::Codex, lines);
    assert_eq!(tally.thread(), Some(("child", Some("parent"))));
    let day = &days_of(&tally, "parent")[0];
    assert_eq!(day.tool_calls, 1, "the copied call is the parent's");
    assert_eq!(
        day.models[0].model, "gpt-5.1-codex",
        "the model of the history holds"
    );
    assert_eq!(
        day.started_at.to_rfc3339(),
        "2026-11-08T11:00:01+00:00",
        "copied lines do not move the start"
    );
}

#[test]
fn ais_018_a_sub_agent_counts_under_the_root_of_its_chain() {
    let parents = [("c", "b"), ("b", "a")];
    let parent_of = |id: &str| {
        parents
            .iter()
            .find(|(child, _)| *child == id)
            .map(|(_, p)| *p)
    };
    assert_eq!(root_thread("c", parent_of), "a");
    assert_eq!(root_thread("a", parent_of), "a");
    let looped = |id: &str| Some(if id == "x" { "y" } else { "x" });
    assert!(
        ["x", "y"].contains(&root_thread("x", looped)),
        "a loop ends"
    );
}

proptest! {
    /// Any bytes are skipped or counted, never a panic.
    #[test]
    fn ais_018_the_codex_reader_takes_any_line(line in proptest::collection::vec(any::<u8>(), 0..256)) {
        let mut tally = crate::agent_usage::Tally::new(Source::Codex);
        tally.feed(&line);
    }

    /// Token events with any numbers keep every count within the K8 limit.
    #[test]
    fn ais_018_codex_tokens_stay_within_the_limit(input in any::<u64>(), cached in any::<u64>(), output in any::<u64>()) {
        let mut tally = crate::agent_usage::Tally::new(Source::Codex);
        for total in [1_u64, 2] {
            let line = serde_json::json!({
                "timestamp": "2026-11-08T10:00:00Z", "type": "event_msg",
                "payload": { "type": "token_count", "info": {
                    "total_token_usage": { "input_tokens": total },
                    "last_token_usage": { "input_tokens": input, "cached_input_tokens": cached, "output_tokens": output } } }
            });
            tally.feed(line.to_string().as_bytes());
        }
        for day in tally.days().values() {
            prop_assert!(day.models.values().all(|tokens| tokens.fits()));
        }
    }
}
