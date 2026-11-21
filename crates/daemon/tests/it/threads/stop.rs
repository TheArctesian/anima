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
