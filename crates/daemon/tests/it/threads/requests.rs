//! The runner with made programs: an open request across a restart of the runner, one runner
//! for one folder, and the environment an agent program gets (THR-030, THR-040, THR-113).

use std::time::Duration;

use agent_threads::control::{Action, Change, Outcome, RunState};
use agent_threads::drive::{Answer, Mode, Program};
use uuid::Uuid;

use super::runner::{Client, Kit, SECRET, change, child_pid, state_is};

/// A made Claude Code that writes its environment to `env.txt` in its folder and names its
/// session.
const DUMP: &str = r#"#!/usr/bin/perl
$| = 1;
open(my $file, ">", "env.txt"); print $file map { "$_=$ENV{$_}\n" } sort keys %ENV; close($file);
print qq({"type":"system","subtype":"init","session_id":"s","model":"made","permissionMode":"bypassPermissions"}\n);
while (my $line = <STDIN>) {}
"#;

async fn start(client: &mut Client, kit: &Kit) -> Uuid {
    let start = client
        .call(Action::Start {
            program: Program::ClaudeCode,
            folder: kit.path("work"),
            mode: Mode::FullAccess,
            model: None,
            effort: None,
            prompt: "Run the command".to_owned(),
        })
        .await;
    let Outcome::Started { thread } = client.done(start).await else {
        panic!("the thread started");
    };
    thread
}

#[tokio::test]
async fn thr_030_an_open_request_survives_a_restart_of_the_runner_and_takes_one_answer() {
    let mut kit = Kit::new();
    let mut client = kit.client().await;
    let watch = client.call(Action::Watch).await;
    client.done(watch).await;
    let thread = start(&mut client, &kit).await;
    let asked = client
        .until(|reply| matches!(change(reply), Some(Change::Asked { .. })))
        .await;
    let Some(Change::Asked { request, .. }) = change(&asked) else {
        unreachable!()
    };
    let request = *request;
    client.until(state_is(RunState::NeedsYou)).await;
    drop(client);

    kit.end_runner();
    kit.start();
    let mut client = kit.client().await;
    let list = client.call(Action::List).await;
    let Outcome::Threads { threads } = client.done(list).await else {
        panic!("threads")
    };
    assert_eq!(threads[0].state, RunState::NeedsYou);
    assert_eq!(threads[0].open, [request], "the request kept its ID");
    let answer = Action::Answer {
        thread,
        request,
        answer: Answer::Allow,
    };
    let first = client.call(answer.clone()).await;
    assert_eq!(client.done(first).await, Outcome::Ok);
    let child = child_pid(&kit.path("work")).await;
    let second = client.call(answer).await;
    assert!(matches!(client.done(second).await, Outcome::Refused { .. }));
    let stop = client.call(Action::Stop { thread }).await;
    client.done(stop).await;
    assert!(super::gone(child).await);
}

#[tokio::test]
async fn thr_113_a_second_runner_on_the_same_folder_ends_at_once() {
    let kit = Kit::new();
    let mut client = kit.client().await;
    let mut second = kit.spawn_runner();
    let mut ended = None;
    for _ in 0..100 {
        ended = second.try_wait().expect("a state");
        if ended.is_some() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert!(
        ended.is_some_and(|status| !status.success()),
        "the second runner ended with a fault"
    );
    let list = client.call(Action::List).await;
    assert!(
        matches!(client.done(list).await, Outcome::Threads { .. }),
        "the first one serves"
    );
}

#[tokio::test]
async fn thr_040_an_agent_gets_the_environment_of_the_login_and_no_secret_of_anima() {
    let kit = Kit::with(DUMP);
    let mut client = kit.client().await;
    let thread = start(&mut client, &kit).await;
    let file = kit.path("work").join("env.txt");
    let mut text = String::new();
    for _ in 0..100 {
        text = std::fs::read_to_string(&file).unwrap_or_default();
        if !text.is_empty() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert!(
        text.contains("LOGIN_ONLY=yes\n"),
        "the names of the login profile: {text}"
    );
    assert!(
        !text.contains(SECRET),
        "no value of a secret of Anima: {text}"
    );
    assert!(!text.contains("ANIMA_"), "no name of Anima: {text}");
    assert!(text.contains("CLAUDE_CODE_EMIT_SESSION_STATE_EVENTS=1\n"));
    let stop = client.call(Action::Stop { thread }).await;
    client.done(stop).await;
}
