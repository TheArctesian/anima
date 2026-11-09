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
use crate::sampling::Sampler;
use crate::sampling::gpu::Discovery;
use crate::token_store::{TokenStore, TokenStoreError};

/// The time between samplings (MET-014).
const SAMPLE_EVERY: Duration = Duration::from_secs(2);

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
    #[error("sampling stopped")]
    SamplingStopped,
    #[error("the token was issued by another hub than the configuration names")]
    OriginMismatch,
}

/// Samples and pushes until `stop` completes.
///
/// # Errors
/// [`RunError`] when the agent is not enrolled or its files or client cannot be set up, or
/// when sampling ends before `stop` (the service manager then restarts the agent).
pub async fn run(home: &Home, stop: impl Future<Output = ()>) -> Result<(), RunError> {
    let (pusher, device) = prepare(home)?;
    let (sender, receiver) = mpsc::channel(CHANNEL_ROOM);
    let mut sampling = tokio::spawn(sample(Platform::current(), sender));
    report::running(device);
    let outcome = tokio::select! {
        () = push(pusher, receiver) => Err(RunError::SamplingStopped),
        ended = &mut sampling => {
            if ended.is_err() {
                report::sampling_panicked();
            } else {
                report::sampling_stopped();
            }
            Err(RunError::SamplingStopped)
        }
        () = stop => {
            report::stopping();
            Ok(())
        }
    };
    sampling.abort();
    outcome
}

/// The pusher for the enrolled device, from the configuration and the token store.
fn prepare(home: &Home) -> Result<(Pusher, Uuid), RunError> {
    let config = home.read_config()?.ok_or(RunError::NotEnrolled)?;
    let store = TokenStore::open(home);
    let credential = store.load()?.ok_or(RunError::NotEnrolled)?;
    if credential
        .hub
        .as_ref()
        .is_some_and(|issued| *issued != config.hub)
    {
        return Err(RunError::OriginMismatch);
    }
    let hub = HubClient::new(config.hub.url().clone())?;
    Ok((
        Pusher::new(hub, config.device, credential, store),
        config.device,
    ))
}

/// Every 2 seconds: read, make a sample, and send the point each 10-second window closes.
async fn sample(platform: Platform, events: mpsc::Sender<Event>) {
    let mac = platform == Platform::Macos;
    let mut sampler = Sampler::new();
    let mut found = discover(mac, &events).await;
    let mut next_discovery = Instant::now() + found.next_after(None);
    let mut ticks = interval(SAMPLE_EVERY);
    ticks.set_missed_tick_behavior(MissedTickBehavior::Skip);
    loop {
        ticks.tick().await;
        if Instant::now() >= next_discovery {
            let again = discover(mac, &events).await;
            next_discovery = Instant::now() + again.next_after(Some(&found));
            found = again;
        }
        let now = Utc::now();
        let reading = readers::reading(platform, now).await;
        let gpus = found
            .placement
            .place_all(gpu::read(found.sources, mac).await);
        if let Some(point) = sampler.take(now, reading, &gpus)
            && events.send(Event::Point(point)).await.is_err()
        {
            return;
        }
    }
}

/// Finds the GPUs and tells the pusher the whole set, `[]` included (K8 Section 4).
async fn discover(mac: bool, events: &mpsc::Sender<Event>) -> Discovery {
    let found = Discovery::new(gpu::read(gpu::Sources::ALL, mac).await);
    events.send(Event::Gpus(found.announce.clone())).await.ok();
    found
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

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use agent_core::wire::AgentToken;
    use chrono::DateTime;

    use super::*;
    use crate::config::Config;
    use crate::config::origin::Origin;
    use crate::token_store::Credential;

    fn setup(issued_by: &str) -> (tempfile::TempDir, Home) {
        let dir = tempfile::tempdir().expect("a folder");
        let home = Home::at(dir.path().join("agent"));
        let hub = Origin::parse("https://hub.example").expect("origin");
        let config = Config {
            hub,
            device: Uuid::from_u128(1),
            device_name: "pop-os".to_owned(),
        };
        home.write_config(&config).expect("config");
        let credential = Credential {
            token: AgentToken::from_random([1; 32]),
            expires_at: DateTime::UNIX_EPOCH,
            hub: Some(Origin::parse(issued_by).expect("origin")),
        };
        TokenStore::open(&home)
            .save_new(&credential)
            .expect("token");
        (dir, home)
    }

    #[test]
    fn met_020_a_token_issued_by_another_hub_is_not_sent_to_the_configured_one() {
        let (_dir, home) = setup("https://other.example");
        assert!(matches!(prepare(&home), Err(RunError::OriginMismatch)));
    }

    #[test]
    fn met_020_a_token_issued_by_the_configured_hub_is_used() {
        let (_dir, home) = setup("https://hub.example");
        assert!(prepare(&home).is_ok());
    }
}
