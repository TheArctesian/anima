//! `anima-agent run`: two tasks and the clock. One samples every 2 seconds and hands points
//! over; the other keeps the buffer and pushes every 10 seconds, so a slow hub never delays
//! sampling (MET-014, K8 Section 7). This is the I/O edge: clock, sleep, signals.

use std::time::Duration;

use chrono::Utc;
use tokio::sync::mpsc;
use tokio::time::{Instant, MissedTickBehavior, interval, sleep_until};
use uuid::Uuid;

use crate::config::home::{Home, HomeError};
use crate::hub::{HubClient, HubError, TIMEOUT};
use crate::push::pacing::RHYTHM;
use crate::push::{Event, Pusher};
use crate::readers::{self, Platform, gpu};
use crate::report;
use crate::sampling::Sampler;
use crate::sampling::drives::DriveWatch;
use crate::sampling::gpu::Discovery;
use crate::token_store::{TokenStore, TokenStoreError};

/// The time between samplings (MET-014).
const SAMPLE_EVERY: Duration = Duration::from_secs(2);

/// Points waiting to cross from sampling to the pusher: far more than a slow push can build up.
const CHANNEL_ROOM: usize = 64;

#[derive(Debug, thiserror::Error)]
pub enum RunError {
    #[error(
        "this agent is not enrolled: run `anima-agent enrol -` (the code on standard input) first"
    )]
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
            report::sampling_ended(&ended);
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
    if credential.hub != config.hub {
        return Err(RunError::OriginMismatch);
    }
    let hub = HubClient::new(config.hub.url().clone(), TIMEOUT)?;
    Ok((
        Pusher::new(hub, config.device, credential, store),
        config.device,
    ))
}

/// Every 2 seconds: read, make a sample, and send the point each 10-second window closes.
async fn sample(platform: Platform, events: mpsc::Sender<Event>) {
    let mac = platform == Platform::Macos;
    let mut sampler = Sampler::new();
    let mut drives = DriveWatch::new();
    let mut watch = Watch::start(discover(mac, &events).await, Instant::now());
    let mut ticks = interval(SAMPLE_EVERY);
    ticks.set_missed_tick_behavior(MissedTickBehavior::Skip);
    loop {
        ticks.tick().await;
        if watch.due(Instant::now()) {
            watch.replace(discover(mac, &events).await, Instant::now());
        }
        let found = &watch.found;
        let now = Utc::now();
        let reading = readers::reading(platform, now).await;
        if let Some(changed) = drives.changed(reading.drives.as_deref())
            && events.send(Event::Drives(changed)).await.is_err()
        {
            return;
        }
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

/// The GPUs found last, and when to look again.
struct Watch {
    found: Discovery,
    next: Instant,
}

impl Watch {
    fn start(found: Discovery, now: Instant) -> Self {
        let next = found.next_at(now, None);
        Self { found, next }
    }

    fn due(&self, now: Instant) -> bool {
        now >= self.next
    }

    /// Keeps `again`, and looks again soon only if it lost GPUs (MET-006).
    fn replace(&mut self, again: Discovery, now: Instant) {
        self.next = again.next_at(now, Some(&self.found));
        self.found = again;
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
            hub: Origin::parse(issued_by).expect("origin"),
        };
        TokenStore::open(&home)
            .save_new(&credential)
            .expect("token");
        (dir, home)
    }

    /// The first thing sampling sends is the GPU list, `[]` included (K8 Section 4): the hub
    /// is told the set before any point arrives.
    #[tokio::test]
    async fn k8_4_sampling_starts_by_announcing_the_gpus() {
        let (sender, mut events) = mpsc::channel(CHANNEL_ROOM);
        let sampling = tokio::spawn(sample(Platform::Linux, sender));
        let first = tokio::time::timeout(Duration::from_secs(20), events.recv()).await;
        sampling.abort();
        assert!(matches!(first, Ok(Some(Event::Gpus(_)))));
    }

    #[test]
    fn met_006_a_stable_discovery_is_not_repeated_on_the_next_tick() {
        let now = Instant::now();
        let mut watch = Watch::start(Discovery::new(Vec::new()), now);
        assert!(!watch.due(now + SAMPLE_EVERY), "no GPU: soon, not at once");
        watch.replace(Discovery::new(Vec::new()), now);
        assert!(!watch.due(now + SAMPLE_EVERY));
        assert!(watch.due(now + Duration::from_hours(25)));
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
