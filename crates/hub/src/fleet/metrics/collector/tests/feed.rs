//! The feed of collections (D-9).

use async_graphql::ID;
use chrono::{DateTime, TimeDelta, Utc};
use tokio::sync::broadcast::error::{RecvError, TryRecvError};

use crate::fleet::metrics::{Feed, MetricsAdded, Readings};

fn added(seconds: i64) -> MetricsAdded {
    let at =
        DateTime::<Utc>::from_timestamp(1_800_000_000, 0).unwrap() + TimeDelta::seconds(seconds);
    MetricsAdded {
        device_id: ID("0190a000-0000-7000-8002-000000000002".to_owned()),
        at,
        samples: Vec::new(),
        readings: Readings {
            cpu_percent: Some(12.5),
            ..Readings::empty(at)
        },
    }
}

#[tokio::test]
async fn d9_every_subscriber_gets_what_is_published_after_it_subscribed() {
    let feed = Feed::new();
    feed.publish(added(0));
    let mut first = feed.subscribe();
    let mut second = feed.clone().subscribe();
    feed.publish(added(1));
    assert_eq!(first.recv().await.unwrap(), added(1));
    assert_eq!(second.recv().await.unwrap(), added(1));
    assert_eq!(first.try_recv(), Err(TryRecvError::Empty));
}

#[tokio::test]
async fn d9_a_subscriber_more_than_256_behind_skips_the_oldest_and_goes_on() {
    let feed = Feed::default();
    let mut slow = feed.subscribe();
    for second in 0..300 {
        feed.publish(added(second));
    }
    assert_eq!(slow.recv().await, Err(RecvError::Lagged(44)));
    assert_eq!(slow.recv().await.unwrap(), added(44));
}

#[test]
fn d9_publishing_with_no_subscriber_is_fine() {
    Feed::new().publish(added(0));
}
