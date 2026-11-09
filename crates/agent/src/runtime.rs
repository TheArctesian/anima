//! `anima-agent run`: two tasks and the clock. One samples every 2 seconds and hands points
//! over; the other keeps the buffer and pushes every 10 seconds, so a slow hub never delays
//! sampling (MET-014, K8 Section 7). This is the I/O edge: clock, sleep, signals.

use std::time::Duration;

use chrono::Utc;
use tokio::sync::mpsc;
use tokio::time::{Instant, MissedTickBehavior, interval, sleep_until};
use uuid::Uuid;

use crate::config::home::{Home, HomeError};
use crate::hub::{HubClient, HubError};
use crate::push::pacing::RHYTHM;
use crate::push::{Event, Pusher};
use crate::readers::{self, Platform, gpu};
use crate::report;
use crate::sampling::{Sampler, gpu as gpu_series};
use crate::token_store::{TokenStore, TokenStoreError};

/// The time between samplings (MET-014).
const SAMPLE_EVERY: Duration = Duration::from_secs(2);

/// The time between GPU discoveries (MET-006).
const DISCOVER_EVERY: Duration = Duration::from_hours(24);

/// Points waiting to cross from sampling to the pusher: far more than a slow push can build up.
const CHANNEL_ROOM: usize = 64;

#[derive(Debug, thiserror::Error)]
pub enum RunError {
    #[error("this agent is not enrolled: run `anima-agent enrol <code>` first")]
    NotEnrolled,
    #[error("{0}")]
    Home(#[from] HomeError),
    #[error("{0}")]
    Token(#[from] TokenStoreError),
    #[error("{0}")]
    Client(#[from] HubError),
}

/// Samples and pushes until `stop` completes.
///
/// # Errors
/// [`RunError`] when the agent is not enrolled or its files or client cannot be set up.
pub async fn run(home: &Home, stop: impl Future<Output = ()>) -> Result<(), RunError> {
    let (pusher, device) = prepare(home)?;
    let (sender, receiver) = mpsc::channel(CHANNEL_ROOM);
    let sampling = tokio::spawn(sample(Platform::current(), sender));
    report::running(device);
    tokio::select! {
        () = push(pusher, receiver) => {}
        () = stop => report::stopping(),
    }
    sampling.abort();
    Ok(())
}

/// The pusher for the enrolled device, from the configuration and the token store.
fn prepare(home: &Home) -> Result<(Pusher, Uuid), RunError> {
    let config = home.read_config()?.ok_or(RunError::NotEnrolled)?;
    let store = TokenStore::open(home);
    let credential = store.load()?.ok_or(RunError::NotEnrolled)?;
    let hub = HubClient::new(config.hub.url().clone())?;
    Ok((
        Pusher::new(hub, config.device, credential, store),
        config.device,
    ))
}

/// Every 2 seconds: read, make a sample, and send the point each 10-second window closes.
async fn sample(platform: Platform, events: mpsc::Sender<Event>) {
    let mut sampler = Sampler::new();
    let (mut sources, mut placement) = discover(platform, &events).await;
    let mut next_discovery = Instant::now() + DISCOVER_EVERY;
    let mut ticks = interval(SAMPLE_EVERY);
    ticks.set_missed_tick_behavior(MissedTickBehavior::Skip);
    loop {
        ticks.tick().await;
        if Instant::now() >= next_discovery {
            (sources, placement) = discover(platform, &events).await;
            next_discovery = Instant::now() + DISCOVER_EVERY;
        }
        let now = Utc::now();
        let reading = readers::reading(platform, now).await;
        let gpus: Vec<_> = gpu::read(sources, platform == Platform::Macos)
            .await
            .into_iter()
            .filter_map(|found| placement.place(found))
            .collect();
        if let Some(point) = sampler.take(now, reading, &gpus)
            && events.send(Event::Point(point)).await.is_err()
        {
            return;
        }
    }
}

/// Finds the GPUs, tells the pusher, and returns which sources to keep reading.
async fn discover(
    platform: Platform,
    events: &mpsc::Sender<Event>,
) -> (gpu::Sources, gpu_series::Placement) {
    let found = gpu::read(gpu::Sources::ALL, platform == Platform::Macos).await;
    let placement = gpu_series::Placement::from_discovery(&found);
    let placed: Vec<_> = found
        .into_iter()
        .filter_map(|reading| placement.place(reading))
        .collect();
    if !placed.is_empty() {
        events
            .send(Event::Gpus(gpu_series::announce(&placed)))
            .await
            .ok();
    }
    (gpu::Sources::seen(&placed), placement)
}

/// Keeps what sampling sends and pushes it every 10 seconds, or after the wait a failure asks.
async fn push(mut pusher: Pusher, mut events: mpsc::Receiver<Event>) {
    let mut next = Instant::now() + RHYTHM;
    loop {
        tokio::select! {
            event = events.recv() => match event {
                Some(event) => pusher.accept(event),
                None => return,
            },
            () = sleep_until(next) => {
                let wait = pusher.push_due(Utc::now()).await;
                while let Ok(event) = events.try_recv() {
                    pusher.accept(event);
                }
                next = Instant::now() + wait;
            }
        }
    }
}
