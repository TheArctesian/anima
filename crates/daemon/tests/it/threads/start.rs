//! A start, the input and the output of a program in the holder (THR-040, THR-113, THR-128).

use std::time::Duration;

use anima_daemon::threads::wire::{Action, Event, Outcome, Refusal};

use super::*;

#[tokio::test]
async fn thr_040_a_program_gets_only_the_given_environment_and_three_descriptors() {
    let holder = Holder::start(0);
    let mut link = holder.link().await;
    let id = link.send(start("env", "/usr/bin/env", &[])).await;
    assert!(matches!(link.answer(id).await, Outcome::Started { .. }));
    let lines = texts(&link.until(exited("env")).await);
    assert_eq!(
        lines
            .iter()
            .map(|(_, text)| text.as_str())
            .collect::<Vec<_>>(),
        ["AGENT_THREAD=env", "ONLY=this"],
        "the given environment and the mark of the thread (THR-049)"
    );

    let probe = r#"opendir D, "/dev/fd"; print join(" ", sort grep { /^\d+$/ } readdir D), "\n""#;
    let id = link
        .send(start("fds", "/usr/bin/perl", &["-e", probe]))
        .await;
    link.answer(id).await;
    let found = texts(&link.until(exited("fds")).await);
    // Descriptor 3 is the folder that perl itself opened to list them.
    assert_eq!(found, [(1, "0 1 2 3".to_owned())]);
}

#[tokio::test]
async fn thr_113_a_line_goes_in_and_comes_out_and_a_command_sent_twice_is_done_once() {
    let holder = Holder::start(0);
    let mut link = holder.link().await;
    let id = link.send(start("cat", "/bin/cat", &[])).await;
    link.answer(id).await;
    let write = link
        .send(Action::Write {
            thread: "cat".into(),
            line: "hello".into(),
        })
        .await;
    let again = anima_daemon::threads::wire::Command {
        id: write,
        action: Action::Write {
            thread: "cat".into(),
            line: "hello".into(),
        },
    };
    link.link.resend(&again).await.expect("sent");
    link.send(Action::CloseInput {
        thread: "cat".into(),
    })
    .await;
    let lines = texts(&link.until(exited("cat")).await);
    assert_eq!(lines, [(1, "hello".to_owned())]);
}

#[tokio::test]
async fn thr_113_no_line_is_lost_or_doubled_while_the_daemon_is_away_and_the_limit_holds() {
    let holder = Holder::start(4096);
    let mut link = holder.link().await;
    let script = "n=0; while [ $n -lt 2000 ]; do echo line-$n; n=$((n+1)); done";
    let id = link.send(start("many", "/bin/sh", &["-c", script])).await;
    link.answer(id).await;
    drop(link);
    tokio::time::sleep(Duration::from_millis(500)).await;
    let mut link = holder.link().await;
    let mut got: Vec<(u64, String)> = Vec::new();
    let mut waited = false;
    loop {
        let event = tokio::time::timeout(Duration::from_secs(20), link.link.events.recv())
            .await
            .expect("in time")
            .expect("there");
        waited |= matches!(event, Event::Waiting { .. });
        for (seq, text) in texts(std::slice::from_ref(&event)) {
            got.push((seq, text));
        }
        if let Some((seq, _)) = got.last() {
            link.send(Action::Ack {
                thread: "many".into(),
                seq: *seq,
            })
            .await;
        }
        if exited("many")(&event) {
            break;
        }
    }
    assert!(waited, "the holder stopped reading at its limit");
    assert_eq!(got.len(), 2000);
    assert!(
        got.iter()
            .enumerate()
            .all(|(at, (seq, text))| *seq == at as u64 + 1 && *text == format!("line-{at}"))
    );
}

#[tokio::test]
async fn thr_113_a_daemon_of_a_different_version_gets_no_new_start() {
    let holder = Holder::start(0);
    let mut link = holder.link().await;
    link.send(Action::Hello { version: 999 }).await;
    let id = link.send(start("new", "/bin/cat", &[])).await;
    assert_eq!(
        link.answer(id).await,
        Outcome::Refused {
            why: Refusal::Version
        }
    );
}

#[tokio::test]
async fn thr_128_the_folder_and_socket_are_private_and_a_second_holder_ends_at_once() {
    use std::os::unix::fs::PermissionsExt as _;
    let holder = Holder::start(0);
    let _link = holder.link().await;
    let mode = |path: &Path| std::fs::metadata(path).expect("there").permissions().mode() & 0o777;
    assert_eq!(mode(&holder.dir), 0o700);
    assert_eq!(mode(&holder.dir.join("holder.sock")), 0o600);
    let second = spawn(&holder.dir, 0).wait_with_output().expect("it ends");
    assert!(
        !second.status.success(),
        "a second holder of the folder ends"
    );
}
