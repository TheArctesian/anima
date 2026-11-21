//! The thread core of the runner with the lines of made programs (TH11, TH15).

use std::path::PathBuf;

use agent_threads::control::{Change, Refusal, RunState, ThreadRow};
use agent_threads::drive::{Answer, Mode, Program};
use uuid::Uuid;

use super::codex::Codex;
use super::thread::{Step, Thread};

fn row(program: Program) -> ThreadRow {
    ThreadRow {
        id: Uuid::nil(),
        program,
        folder: PathBuf::from("/w"),
        mode: Mode::FullAccess,
        model: None,
        state: RunState::Starting,
        session: None,
        title: String::new(),
        started: chrono::DateTime::UNIX_EPOCH,
        ended: None,
        open: Vec::new(),
    }
}

fn claude() -> Thread {
    Thread::new(row(Program::ClaudeCode), None, None)
}

/// A Claude Code thread whose program the holder started.
fn held_claude() -> Thread {
    let mut thread = claude();
    thread.begin("go");
    thread
}

fn codex() -> Thread {
    let codex = Codex::new(PathBuf::from("/w"), Mode::FullAccess, None, None);
    Thread::new(row(Program::Codex), None, Some(codex))
}

fn counter() -> impl FnMut() -> Uuid {
    let mut next = 0_u128;
    move || {
        next += 1;
        Uuid::from_u128(next)
    }
}

fn json(line: &str) -> serde_json::Value {
    serde_json::from_str(line).expect("a JSON line")
}

fn writes(steps: &[Step]) -> Vec<serde_json::Value> {
    steps
        .iter()
        .filter_map(|step| match step {
            Step::Write(line) => Some(serde_json::from_str(line).expect("a JSON line")),
            _ => None,
        })
        .collect()
}

fn asked(steps: &[Step]) -> Vec<Uuid> {
    steps
        .iter()
        .filter_map(|step| match step {
            Step::Change(Change::Asked { request, .. }) => Some(*request),
            _ => None,
        })
        .collect()
}

const CLAUDE_INIT: &str = r#"{"type":"system","subtype":"init","session_id":"s-1","model":"m","permissionMode":"default"}"#;
const CLAUDE_ASK: &str = r#"{"type":"control_request","request_id":"r-1","request":{"subtype":"can_use_tool","tool_name":"Bash","input":{"command":"ls"},"permission_suggestions":[],"tool_use_id":"t-1"}}"#;
const CLAUDE_DELTA: &str = r#"{"type":"stream_event","event":{"type":"content_block_delta","delta":{"type":"text_delta","text":"hi"}}}"#;

#[test]
fn th11_a_claude_thread_writes_its_prompt_and_names_its_session() {
    let mut thread = claude();
    let steps = thread.begin("Look at the tests\nand more");
    assert_eq!(
        writes(&steps)[0]["message"]["content"],
        "Look at the tests\nand more"
    );
    assert_eq!(thread.row.title, "Look at the tests");
    thread.line(CLAUDE_INIT, &mut counter());
    assert_eq!(thread.row.session.as_deref(), Some("s-1"));
}

#[test]
fn thr_030_one_answer_for_each_request_and_a_second_is_refused() {
    let mut thread = held_claude();
    let request = asked(&thread.line(CLAUDE_ASK, &mut counter()))[0];
    assert_eq!(thread.row.state, RunState::NeedsYou);
    assert_eq!(thread.row.open, [request]);
    let line = thread
        .answer(request, &Answer::Allow)
        .expect("the first answer");
    assert_eq!(json(&line)["response"]["request_id"], "r-1");
    assert_eq!(
        thread.answer(request, &Answer::Allow),
        Err(Refusal::RequestGone),
        "a second answer while the first waits for the holder"
    );
    assert_eq!(thread.row.open, [request], "open until the holder took it");
    let closed = thread.answered(request);
    assert!(
        closed
            .iter()
            .any(|step| matches!(step, Step::Change(Change::Closed { .. })))
    );
    assert_eq!(thread.row.open.len(), 0);
    assert_ne!(thread.row.state, RunState::NeedsYou);
    assert_eq!(
        thread.answer(request, &Answer::Allow),
        Err(Refusal::RequestGone)
    );
    assert_eq!(
        thread.answer(Uuid::from_u128(99), &Answer::Allow),
        Err(Refusal::RequestGone),
        "an ID the runner did not make"
    );
}

