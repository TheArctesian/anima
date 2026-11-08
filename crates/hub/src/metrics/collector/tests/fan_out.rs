//! At most 8 collections at once and 20 seconds per device, on paused time: stalled devices
//! fail with the deadline while the others finish (D-11).

use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use tokio::time::{Instant, sleep};

use crate::{
    metrics::collector::{
        AT_ONCE,
        collection::{Failure, within_deadline},
        fan_out::FanOut,
    },
    ssh::LinkError,
};

/// How many jobs run now, and the most that ever ran together.
#[derive(Default)]
struct Gauge {
    now: AtomicUsize,
    most: AtomicUsize,
}

/// A device that answers after `seconds`, within the collection deadline, as a collection does.
async fn device(gauge: Arc<Gauge>, index: usize, seconds: u64) -> (usize, Result<usize, Failure>) {
    let running = gauge.now.fetch_add(1, Ordering::SeqCst) + 1;
    gauge.most.fetch_max(running, Ordering::SeqCst);
    let answered = within_deadline(async {
        sleep(Duration::from_secs(seconds)).await;
        Ok::<_, LinkError>(index)
    })
    .await;
    gauge.now.fetch_sub(1, Ordering::SeqCst);
    (index, answered)
}

#[tokio::test(start_paused = true)]
async fn d11_eight_at_once_and_a_stalled_device_fails_at_20_seconds_while_the_rest_finish() {
    let gauge = Arc::new(Gauge::default());
    // Devices 0 and 1 stall for a minute; the other ten answer after 19 seconds.
    let jobs: Vec<_> = (0..12)
        .map(|index| device(Arc::clone(&gauge), index, if index < 2 { 60 } else { 19 }))
        .collect();
    let start = Instant::now();
    let mut ended = Vec::new();
    let mut collections = FanOut::new(jobs, AT_ONCE);
    while let Some(joined) = collections.next().await {
        let (index, answered) = joined.unwrap();
        ended.push((index, answered, start.elapsed()));
    }
    assert_eq!(
        gauge.most.load(Ordering::SeqCst),
        8,
        "never more than 8 at once"
    );
    assert_eq!(ended.len(), 12, "every device ends");
    for (index, answered, at) in &ended {
        if *index < 2 {
            assert!(
                matches!(answered, Err(Failure::Deadline)),
                "{index}: {answered:?}"
            );
            assert_eq!(*at, Duration::from_secs(20), "{index}");
        } else {
            assert_eq!(answered.as_ref().ok(), Some(index));
        }
    }
    // Devices 2 to 7 end at 19 s and free six places; 8 to 11 take four of them.
    let last = ended.iter().map(|(_, _, at)| *at).max().unwrap();
    assert_eq!(last, Duration::from_secs(38));
}

#[tokio::test(start_paused = true)]
async fn d11_a_link_error_inside_the_deadline_is_the_link_failure() {
    let failed = within_deadline(async { Err::<(), _>(LinkError::NoHostKey) }).await;
    assert!(matches!(failed, Err(Failure::Link(LinkError::NoHostKey))));
}
