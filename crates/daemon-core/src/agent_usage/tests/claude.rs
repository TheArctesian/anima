//! The Claude Code reader (AIS-018, AIS-022): prompts the operator typed, tool calls, the tokens
//! of each reply once, sub-agent lines without their prompts; nothing of the text kept.

use proptest::prelude::*;

use super::{MARKER, assert_no_marker, days_of, tally};
use crate::agent_usage::{Source, Tokens};

/// A synthetic session: a title, a typed prompt, a reminder the program wrote, a reply in two
/// lines (one usage), a tool's result, a sub-agent's prompt and reply, a meta line, a
/// synthetic reply, and lines that are not JSON or not of a known form.
const SESSION: &str = r#"{"type":"custom-title","customTitle":"MARKER-7f3a9c-SECRET title","sessionId":"s1"}
{"type":"user","timestamp":"2026-11-07T23:59:58.120Z","cwd":"/Users/me/MARKER-7f3a9c-SECRET/app","isSidechain":false,"message":{"role":"user","content":"Fix the MARKER-7f3a9c-SECRET bug"}}
{"type":"user","timestamp":"2026-11-07T23:59:59Z","cwd":"/elsewhere","message":{"role":"user","content":"<system-reminder>MARKER-7f3a9c-SECRET</system-reminder>"}}
{"type":"assistant","timestamp":"2026-11-08T00:00:01.500Z","message":{"id":"msg_1","model":"claude-opus-5-5","content":[{"type":"thinking","thinking":"MARKER-7f3a9c-SECRET"}],"usage":{"input_tokens":10,"output_tokens":200,"cache_read_input_tokens":3000,"cache_creation_input_tokens":40}}}
{"type":"assistant","timestamp":"2026-11-08T00:00:02Z","message":{"id":"msg_1","model":"claude-opus-5-5","content":[{"type":"tool_use","id":"t1","name":"Bash","input":{"command":"cat MARKER-7f3a9c-SECRET"}}],"usage":{"input_tokens":10,"output_tokens":200,"cache_read_input_tokens":3000,"cache_creation_input_tokens":40}}}
{"type":"user","timestamp":"2026-11-08T00:00:03Z","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"t1","content":"MARKER-7f3a9c-SECRET"}]},"toolUseResult":{"stdout":"MARKER-7f3a9c-SECRET"}}
{"type":"user","timestamp":"2026-11-08T00:00:04Z","isSidechain":true,"message":{"role":"user","content":"Sub-agent task MARKER-7f3a9c-SECRET"}}
{"type":"assistant","timestamp":"2026-11-08T00:00:05Z","isSidechain":true,"message":{"id":"msg_2","model":"claude-haiku-4-5","content":[{"type":"tool_use","name":"Read","input":{}},{"type":"text","text":"MARKER-7f3a9c-SECRET"}],"usage":{"input_tokens":5,"output_tokens":7}}}
{"type":"user","timestamp":"2026-11-08T00:00:06Z","isMeta":true,"message":{"role":"user","content":"MARKER-7f3a9c-SECRET"}}
{"type":"assistant","timestamp":"2026-11-08T00:00:07Z","message":{"id":"msg_3","model":"<synthetic>","content":[{"type":"text","text":"MARKER-7f3a9c-SECRET"}],"usage":{"input_tokens":0,"output_tokens":0}}}
{"type":"user","timestamp":"2026-11-08T00:00:08Z","message":{"role":"user","content":[{"type":"text","text":"  A second MARKER-7f3a9c-SECRET prompt"},{"type":"image","source":{}}]}}
{"type":"user","timestamp":"2026-11-08T00:00:09Z","message":{"role":"user","content":"<command-name>/review</command-name>"}}
{"type":"last-prompt","lastPrompt":"MARKER-7f3a9c-SECRET"}
not json MARKER-7f3a9c-SECRET
{"type":"assistant","timestamp":"yesterday","message":{"id":"msg_4","model":"claude-opus-5-5","usage":{"output_tokens":99}}}
{"type":"assistant","timestamp":"2026-11-08T00:00:10Z","isSidechain":"MARKER-7f3a9c-SECRET","message":{"id":"msg_5","model":"x","usage":{"output_tokens":5}}}
{"type":"assistant","timestamp":"2026-11-08T00:00:11Z","message":{"id":"msg_6","model":"MARKER/7f3a9c SECRET","usage":{"output_tokens":1}}}
"#;

#[test]
fn ais_018_a_claude_session_gives_prompts_tool_calls_and_tokens_by_day_and_model() {
    let tally = tally(Source::ClaudeCode, SESSION);
    let days = days_of(&tally, "s1");
    assert_eq!(days.len(), 2, "the first prompt is on the day before");
    let (before, after) = (&days[0], &days[1]);
    assert_eq!(before.day.to_string(), "2026-11-07");
    assert_eq!((before.prompts, before.tool_calls), (1, 0));
    assert_eq!(before.models.len(), 0);
    assert_eq!(
        after.prompts, 2,
        "a second typed prompt and a slash command"
    );
    assert_eq!(
        after.tool_calls, 2,
        "the main reply's call and the sub-agent's"
    );
    let names: Vec<&str> = after.models.iter().map(|one| one.model.as_str()).collect();
    assert_eq!(names, ["claude-haiku-4-5", "claude-opus-5-5", "other"]);
    let opus = Tokens {
        input: 10,
        output: 200,
        cache_read: 3000,
        cache_write: 40,
    };
    assert_eq!(
        after.models[1].tokens, opus,
        "one reply, two lines, counted once"
    );
    assert_eq!(
        after.models[2].tokens.output, 1,
        "a name that breaks the rule"
    );
    assert_eq!(after.started_at.to_rfc3339(), "2026-11-07T23:59:58+00:00");
    assert_eq!(after.ended_at.to_rfc3339(), "2026-11-08T00:00:11+00:00");
    assert_eq!(
        before.started_at, after.started_at,
        "the whole session's span"
    );
}

#[test]
fn ais_018_no_text_of_a_claude_session_is_kept_or_sent() {
    let tally = tally(Source::ClaudeCode, SESSION);
    assert_no_marker(&tally, "s1");
    assert_eq!(
        tally.cwd(),
        Some(format!("/Users/me/{MARKER}/app").as_str()),
        "the first folder, kept for the daemon only"
    );
}

#[test]
fn ais_018_a_file_read_in_two_parts_counts_as_one_read() {
    let lines: Vec<&str> = SESSION.lines().collect();
    let whole = tally(Source::ClaudeCode, SESSION);
    let mut parts = tally(Source::ClaudeCode, &lines[..5].join("\n"));
    for line in &lines[5..] {
        parts.feed(line.as_bytes());
    }
    assert_eq!(parts.days(), whole.days());
    assert_eq!(parts.span(), whole.span());
}

#[test]
fn ais_018_each_new_reply_counts_however_many_came_before() {
    let reply = |id: u32| {
        format!(
            r#"{{"type":"assistant","timestamp":"2026-11-08T00:00:00Z","message":{{"id":"m{id}","model":"m","usage":{{"output_tokens":1}}}}}}"#
        )
    };
    let lines: Vec<String> = (0..40).map(reply).collect();
    let tally = tally(Source::ClaudeCode, &lines.join("\n"));
    let output = |tally: &crate::agent_usage::Tally| {
        tally.days().values().next().expect("a day").models["m"].output
    };
    assert_eq!(output(&tally), 40);
}

proptest! {
    /// Any bytes are skipped or counted, never a panic.
    #[test]
    fn ais_018_the_claude_reader_takes_any_line(line in proptest::collection::vec(any::<u8>(), 0..256)) {
        let mut tally = crate::agent_usage::Tally::new(Source::ClaudeCode);
        tally.feed(&line);
    }

    /// Lines of the right form with any values never panic or keep their text.
    #[test]
    fn ais_018_the_claude_reader_takes_any_values(
        kind in "user|assistant|x",
        text in "\\PC{0,40}",
        tokens in any::<u64>(),
    ) {
        let line = serde_json::json!({
            "type": kind, "timestamp": "2026-11-08T00:00:00Z",
            "message": { "id": text, "model": text, "content": text,
                "usage": { "output_tokens": tokens, "input_tokens": tokens } }
        });
        let mut tally = crate::agent_usage::Tally::new(Source::ClaudeCode);
        tally.feed(line.to_string().as_bytes());
        tally.feed(line.to_string().as_bytes());
        for day in tally.days().values() {
            prop_assert!(day.models.values().all(|tokens| tokens.fits()));
            prop_assert!(day.models.keys().all(|model| crate::agent_usage::is_model(model)));
        }
    }
}