#[test]
fn thr_030_an_answer_the_holder_did_not_take_can_be_given_again() {
    let mut thread = held_claude();
    let request = asked(&thread.line(CLAUDE_ASK, &mut counter()))[0];
    thread
        .answer(request, &Answer::Allow)
        .expect("the first answer");
    thread.answer_failed(request);
    assert_eq!(thread.row.open, [request]);
    assert!(thread.answer(request, &Answer::Allow).is_ok());
}

#[test]
fn thr_030_a_request_given_again_keeps_its_one_id() {
    let mut thread = held_claude();
    let mut ids = counter();
    let first = asked(&thread.line(CLAUDE_ASK, &mut ids));
    let again = asked(&thread.line(CLAUDE_ASK, &mut ids));
    assert_eq!(first.len(), 1);
    assert!(
        again.is_empty(),
        "no second ID for the same program request"
    );
    assert_eq!(thread.row.open, first);
}

#[test]
fn th11_nothing_is_written_to_a_program_before_the_holder_started_it() {
    let mut thread = claude();
    assert_eq!(thread.send("early"), Err(Refusal::NotRunning));
}

#[test]
fn thr_087_other_output_does_not_close_a_request() {
    // The core of a thread has no clock (the idle stop is not built): only an answer, a stop
    // or the end of the process closes a request; much other output changes nothing.
    let mut thread = held_claude();
    let request = asked(&thread.line(CLAUDE_ASK, &mut counter()))[0];
    for _ in 0..(30 * 24) {
        thread.line(CLAUDE_DELTA, &mut counter());
    }
    assert_eq!(thread.row.state, RunState::NeedsYou);
    assert_eq!(thread.row.open, [request]);
}

#[test]
fn thr_030_the_end_of_the_process_closes_its_requests_and_refuses_a_late_answer() {
    let mut thread = claude();
    let request = asked(&thread.line(CLAUDE_ASK, &mut counter()))[0];
    let steps = thread.exited(Some(1), None);
    assert!(steps.iter().any(|step| matches!(step, Step::Change(Change::Closed { request: closed, .. }) if *closed == request)));
    assert_eq!(thread.row.state, RunState::Stopped);
    assert_eq!(
        thread.answer(request, &Answer::Allow),
        Err(Refusal::RequestGone)
    );
    assert_eq!(thread.send("more"), Err(Refusal::NotRunning));
}

#[test]
fn th11_the_updates_kept_for_a_watch_have_a_bound_of_bytes() {
    let mut thread = claude();
    let big = "x".repeat(1024 * 1024);
    for _ in 0..10 {
        thread.note(Change::Prompt {
            text: big.clone(),
            source: "test".to_owned(),
        });
    }
    let kept = thread.kept().count();
    assert!((1..=4).contains(&kept), "kept {kept} updates of 1 MiB");
}

