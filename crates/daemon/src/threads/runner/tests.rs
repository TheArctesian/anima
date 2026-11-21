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
    let mut thread = claude();
    let mut ids = counter();
    let steps = thread.line(CLAUDE_ASK, &mut ids);
    let request = asked(&steps)[0];
    assert_eq!(thread.row.state, RunState::NeedsYou);
    assert_eq!(thread.row.open, [request]);
    let answered = thread
        .answer(request, &Answer::Allow)
        .expect("the first answer");
    assert_eq!(writes(&answered)[0]["response"]["request_id"], "r-1");
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
fn thr_087_no_output_and_no_time_closes_a_request() {
    // The core of a thread has no clock: only an answer, a stop or the end of the process
    // closes a request. Thirty days of other output change nothing.
    let mut thread = claude();
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
fn thr_049_a_stop_of_a_codex_turn_ends_the_processes_below_it() {
    let mut thread = codex();
    let mut ids = counter();
    thread.begin("go");
    thread.line(r#"{"id":1,"result":{}}"#, &mut ids);
    thread.line(r#"{"id":2,"result":{"thread":{"id":"th"}}}"#, &mut ids);
    thread.line(
        r#"{"method":"turn/started","params":{"turn":{"id":"u1"}}}"#,
        &mut ids,
    );
    let steps = thread.interrupt();
    assert_eq!(writes(&steps)[0]["method"], "turn/interrupt");
    assert!(steps.contains(&Step::EndChildren));
    let claude_stop = claude().interrupt();
    assert_eq!(writes(&claude_stop)[0]["request"]["subtype"], "interrupt");
    assert!(!claude_stop.contains(&Step::EndChildren));
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
    let steer = writes(&thread.send("also this").expect("running"));
    assert_eq!(steer[0]["method"], "turn/steer");
    thread.line(
        r#"{"method":"turn/completed","params":{"turn":{"id":"u1","status":"completed"}}}"#,
        &mut ids,
    );
    let next = writes(&thread.send("next").expect("running"));
    assert_eq!(next[0]["method"], "turn/start");
}

#[test]
fn th11_a_codex_server_that_refuses_its_thread_says_why() {
    let mut thread = codex();
    thread.begin("go");
    let mut ids = counter();
    thread.line(r#"{"id":1,"result":{}}"#, &mut ids);
    thread.line(
        r#"{"id":2,"error":{"code":-32600,"message":"bad project file"}}"#,
        &mut ids,
    );
    let steps = thread.exited(Some(1), None);
    let Some(Step::Change(Change::State { row, .. })) = steps.last() else {
        panic!("a state");
    };
    assert!(
        row.ended
            .as_deref()
            .is_some_and(|why| why.contains("bad project file"))
    );
}

#[test]
fn thr_046_a_line_the_runner_does_not_know_stops_the_turn() {
    let mut thread = claude();
    let steps = thread.line(r#"{"type":"something_new"}"#, &mut counter());
    assert_eq!(writes(&steps)[0]["request"]["subtype"], "interrupt");
}

#[test]
fn thr_030_a_restored_thread_keeps_the_id_of_its_open_request() {
    let mut thread = claude();
    let request = asked(&thread.line(CLAUDE_ASK, &mut counter()))[0];
    let saved = serde_json::to_string(&thread.saved()).expect("saved");
    let mut again = Thread::restore(serde_json::from_str(&saved).expect("reads"), None);
    assert_eq!(
        again.row.state,
        RunState::NeedsYou,
        "not Starting after a restart"
    );
    let steps = again
        .answer(request, &Answer::Allow)
        .expect("the request is still open");
    assert_eq!(
        writes(&steps)[0]["response"]["response"]["updatedInput"]["command"],
        "ls"
    );
}
