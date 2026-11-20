use super::*;

fn read(source: Source, lines: &[&str]) -> Thread {
    let mut reader = ThreadReader::new(source);
    for line in lines {
        reader.feed(line.as_bytes());
    }
    reader.finish()
}

fn kinds(thread: &Thread) -> Vec<&'static str> {
    thread
        .entries
        .iter()
        .map(|entry| match entry.kind {
            EntryKind::Prompt { .. } => "prompt",
            EntryKind::Reply { .. } => "reply",
            EntryKind::Reasoning { .. } => "reasoning",
            EntryKind::Tool(_) => "tool",
            EntryKind::TurnEnd { .. } => "end",
            EntryKind::Compacted => "compacted",
        })
        .collect()
}

fn tool(thread: &Thread) -> &ToolCall {
    thread
        .entries
        .iter()
        .find_map(|entry| match &entry.kind {
            EntryKind::Tool(tool) => Some(tool),
            _ => None,
        })
        .expect("a tool call")
}

const CLAUDE: [&str; 7] = [
    r#"{"type":"user","timestamp":"2026-11-20T09:00:00Z","cwd":"/w/anima","gitBranch":"dev","message":{"role":"user","content":"Fix the login test\nplease"}}"#,
    r#"{"type":"user","timestamp":"2026-11-20T09:00:01Z","isMeta":true,"message":{"content":"<system-reminder>x</system-reminder>"}}"#,
    r#"{"type":"assistant","timestamp":"2026-11-20T09:00:02Z","message":{"model":"claude-opus-5-5","content":[{"type":"thinking","thinking":"Look at the test first."}]}}"#,
    r#"{"type":"assistant","timestamp":"2026-11-20T09:00:03Z","message":{"model":"claude-opus-5-5","content":[{"type":"tool_use","id":"t1","name":"Edit","input":{"file_path":"src/a.rs","old_string":"a\nb","new_string":"c"}}]}}"#,
    r#"{"type":"user","timestamp":"2026-11-20T09:00:04Z","message":{"content":[{"type":"tool_result","tool_use_id":"t1","content":"done","is_error":false}]}}"#,
    r#"{"type":"assistant","timestamp":"2026-11-20T09:00:05Z","isSidechain":true,"message":{"content":[{"type":"text","text":"sub-agent text"}]}}"#,
    r#"{"type":"assistant","timestamp":"2026-11-20T09:00:06Z","message":{"content":[{"type":"text","text":"Fixed."}]}}"#,
];

#[test]
fn thr_003_a_claude_code_file_reads_as_a_thread_in_order() {
    let thread = read(Source::ClaudeCode, &CLAUDE);
    assert_eq!(kinds(&thread), ["prompt", "reasoning", "tool", "reply"]);
    assert_eq!(thread.title.as_deref(), Some("Fix the login test"));
    assert_eq!(thread.folder.as_deref(), Some("/w/anima"));
    assert_eq!(thread.branch.as_deref(), Some("dev"));
    assert_eq!(thread.model.as_deref(), Some("claude-opus-5-5"));
    assert_eq!(thread.prompts, 1);
    assert_eq!(thread.last, time(Some("2026-11-20T09:00:06Z")));
}

#[test]
fn thr_005_a_tool_result_is_attached_to_its_call_with_the_patch() {
    let thread = read(Source::ClaudeCode, &CLAUDE);
    let call = tool(&thread);
    assert_eq!(
        (call.kind, call.summary.as_str()),
        (ToolKind::Edit, "src/a.rs")
    );
    assert_eq!(call.patch.as_deref(), Some("-a\n-b\n+c"));
    assert_eq!((call.output.as_deref(), call.failed), (Some("done"), false));
}