fn codex_in_a_turn(ids: &mut impl FnMut() -> Uuid) -> (Thread, Vec<Step>) {
    let mut thread = codex();
    thread.begin("go");
    thread.line(r#"{"id":1,"result":{}}"#, ids);
    thread.line(r#"{"id":2,"result":{"thread":{"id":"th"}}}"#, ids);
    let started = thread.line(
        r#"{"method":"turn/started","params":{"turn":{"id":"u1"}}}"#,
        ids,
    );
    (thread, started)
}

#[test]
fn thr_049_a_stop_of_a_codex_turn_ends_what_the_turn_started_and_spares_its_servers() {
    let mut ids = counter();
    let (mut thread, started) = codex_in_a_turn(&mut ids);
    assert_eq!(
        started.first(),
        Some(&Step::Mark),
        "the servers are marked at the turn start"
    );
    let steps = thread.interrupt();
    assert_eq!(writes(&steps)[0]["method"], "turn/interrupt");
    assert!(steps.contains(&Step::EndChildren));
    let claude_stop = held_claude().interrupt();
    assert_eq!(writes(&claude_stop)[0]["request"]["subtype"], "interrupt");
    assert!(!claude_stop.contains(&Step::EndChildren));
}

#[test]
fn thr_049_a_codex_thread_with_no_running_turn_ends_no_process_at_a_stop() {
    let mut thread = codex();
    thread.begin("go");
    let steps = thread.interrupt();
    assert_eq!(writes(&steps).len(), 0);
    assert!(!steps.contains(&Step::EndChildren));
}

#[test]
fn th11_codex_starts_its_server_and_thread_before_the_first_prompt() {
    let mut thread = codex();
    let mut ids = counter();
    let first = thread.begin("go");
    assert_eq!(writes(&first)[0]["method"], "initialize");
    let second = writes(&thread.line(r#"{"id":1,"result":{}}"#, &mut ids));
    assert_eq!(second[0]["method"], "initialized");
    assert_eq!(second[1]["method"], "thread/start");
    assert_eq!(second[1]["params"]["cwd"], "/w");
    let third = writes(&thread.line(r#"{"id":2,"result":{"thread":{"id":"th"}}}"#, &mut ids));
    assert_eq!(third[0]["method"], "turn/start");
    assert_eq!(third[0]["params"]["input"][0]["text"], "go");
    thread.line(
        r#"{"method":"turn/started","params":{"turn":{"id":"u1"}}}"#,
        &mut ids,
    );
    let steer = thread.send("also this").expect("running");
    assert_eq!(json(&steer[0])["method"], "turn/steer");
    thread.line(
        r#"{"method":"turn/completed","params":{"turn":{"id":"u1","status":"completed"}}}"#,
        &mut ids,
    );
    let next = thread.send("next").expect("running");
    assert_eq!(json(&next[0])["method"], "turn/start");
}

#[test]
fn th11_a_codex_server_that_refuses_its_thread_ends_and_says_why() {
    for refusal in [
        r#"{"id":2,"error":{"code":-32600,"message":"bad project file"}}"#,
        r#"{"id":2,"result":{"thread":{}}}"#,
    ] {
        let mut thread = codex();
        thread.begin("go");
        let mut ids = counter();
        thread.line(r#"{"id":1,"result":{}}"#, &mut ids);
        let steps = thread.line(refusal, &mut ids);
        assert!(
            steps.contains(&Step::EndProgram),
            "the program ends: {refusal}"
        );
    }
}

#[test]
fn thr_046_a_line_the_runner_does_not_know_stops_the_turn_and_keeps_the_requests() {
    let mut thread = held_claude();
    let mut ids = counter();
    let request = asked(&thread.line(CLAUDE_ASK, &mut ids))[0];
    let steps = thread.line(r#"{"type":"something_new"}"#, &mut ids);
    assert_eq!(writes(&steps)[0]["request"]["subtype"], "interrupt");
    assert!(
        steps.iter().any(|step| matches!(step, Step::Log(_))),
        "the stop is logged"
    );
    assert_eq!(
        thread.row.open,
        [request],
        "only an answer, a stop or the end closes it"
    );
}

#[test]
fn thr_030_a_restored_thread_keeps_the_id_of_its_open_request() {
    let mut thread = held_claude();
    let request = asked(&thread.line(CLAUDE_ASK, &mut counter()))[0];
    let saved = serde_json::to_string(&thread.saved()).expect("saved");
    let mut again = Thread::restore(serde_json::from_str(&saved).expect("reads"), None);
    assert_eq!(
        again.row.state,
        RunState::NeedsYou,
        "not Starting after a restart"
    );
    let line = again
        .answer(request, &Answer::Allow)
        .expect("the request is still open");
    assert_eq!(
        json(&line)["response"]["response"]["updatedInput"]["command"],
        "ls"
    );
}
