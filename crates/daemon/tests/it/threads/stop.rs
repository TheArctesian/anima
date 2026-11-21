//! The end of the processes of a thread and of the holder (THR-049, H9).

use anima_daemon::threads::tree;
use anima_daemon::threads::wire::{Action, Event, Outcome};

use super::*;

#[tokio::test]
async fn thr_049_a_stop_ends_three_children_in_three_process_groups() {
    let holder = Holder::start(0);
    let mut link = holder.link().await;
    let script = "for n in 1 2 3; do perl -e 'setpgrp(0,0); sleep 300' & echo $!; done; sleep 300";
    let id = link.send(start("tree", "/bin/sh", &["-c", script])).await;
    let Outcome::Started { pid } = link.answer(id).await else {
        panic!("not started")
    };
    let events = link
        .until(|event| {
            texts(std::slice::from_ref(event))
                .iter()
                .any(|(seq, _)| *seq == 3)
        })
        .await;
    let children: Vec<u32> = texts(&events)
        .iter()
        .map(|(_, text)| text.parse().expect("a pid"))
        .collect();
    assert_eq!(children.len(), 3);
    assert!(children.iter().all(|child| tree::alive(*child)));
    let id = link
        .send(Action::Stop {
            thread: "tree".into(),
        })
        .await;
    assert!(matches!(
        link.answer(id).await,
        Outcome::Stopped {
            all_ended: true,
            ..
        }
    ));
    link.until(exited("tree")).await;
    for child in children {
        assert!(gone(child).await, "a child is left");
    }
    assert!(gone(pid).await);
}

#[tokio::test]
async fn thr_049_ending_the_children_keeps_the_program() {
    let holder = Holder::start(0);
    let mut link = holder.link().await;
    let script = "perl -e 'setpgrp(0,0); sleep 300' & echo $!; read line; echo after";
    let id = link.send(start("server", "/bin/sh", &["-c", script])).await;
    link.answer(id).await;
    let events = link
        .until(|event| matches!(event, Event::Lines { .. }))
        .await;
    let child: u32 = texts(&events)[0].1.parse().expect("a pid");
    let id = link
        .send(Action::EndChildren {
            thread: "server".into(),
        })
        .await;
    link.answer(id).await;
    assert!(gone(child).await);
    link.send(Action::Write {
        thread: "server".into(),
        line: "go".into(),
    })
    .await;
    let lines = texts(&link.until(exited("server")).await);
    assert_eq!(
        lines.last().map(|(_, text)| text.as_str()),
        Some("after"),
        "the program went on"
    );
}

#[tokio::test]
async fn thr_021_ending_the_children_spares_the_servers_marked_before_the_turn() {
    let holder = Holder::start(0);
    let mut link = holder.link().await;
    let script = "perl -e 'setpgrp(0,0); sleep 300' & echo $!; read line; perl -e 'setpgrp(0,0); sleep 300' & echo $!; read line";
    let id = link.send(start("turns", "/bin/sh", &["-c", script])).await;
    link.answer(id).await;
    let first = texts(
        &link
            .until(|event| matches!(event, Event::Lines { .. }))
            .await,
    );
    let server: u32 = first[0].1.parse().expect("a pid");
    let mark = link
        .send(Action::Mark {
            thread: "turns".into(),
        })
        .await;
    assert_eq!(link.answer(mark).await, Outcome::Ok);
    link.send(Action::Write {
        thread: "turns".into(),
        line: "turn".into(),
    })
    .await;
    let second = texts(
        &link
            .until(|event| matches!(event, Event::Lines { .. }))
            .await,
    );
    let command: u32 = second[0].1.parse().expect("a pid");
    let end = link
        .send(Action::EndChildren {
            thread: "turns".into(),
        })
        .await;
    link.answer(end).await;
    assert!(gone(command).await, "the process of the turn ended");
    assert!(
        tree::alive(server),
        "the server marked before the turn is spared"
    );
    let stop = link
        .send(Action::Stop {
            thread: "turns".into(),
        })
        .await;
    link.answer(stop).await;
    assert!(gone(server).await, "a stop ends the spared server too");
}

