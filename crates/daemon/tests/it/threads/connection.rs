//! The connection between the daemon and the holder: a takeover, a frame that does not read, a
//! command sent again on a new connection, output larger than a frame (H8, H10, THR-128).

use std::time::Duration;

use anima_daemon::threads::link::{HolderLink, LinkError};
use anima_daemon::threads::wire::{Action, Command, Event, Outcome};
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};

use super::{Holder, exited, start, texts};

fn hello_threads(events: &[Event]) -> Vec<String> {
    events
        .iter()
        .find_map(|event| match event {
            Event::Hello { threads, .. } => {
                Some(threads.iter().map(|held| held.thread.clone()).collect())
            }
            _ => None,
        })
        .unwrap_or_default()
}

#[tokio::test]
async fn h10_a_newer_connection_takes_the_place_of_the_older_and_the_older_stops() {
    let holder = Holder::start(0);
    let mut older = holder.link().await;
    older
        .until(|event| matches!(event, Event::Hello { .. }))
        .await;
    let mut newer = holder.link().await;
    newer
        .until(|event| matches!(event, Event::Hello { .. }))
        .await;
    let closed = tokio::time::timeout(Duration::from_secs(5), async {
        while older.link.events.recv().await.is_some() {}
    })
    .await;
    assert!(closed.is_ok(), "the older connection is closed in time");
    let _ = older.link.send(start("late", "/bin/cat", &[])).await;
    let id = newer
        .send(Action::Hello {
            version: anima_daemon::threads::wire::VERSION,
        })
        .await;
    let events = newer
        .until(|event| matches!(event, Event::Done { id: done, .. } if *done == id))
        .await;
    assert!(
        hello_threads(&events).is_empty(),
        "a command of the older connection did not run"
    );
}

#[tokio::test]
async fn h10_a_start_sent_again_on_a_new_connection_is_done_one_time() {
    let holder = Holder::start(0);
    let command = Command {
        id: 7,
        action: start("once", "/bin/cat", &[]),
    };
    let mut first = holder.link().await;
    first.link.resend(&command).await.expect("sent");
    let Outcome::Started { pid } = first.answer(7).await else {
        panic!("it started");
    };
    drop(first);
    let mut second = holder.link().await;
    second.link.resend(&command).await.expect("sent");
    assert_eq!(second.answer(7).await, Outcome::Started { pid });
    let id = second
        .send(Action::Hello {
            version: anima_daemon::threads::wire::VERSION,
        })
        .await;
    let events = second
        .until(|event| matches!(event, Event::Done { id: done, .. } if *done == id))
        .await;
    assert_eq!(hello_threads(&events), ["once"]);
    let stop = second
        .send(Action::Stop {
            thread: "once".into(),
        })
        .await;
    second.answer(stop).await;
}

#[tokio::test]
async fn h8_a_frame_that_does_not_read_closes_the_connection_and_harms_no_program() {
    let holder = Holder::start(0);
    let mut link = holder.link().await;
    let id = link.send(start("kept", "/bin/cat", &[])).await;
    link.answer(id).await;
    drop(link);
    let mut raw = tokio::net::UnixStream::connect(holder.dir.join("holder.sock"))
        .await
        .expect("connects");
    raw.write_all(b"this is not a frame\n")
        .await
        .expect("written");
    let mut rest = Vec::new();
    let read = tokio::time::timeout(Duration::from_secs(5), raw.read_to_end(&mut rest)).await;
    assert!(read.is_ok(), "the holder closed the connection");
    let mut link = holder.link().await;
    let events = link
        .until(|event| matches!(event, Event::Hello { .. }))
        .await;
    assert_eq!(hello_threads(&events), ["kept"], "the program still runs");
    let stop = link
        .send(Action::Stop {
            thread: "kept".into(),
        })
        .await;
    link.answer(stop).await;
}

#[tokio::test]
async fn thr_128_the_daemon_refuses_a_socket_that_others_could_use() {
    use std::os::unix::fs::PermissionsExt as _;
    let holder = Holder::start(0);
    drop(holder.link().await);
    let socket = holder.dir.join("holder.sock");
    std::fs::set_permissions(&socket, std::fs::Permissions::from_mode(0o666)).expect("mode");
    assert!(matches!(
        HolderLink::connect(&holder.dir).await,
        Err(LinkError::NotOurs)
    ));
}

#[tokio::test]
async fn h8_output_over_the_limit_of_the_holder_reaches_a_daemon_that_comes_back() {
    let holder = Holder::start(0);
    let mut link = holder.link().await;
    // Four lines of 3 MiB: more than the 8 MiB that the holder keeps, written while no daemon
    // is there.
    let script = "for n in 1 2 3 4; do head -c 3145728 /dev/zero | tr '\\0' x; echo; done";
    let id = link.send(start("big", "/bin/sh", &["-c", script])).await;
    link.answer(id).await;
    drop(link);
    tokio::time::sleep(Duration::from_millis(1500)).await;
    let mut link = holder.link().await;
    let mut count = 0;
    loop {
        let event = tokio::time::timeout(Duration::from_secs(30), link.link.events.recv())
            .await
            .expect("in time")
            .expect("the connection stays");
        let lines = texts(std::slice::from_ref(&event));
        for (seq, text) in &lines {
            count += 1;
            assert_eq!(*seq, count, "each line one time, in order");
            assert_eq!(text.len(), 3 * 1024 * 1024);
        }
        if let Some((seq, _)) = lines.last() {
            link.send(Action::Ack {
                thread: "big".into(),
                seq: *seq,
            })
            .await;
        }
        if exited("big")(&event) {
            break;
        }
    }
    assert_eq!(count, 4);
}
