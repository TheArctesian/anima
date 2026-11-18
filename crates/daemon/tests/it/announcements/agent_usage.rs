//! The agent usage in a batch: the state and at most 16 session days in the next batch, kept
//! until a 204, dropped after a `MALFORMED` (AIS-019, K8 Section 4).

use anima_daemon::push::Event;
use anima_daemon::sampling::agent_usage::UsageReport;
use chrono::{DateTime, NaiveDate};
use daemon_core::agent_usage::{SessionDay, Source, UsageState};
use daemon_core::wire::RefusalCode;

use crate::fake_hub::{FakeHub, Reply};
use crate::kit::{Rig, at, points, rig, year};

fn feed(rig: &mut Rig, first: i64, count: i64) {
    for point in points(first, count) {
        rig.pusher.accept(Event::Point(point));
    }
}

fn day(id: u32) -> SessionDay {
    SessionDay {
        source: Source::ClaudeCode,
        id: format!("s{id:02}"),
        day: NaiveDate::from_ymd_opt(2026, 11, 8).expect("a day"),
        started_at: DateTime::from_timestamp(1_794_000_000, 0).expect("a time"),
        ended_at: DateTime::from_timestamp(1_794_003_600, 0).expect("a time"),
        repository: Some("github.com/me/app".to_owned()),
        prompts: 2,
        tool_calls: 3,
        models: Vec::new(),
    }
}

fn reading(count: u32) -> Event {
    Event::AgentUsage(UsageReport {
        state: UsageState::Reading,
        session_days: (0..count).map(day).collect(),
    })
}

/// The number of session days in each push, `None` for a push without `agentUsage`.
fn days_of(hub: &FakeHub) -> Vec<Option<usize>> {
    hub.seen("push")
        .iter()
        .map(|seen| {
            let usage = seen.body.get("agentUsage")?;
            Some(usage["sessionDays"].as_array().map_or(0, Vec::len))
        })
        .collect()
}

#[tokio::test]
async fn ais_019_agent_usage_goes_in_the_next_batch_and_not_in_the_one_after() {
    let hub = FakeHub::start().await;
    let mut rig = rig(&hub, 1, year());
    rig.pusher.accept(reading(1));
    feed(&mut rig, 1, 2);
    rig.pusher.push_due(at(30)).await;
    feed(&mut rig, 3, 2);
    rig.pusher.push_due(at(50)).await;
    assert_eq!(days_of(&hub), [Some(1), None], "cleared after the 204");
    let sent = &hub.seen("push")[0].body["agentUsage"];
    assert_eq!(sent["state"], "reading");
    assert_eq!(sent["sessionDays"][0]["repository"], "github.com/me/app");
}

#[tokio::test]
async fn ais_019_session_days_go_16_to_a_batch() {
    let hub = FakeHub::start().await;
    let mut rig = rig(&hub, 1, year());
    rig.pusher.accept(reading(20));
    feed(&mut rig, 1, 2);
    rig.pusher.push_due(at(30)).await;
    feed(&mut rig, 3, 1);
    rig.pusher.push_due(at(40)).await;
    feed(&mut rig, 4, 1);
    rig.pusher.push_due(at(50)).await;
    assert_eq!(days_of(&hub), [Some(16), Some(4), None]);
}

#[tokio::test]
async fn ais_019_agent_usage_is_sent_again_until_a_batch_is_accepted() {
    let hub = FakeHub::start().await;
    let mut rig = rig(&hub, 1, year());
    hub.queue_push([Reply::refuse(RefusalCode::OutOfRange), Reply::json(500, "")]);
    rig.pusher.accept(reading(1));
    feed(&mut rig, 1, 2);
    rig.pusher.push_due(at(30)).await;
    feed(&mut rig, 3, 1);
    rig.pusher.push_due(at(40)).await;
    rig.pusher.push_due(at(50)).await;
    feed(&mut rig, 4, 1);
    rig.pusher.push_due(at(60)).await;
    assert_eq!(
        days_of(&hub),
        [Some(1), Some(1), Some(1), None],
        "kept through a dropped batch and a 5xx, cleared after the 204"
    );
}

#[tokio::test]
async fn ais_019_refused_session_days_are_not_sent_again_after_malformed() {
    let hub = FakeHub::start().await;
    let mut rig = rig(&hub, 1, year());
    hub.queue_push([Reply::refuse(RefusalCode::Malformed)]);
    rig.pusher.accept(reading(1));
    feed(&mut rig, 1, 2);
    rig.pusher.push_due(at(30)).await;
    feed(&mut rig, 3, 2);
    rig.pusher.push_due(at(50)).await;
    assert_eq!(days_of(&hub), [Some(1), None]);
}