#[tokio::test]
async fn thr_049_a_program_that_ends_by_itself_leaves_no_child_in_a_group_of_its_own() {
    let holder = Holder::start(0);
    let mut link = holder.link().await;
    let script = "perl -e 'setpgrp(0,0); sleep 300' & echo $!; sleep 3";
    let id = link.send(start("alone", "/bin/sh", &["-c", script])).await;
    link.answer(id).await;
    let child: u32 = texts(
        &link
            .until(|event| matches!(event, Event::Lines { .. }))
            .await,
    )[0]
    .1
    .parse()
    .expect("a pid");
    link.until(exited("alone")).await;
    assert!(gone(child).await, "the child the program left is ended");
    let stop = link
        .send(Action::Stop {
            thread: "alone".into(),
        })
        .await;
    assert_eq!(
        link.answer(stop).await,
        Outcome::Stopped {
            ended: 0,
            all_ended: true
        }
    );
}

#[tokio::test]
async fn thr_049_a_stop_ends_a_process_that_left_the_tree_after_it_was_seen() {
    let holder = Holder::start(0);
    let mut link = holder.link().await;
    let script = "(perl -e 'setpgrp(0,0); sleep 300' & echo $!; sleep 3) & sleep 300";
    let id = link.send(start("left", "/bin/sh", &["-c", script])).await;
    link.answer(id).await;
    let child: u32 = texts(
        &link
            .until(|event| matches!(event, Event::Lines { .. }))
            .await,
    )[0]
    .1
    .parse()
    .expect("a pid");
    // The holder looks each 2 s; the subshell ends after 3 s and the child leaves the tree.
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    assert!(tree::alive(child), "the child runs outside the tree now");
    let stop = link
        .send(Action::Stop {
            thread: "left".into(),
        })
        .await;
    assert!(matches!(
        link.answer(stop).await,
        Outcome::Stopped {
            all_ended: true,
            ..
        }
    ));
    assert!(
        gone(child).await,
        "the stop ended the child that left the tree"
    );
}

#[tokio::test]
async fn thr_113_a_program_that_does_not_read_its_input_holds_up_no_command() {
    let holder = Holder::start(0);
    let mut link = holder.link().await;
    let id = link
        .send(start("deaf", "/bin/sh", &["-c", "sleep 300"]))
        .await;
    let Outcome::Started { pid } = link.answer(id).await else {
        panic!("not started")
    };
    let big = "x".repeat(1024 * 1024);
    let mut full = false;
    for _ in 0..64 {
        let write = link
            .send(Action::Write {
                thread: "deaf".into(),
                line: big.clone(),
            })
            .await;
        let answer = tokio::time::timeout(std::time::Duration::from_secs(5), link.answer(write))
            .await
            .expect("a write is answered at once");
        if matches!(
            answer,
            Outcome::Refused {
                why: anima_daemon::threads::wire::Refusal::InputFull
            }
        ) {
            full = true;
            break;
        }
    }
    assert!(full, "the queue in front of the input fills and says so");
    let stop = link
        .send(Action::Stop {
            thread: "deaf".into(),
        })
        .await;
    let answer = tokio::time::timeout(std::time::Duration::from_secs(10), link.answer(stop))
        .await
        .expect("a stop is not held up by the full input");
    assert!(matches!(answer, Outcome::Stopped { .. }));
    assert!(gone(pid).await);
}

#[tokio::test]
async fn thr_049_the_end_of_the_holder_ends_its_programs_and_a_new_holder_ends_what_a_killed_one_left()
 {
    let mut holder = Holder::start(0);
    let mut link = holder.link().await;
    let id = link
        .send(start(
            "a",
            "/bin/sh",
            &[
                "-c",
                "perl -e 'setpgrp(0,0); sleep 300' & echo $!; sleep 300",
            ],
        ))
        .await;
    let Outcome::Started { pid } = link.answer(id).await else {
        panic!("not started")
    };
    let child: u32 = texts(
        &link
            .until(|event| matches!(event, Event::Lines { .. }))
            .await,
    )[0]
    .1
    .parse()
    .expect("a pid");
    holder.child.kill().expect("SIGKILL");
    holder.child.wait().expect("ended");
    assert!(tree::alive(pid), "a killed holder ends nothing by itself");
    holder.child = spawn(&holder.dir, 0);
    let _link = holder.link().await;
    assert!(
        gone(pid).await && gone(child).await,
        "the new holder ended what the old one left"
    );

    let mut link = holder.link().await;
    let id = link.send(start("b", "/bin/sh", &["-c", "sleep 300"])).await;
    let Outcome::Started { pid } = link.answer(id).await else {
        panic!("not started")
    };
    rustix::process::kill_process(
        rustix::process::Pid::from_raw(i32::try_from(holder.child.id()).expect("pid"))
            .expect("pid"),
        rustix::process::Signal::TERM,
    )
    .expect("SIGTERM");
    holder.child.wait().expect("the holder ends");
    assert!(gone(pid).await, "SIGTERM to the holder ends its programs");
}
