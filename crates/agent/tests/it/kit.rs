//! Shared set-up: tokens, points, a pusher over a fake hub.

use agent_core::points::{Kind, Point, SeriesKey, Window};
use agent_core::wire::AgentToken;
use anima_agent::config::home::Home;
use anima_agent::hub::HubClient;
use anima_agent::push::Pusher;
use anima_agent::token_store::{Credential, TokenStore};
use chrono::{DateTime, TimeDelta, Utc};
use tempfile::TempDir;
use uuid::Uuid;

use crate::fake_hub::FakeHub;

/// A multiple of 10 seconds, so every point's `at` is a window end.
pub const BASE: i64 = 1_800_000_000;

pub fn at(seconds: i64) -> DateTime<Utc> {
    DateTime::from_timestamp(BASE + seconds, 0).expect("a time")
}

pub fn token(seed: u8) -> AgentToken {
    AgentToken::from_random([seed; 32])
}

pub fn device() -> Uuid {
    Uuid::from_u128(0xd1)
}

/// `count` points, the `i`th ending at `BASE + 10 * (first + i)` seconds, with CPU `first + i`.
pub fn points(first: i64, count: i64) -> Vec<Point> {
    let key = SeriesKey::new(Kind::CpuPercent, None, None).expect("a key");
    let mut window = Window::new();
    let mut made = Vec::new();
    for step in first..=first + count {
        let value = f64::from(u32::try_from(step % 100).expect("small"));
        made.extend(window.add(at(step * 10), [(key.clone(), value)]));
    }
    made
}

/// A pusher with token `seed` expiring `expires_in` after `BASE`, over a store in a new folder.
pub struct Rig {
    pub pusher: Pusher,
    pub home: Home,
    _dir: TempDir,
}

pub fn rig(hub: &FakeHub, seed: u8, expires_in: TimeDelta) -> Rig {
    let dir = tempfile::tempdir().expect("a folder");
    let home = Home::at(dir.path().join("agent"));
    let store = TokenStore::open(&home);
    let credential = Credential {
        token: token(seed),
        expires_at: at(0) + expires_in,
        hub: None,
    };
    store.save(&credential).expect("stored");
    let client = HubClient::new(hub.url.clone()).expect("a client");
    Rig {
        pusher: Pusher::new(client, device(), credential, store),
        home,
        _dir: dir,
    }
}

pub fn year() -> TimeDelta {
    TimeDelta::days(365)
}

/// The `at` of each point of a push body, as seconds after `BASE`.
pub fn pushed_times(body: &serde_json::Value) -> Vec<i64> {
    body["points"]
        .as_array()
        .expect("points")
        .iter()
        .map(|point| {
            let text = point["at"].as_str().expect("at");
            DateTime::parse_from_rfc3339(text)
                .expect("rfc3339")
                .timestamp()
                - BASE
        })
        .collect()
}
