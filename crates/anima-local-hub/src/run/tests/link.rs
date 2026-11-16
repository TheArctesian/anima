//! The link (F-6): started after the hub, started again alone, stopped first and at once.

use std::fs;
use std::time::{Duration, Instant};

use super::{Rig, link_state, rig};
use crate::child::Stop;
use crate::run::Ended;
use crate::state::{Link, LinkState, State};

const LINK_START: &str = "link start link link.sock";

fn link_of(rig: &Rig) -> Option<LinkState> {
    match rig.state() {
        Some(State::Ready { link, .. }) => link,
        _ => None,
    }
}

#[tokio::test]
async fn f_6_a_link_that_ends_is_started_again_alone_with_longer_pauses_and_the_hub_stays_ready() {
    let rig = rig();
    rig.set("link-ends", "");

    let ended = rig
        .run_until(|rig| rig.count(LINK_START) >= 4)
        .await
        .unwrap();

    assert_eq!(ended, Ended::Asked);
    let events = rig.events();
    let link_starts = events.iter().position(|line| line == LINK_START).unwrap();
    assert_eq!(
        events[link_starts - 1],
        "hub start as hub.url gateway 127.0.0.1:4999"
    );
    assert_eq!(
        events[events.len() - 3..],
        ["hub stop", "gateway stop", "database stop"],
        "the group ran on: only the link started again"
    );
    assert_eq!(rig.count("hub start as hub.url gateway 127.0.0.1:4999"), 1);
    let log = fs::read_to_string(rig.plan.folders.logs().join("anima-link.log")).unwrap();
    assert!(log.contains("the relay cannot be reached"), "{log}");
    assert!(log.contains("anima-link is down; next start in"), "{log}");
}

#[tokio::test]
async fn f_6_the_pauses_between_starts_of_the_link_grow() {
    let rig = rig();
    rig.set("link-ends", "");
    let started = std::cell::Cell::new(None::<Instant>);

    rig.run_until(|rig| {
        let starts = rig.count(LINK_START);
        if starts == 1 && started.get().is_none() {
            started.set(Some(Instant::now()));
        }
        starts >= 4
    })
    .await
    .unwrap();

    // 100 ms, then 200 ms, then 400 ms (the longest) between the four starts.
    let took = started.get().unwrap().elapsed();
    assert!(took >= Duration::from_millis(600), "{took:?}");
}

#[tokio::test]
async fn f_6_the_state_tells_that_the_link_is_down_while_the_hub_is_ready() {
    let mut rig = rig();
    rig.set("link-ends", "");
    rig.plan.limits.link_backoff.first = Duration::from_secs(60);

    let ended = rig
        .run_until(|rig| link_of(rig) == Some(link_state(Link::Down)))
        .await
        .unwrap();

    assert_eq!(ended, Ended::Asked);
    assert_eq!(rig.count(LINK_START), 1);
}

#[tokio::test]
async fn f_6_a_stop_during_the_links_pause_acts_at_once() {
    let mut rig = rig();
    rig.set("link-ends", "");
    rig.plan.limits.link_backoff.first = Duration::from_secs(60);
    rig.plan.limits.link_backoff.longest = Duration::from_secs(60);
    let (ask, stop) = Stop::new();
    let asking = async {
        while link_of(&rig) != Some(link_state(Link::Down)) {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        ask.send_replace(true);
        Instant::now()
    };

    let (ended, asked_at) = tokio::join!(rig.run_with(stop, || {}), asking);

    assert_eq!(ended.unwrap(), Ended::Asked);
    let took = asked_at.elapsed();
    assert!(took < Duration::from_secs(5), "{took:?}");
    assert_eq!(rig.state(), Some(State::Stopped));
}

#[tokio::test]
async fn f_6_a_link_that_cannot_be_run_leaves_the_hub_ready() {
    let rig = rig();
    fs::remove_file(rig.plan.carried.program("anima-link")).unwrap();

    let ended = rig
        .run_until(|rig| link_of(rig) == Some(link_state(Link::Down)))
        .await
        .unwrap();

    assert_eq!(ended, Ended::Asked);
    assert_eq!(rig.count("hub stop"), 1);
    let log = fs::read_to_string(rig.plan.folders.logs().join("anima-link.log")).unwrap();
    assert!(log.contains("anima-link cannot be started"), "{log}");
}
