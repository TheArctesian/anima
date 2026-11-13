//! The ten-second cache: one build per round, shared by every scrape, abandoned or not, and a
//! failed build kept two seconds (D-24). Builds run on paused tokio time where they wait.

use std::{
    future::Future,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use chrono::{DateTime, TimeDelta, Utc};

use crate::fleet::checks::exposition::cache::{BUILD_LIMIT, Cache, FAILED_FOR, FRESH_FOR};

fn at(seconds: i64) -> DateTime<Utc> {
    DateTime::from_timestamp(1_800_000_000 + seconds, 0).unwrap()
}

/// The future of a build in these tests.
type Build = Pin<Box<dyn Future<Output = Option<String>> + Send>>;

/// A build that counts its start and names the run in its text, after `takes` of tokio time.
fn counted(builds: &Arc<AtomicUsize>, takes: Duration) -> impl FnOnce() -> Build + use<> {
    let builds = Arc::clone(builds);
    move || {
        let run = builds.fetch_add(1, Ordering::SeqCst) + 1;
        Box::pin(async move {
            tokio::time::sleep(takes).await;
            Some(format!("build {run}"))
        })
    }
}

/// A build that starts at once and finishes without delay.
fn quick(builds: &Arc<AtomicUsize>) -> impl FnOnce() -> Build + use<> {
    counted(builds, Duration::ZERO)
}

fn started(builds: &AtomicUsize) -> usize {
    builds.load(Ordering::SeqCst)
}

#[tokio::test]
async fn d_24_the_body_is_kept_for_ten_seconds() {
    let (cache, builds) = (Cache::default(), Arc::default());
    let fresh_until = TimeDelta::seconds(FRESH_FOR.num_seconds() - 1);

    let first = cache.body(at(0), quick(&builds)).await;
    let within = cache.body(at(0) + fresh_until, quick(&builds)).await;
    let after = cache.body(at(0) + FRESH_FOR, quick(&builds)).await;

    assert_eq!(first.as_deref(), Some(&b"build 1"[..]));
    assert_eq!(within, first);
    assert_eq!(after.as_deref(), Some(&b"build 2"[..]));
    assert_eq!(started(&builds), 2);
}

#[tokio::test]
async fn d_24_a_failed_build_is_kept_two_seconds_then_rebuilt() {
    let (cache, builds) = (Cache::default(), Arc::<AtomicUsize>::default());
    let failing = {
        let builds = Arc::clone(&builds);
        move || {
            builds.fetch_add(1, Ordering::SeqCst);
            std::future::ready(None)
        }
    };

    let failed = cache.body(at(0), failing).await;
    let kept_until = FAILED_FOR - TimeDelta::milliseconds(1);
    let kept = cache.body(at(0) + kept_until, quick(&builds)).await;
    let rebuilt = cache.body(at(0) + FAILED_FOR, quick(&builds)).await;

    assert_eq!((failed, kept), (None, None));
    assert_eq!(rebuilt.as_deref(), Some(&b"build 2"[..]));
    assert_eq!(started(&builds), 2);
}

#[tokio::test]
async fn d_24_scrapes_during_a_build_share_it() {
    let (cache, builds) = (Cache::default(), Arc::default());
    let (release, held) = tokio::sync::oneshot::channel::<()>();
    let quick_one = quick(&builds);
    let slow = move || async move {
        held.await.unwrap();
        quick_one().await
    };

    let (first, second, ()) = tokio::join!(
        cache.body(at(0), slow),
        cache.body(at(1), quick(&builds)),
        async { release.send(()).unwrap() },
    );

    assert_eq!(started(&builds), 1);
    assert_eq!(first.as_deref(), Some(&b"build 1"[..]));
    assert_eq!(second, first);
}

#[tokio::test(start_paused = true)]
async fn d_24_abandoned_scrapes_share_one_build() {
    let (cache, builds) = (Cache::default(), Arc::default());
    let slow = Duration::from_millis(200);

    for _ in 0..50 {
        let scrape = cache.body(at(0), counted(&builds, slow));
        let abandoned = tokio::time::timeout(Duration::from_millis(2), scrape).await;
        assert!(abandoned.is_err(), "the build is still running");
    }
    let waited = cache.body(at(0), counted(&builds, slow)).await;
    let next = cache.body(at(1), counted(&builds, slow)).await;

    assert_eq!(started(&builds), 1);
    assert_eq!(waited.as_deref(), Some(&b"build 1"[..]));
    assert_eq!(next, waited);
}

#[tokio::test(start_paused = true)]
async fn d_24_a_hung_build_fails_its_round_after_the_limit() {
    let (cache, builds) = (Cache::default(), Arc::default());
    let start = tokio::time::Instant::now();

    let hung = cache.body(at(0), std::future::pending).await;
    let waited = start.elapsed();
    let next = cache.body(at(0) + FAILED_FOR, quick(&builds)).await;

    assert_eq!(hung, None);
    assert!(waited >= BUILD_LIMIT, "{waited:?}");
    assert_eq!(next.as_deref(), Some(&b"build 1"[..]));
}

#[tokio::test]
async fn d_24_a_build_that_panics_fails_its_scrape_and_the_next_scrape_rebuilds() {
    let (cache, builds) = (Cache::default(), Arc::default());
    let panics = || async { panic!("the build broke") };

    let broken = cache.body(at(0), panics).await;
    let next = cache.body(at(0), quick(&builds)).await;

    assert_eq!(broken, None);
    assert_eq!(next.as_deref(), Some(&b"build 1"[..]));
}

#[tokio::test]
async fn d_24_a_clock_moved_back_ten_seconds_rebuilds() {
    let (cache, builds) = (Cache::default(), Arc::default());

    cache.body(at(100), quick(&builds)).await;
    let back = cache.body(at(100) - FRESH_FOR, quick(&builds)).await;

    assert_eq!(back.as_deref(), Some(&b"build 2"[..]));
}