#[test]
fn thr_004_a_title_the_person_gave_wins_over_the_first_prompt() {
    let mut lines = CLAUDE.to_vec();
    lines.push(r#"{"type":"custom-title","customTitle":"Login test","sessionId":"s"}"#);
    let thread = read(Source::ClaudeCode, &lines);
    assert_eq!(thread.title.as_deref(), Some("Login test"));
    assert!(thread.title_given);
    assert!(!read(Source::ClaudeCode, &CLAUDE).title_given);
}

#[test]
fn thr_003_text_the_program_wrote_in_the_turn_of_the_person_is_not_a_prompt() {
    let thread = read(
        Source::ClaudeCode,
        &[
            r#"{"type":"user","message":{"content":"<local-command-stdout>ok</local-command-stdout>"}}"#,
            r#"{"type":"user","message":{"content":[{"type":"text","text":"<command-name>/review</command-name>"}]}}"#,
        ],
    );
    assert_eq!(thread.prompts, 1, "only the slash command the person typed");
}

const CODEX: [&str; 8] = [
    r#"{"timestamp":"2026-11-20T10:00:00Z","type":"session_meta","payload":{"id":"019a-thread","cwd":"/w/site","git":{"branch":"main"}}}"#,
    r#"{"timestamp":"2026-11-20T10:00:00Z","type":"turn_context","payload":{"model":"gpt-5.5-codex","cwd":"/w/site"}}"#,
    r#"{"timestamp":"2026-11-20T10:00:01Z","type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"<environment_context>x</environment_context>"}]}}"#,
    r#"{"timestamp":"2026-11-20T10:00:01Z","type":"event_msg","payload":{"type":"user_message","message":"Run the tests","images":[]}}"#,
    r#"{"timestamp":"2026-11-20T10:00:02Z","type":"response_item","payload":{"type":"function_call","name":"shell","arguments":"{\"command\":[\"bun\",\"test\"]}","call_id":"c1"}}"#,
    r#"{"timestamp":"2026-11-20T10:00:03Z","type":"response_item","payload":{"type":"function_call_output","call_id":"c1","output":"{\"output\":\"1 failed\",\"metadata\":{\"exit_code\":1}}"}}"#,
    r#"{"timestamp":"2026-11-20T10:00:04Z","type":"event_msg","payload":{"type":"agent_message","message":"One test fails."}}"#,
    r#"{"timestamp":"2026-11-20T10:00:05Z","type":"event_msg","payload":{"type":"turn_aborted","reason":"interrupted"}}"#,
];

#[test]
fn thr_003_a_codex_rollout_reads_as_a_thread_in_order() {
    let thread = read(Source::Codex, &CODEX);
    assert_eq!(kinds(&thread), ["prompt", "tool", "reply", "end"]);
    assert_eq!(thread.id.as_deref(), Some("019a-thread"));
    assert_eq!(thread.title.as_deref(), Some("Run the tests"));
    assert_eq!(thread.branch.as_deref(), Some("main"));
    assert_eq!(thread.model.as_deref(), Some("gpt-5.5-codex"));
    assert!(matches!(
        thread.entries.last().map(|entry| &entry.kind),
        Some(EntryKind::TurnEnd { stopped: true })
    ));
}

#[test]
fn thr_005_a_codex_command_that_exits_with_an_error_failed() {
    let thread = read(Source::Codex, &CODEX);
    let call = tool(&thread);
    assert_eq!(
        (call.kind, call.summary.as_str()),
        (ToolKind::Command, "bun test")
    );
    assert_eq!(
        (call.output.as_deref(), call.failed),
        (Some("1 failed"), true)
    );
}

#[test]
fn thr_005_a_codex_patch_names_its_file_and_is_its_own_patch() {
    let patch = "*** Begin Patch\n*** Update File: src/b.ts\n-x\n+y\n*** End Patch";
    let line = serde_json::json!({"type":"response_item","payload":{"type":"custom_tool_call","name":"apply_patch","input":patch,"call_id":"p1"}}).to_string();
    let thread = read(Source::Codex, &[&line]);
    let call = tool(&thread);
    assert_eq!(
        (call.kind, call.summary.as_str()),
        (ToolKind::Edit, "src/b.ts")
    );
    assert_eq!(call.patch.as_deref(), Some(patch));
}

#[test]
fn thr_006_text_is_held_to_its_limit_at_a_character_boundary() {
    let text = "é".repeat(MAX_TEXT);
    let held = held(&text, MAX_TEXT);
    assert!(held.len() < MAX_TEXT + 40);
    assert!(held.ends_with(&format!("[cut: {} more bytes]", text.len() - MAX_TEXT)));
}

#[test]
fn thr_006_a_line_that_is_not_json_changes_nothing() {
    let thread = read(Source::Codex, &["not json", "{\"type\":5}"]);
    assert_eq!(thread, Thread::default());
}

#[test]
fn thr_005_text_a_command_printed_is_not_read_as_the_wrapper_of_codex() {
    let printed = r#"{"output":"All tests passed","metadata":{"exit_code":0},"note":"x"}"#;
    let call = r#"{"type":"response_item","payload":{"type":"function_call","name":"shell","arguments":"{}","call_id":"c1"}}"#;
    let output = serde_json::json!({"type":"response_item","payload":{"type":"function_call_output","call_id":"c1","output":printed}}).to_string();
    let thread = read(Source::Codex, &[call, &output]);
    assert_eq!(tool(&thread).output.as_deref(), Some(printed));
}

#[test]
fn thr_006_a_thread_keeps_a_known_size_of_text_and_counts_the_rest() {
    let reply = serde_json::json!({"type":"assistant","message":{"content":[{"type":"text","text":"x".repeat(MAX_TEXT)}]}}).to_string();
    let mut reader = ThreadReader::new(Source::ClaudeCode);
    for _ in 0..(MAX_THREAD / MAX_TEXT + 10) {
        reader.feed(reply.as_bytes());
    }
    let thread = reader.finish();
    assert_eq!(thread.entries.len(), MAX_THREAD / MAX_TEXT);
    assert_eq!(thread.dropped, 10);
}

#[test]
fn thr_004_a_title_of_the_program_is_one_line_of_80_characters_at_most() {
    let line =
        serde_json::json!({"type":"summary","summary":format!("{}\nsecond line", "t".repeat(200))})
            .to_string();
    let thread = read(Source::ClaudeCode, &[&line]);
    assert_eq!(
        thread.title.map(|title| title.chars().count()),
        Some(TITLE_LENGTH)
    );
}
