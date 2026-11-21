//! An update of the daemon under a running holder (TH98, SF-235): the runner says
//! that the holder is older, keeps it while a thread runs, and replaces it when none runs.

use std::path::Path;
use std::time::Duration;

use agent_threads::control::{Action, Outcome};
use agent_threads::drive::{Mode, Program};

use super::runner::{CLAUDE, Client, Kit};

/// The process ID of the holder of `dir`, from `ps`.
fn holder_pid(dir: &Path) -> Option<u32> {
    let output = std::process::Command::new("/bin/ps")
        .args(["-axww", "-o", "pid=,command="])
        .output()
        .ok()?;
    let mark = format!(" holder {}", dir.display());
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .find(|line| line.trim_end().ends_with(&mark))
        .and_then(|line| line.split_whitespace().next()?.parse().ok())
}

/// A copy of the daemon at `path`, as an update writes it: a new file put in place.
fn install(path: &Path) {
    let fresh = path.with_extension("new");
    std::fs::copy(env!("CARGO_BIN_EXE_anima-daemon"), &fresh).expect("copied");
    std::fs::rename(&fresh, path).expect("put in place");
}

async fn holder_words(client: &mut Client) -> Option<String> {
    let list = client.call(Action::List).await;
    let Outcome::Threads { holder, .. } = client.done(list).await else {
        panic!("threads")
    };
    holder
}

#[tokio::test]
async fn th98_an_older_holder_is_named_kept_while_a_thread_runs_and_replaced_after() {
    let mut kit = Kit::unstarted(CLAUDE);
    let daemon = kit.path("anima-daemon");
    install(&daemon);
    kit.start_from(&daemon);
    let mut client = kit.client().await;
    assert_eq!(
        holder_words(&mut client).await,
        None,
        "the holder of this build"
    );
    let start = client
        .call(Action::Start {
            program: Program::ClaudeCode,
            folder: kit.path("work"),
            mode: Mode::FullAccess,
            model: None,
            effort: None,
            prompt: "Run the command".to_owned(),
            program_path: None,
        })
        .await;
    let Outcome::Started { thread } = client.done(start).await else {
        panic!("the thread started");
    };
    let first = holder_pid(&kit.threads()).expect("a holder");
    drop(client);

    kit.end_runner();
    install(&daemon);
    kit.start_from(&daemon);
    let mut client = kit.client().await;
    let words = holder_words(&mut client).await.expect("an older holder");
    assert!(words.contains("older than the daemon"), "{words}");
    assert!(words.contains("(1 running now)"), "{words}");
    assert_eq!(
        holder_pid(&kit.threads()),
        Some(first),
        "kept while a thread runs"
    );

    let stop = client.call(Action::Stop { thread }).await;
    client.done(stop).await;
    let mut replaced = None;
    for _ in 0..60 {
        tokio::time::sleep(Duration::from_millis(500)).await;
        replaced = holder_pid(&kit.threads()).filter(|pid| *pid != first);
        if replaced.is_some() {
            break;
        }
    }
    assert!(replaced.is_some(), "a holder of the new build runs");
    drop(client);
    let mut client = kit.client().await;
    for _ in 0..20 {
        if holder_words(&mut client).await.is_none() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    panic!("the runner still names the holder older");
}

fn start_action(kit: &Kit) -> Action {
    Action::Start {
        program: Program::ClaudeCode,
        folder: kit.path("work"),
        mode: Mode::FullAccess,
        model: None,
        effort: None,
        prompt: "Run the command".to_owned(),
        program_path: None,
    }
}

#[tokio::test]
async fn th98_a_refused_quit_lets_starts_go_on() {
    let mut kit = Kit::unstarted(CLAUDE);
    let daemon = kit.path("anima-daemon");
    install(&daemon);
    kit.start_from(&daemon);
    let mut client = kit.client().await;
    let start = client.call(start_action(&kit)).await;
    assert!(matches!(client.done(start).await, Outcome::Started { .. }));
    drop(client);

    // A runner that forgot its thread meets an older holder that still holds the program: it
    // ends the program and asks the holder to quit at once, and the holder may refuse while the
    // program is not yet reaped.
    kit.end_runner();
    std::fs::remove_file(kit.threads().join("threads.json")).expect("the record");
    install(&daemon);
    kit.start_from(&daemon);
    let mut client = kit.client().await;
    for _ in 0..20 {
        let start = client.call(start_action(&kit)).await;
        if matches!(client.done(start).await, Outcome::Started { .. }) {
            return;
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    panic!("no thread starts in 10 seconds after the old holder was asked to quit");
}
