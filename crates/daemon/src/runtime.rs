//! `anima-daemon run`: four tasks and the clock. One samples every 2 seconds and hands points
//! over; one probes the tool inventory at start and every 15 minutes (CAP-027); one reads the
//! session files of the AI agent programs at start and every 15 minutes (AIS-018); the other
//! keeps the buffer and pushes every 10 seconds, so a slow hub, probe or read never delays
//! sampling (MET-014, K8 Section 7). This is the I/O edge: clock, sleep, signals.

mod agent_usage;
#[cfg(unix)]
pub mod old_origin;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use chrono::Utc;
use tokio::sync::mpsc;
use tokio::time::{Instant, MissedTickBehavior, interval, sleep_until};
use uuid::Uuid;

use std::path::PathBuf;

use crate::config::home::{Home, HomeError};
use crate::hub::{HubClient, HubError, TIMEOUT};
use crate::keys::{keeper::Keeper, store::KeyStore};
use crate::push::pacing::RHYTHM;
use crate::push::{Event, Pusher};
use crate::readers::{self, Platform, Source, container::Container, gpu};
use crate::report;
use crate::sampling::Sampler;
use crate::sampling::batteries::BatteryWatch;
use crate::sampling::drives::DriveWatch;
use crate::sampling::gpu::Discovery;
use crate::sampling::inventory::InventoryWatch;
use crate::token_store::{TokenStore, TokenStoreError};
use agent_usage::{count, counting};

/// The time between samplings (MET-014).
const SAMPLE_EVERY: Duration = Duration::from_secs(2);

/// The time between two probe runs of the tool inventory (CAP-027).
const PROBE_EVERY: Duration = Duration::from_mins(15);

/// Points waiting to cross from sampling to the pusher: far more than a slow push can build up.
const CHANNEL_ROOM: usize = 64;

#[derive(Debug, thiserror::Error)]
pub enum RunError {
    #[error(
        "this daemon is not enrolled: run `anima-daemon enrol -` (the code on standard input) first"
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
    #[cfg(unix)]
    #[error("{0}")]
    Move(#[from] old_origin::MoveError),
    #[error("the socket of anima-link passes its check now; starting again to move to it")]
    LinkAppeared,
}

/// Samples and pushes until `stop` completes. With `hosted_hub`, the folder of the data of a
/// hosted hub, it reads that container and not the machine (HMD-071).
///
/// # Errors
/// [`RunError`] when the daemon is not enrolled or its files or client cannot be set up, or
/// when sampling ends before `stop` (the service manager then restarts the daemon).
pub async fn run(
    home: &Home,
    hosted_hub: Option<PathBuf>,
    stop: impl Future<Output = ()>,
) -> Result<(), RunError> {
    let (pusher, device, link) = prepare(home).await?;
    let (sender, receiver) = mpsc::channel(CHANNEL_ROOM);
    let probing = tokio::spawn(probe(Platform::current(), sender.clone()));
    let stopping = Arc::new(AtomicBool::new(false));
    let usage = tokio::spawn(count(
        counting(home, |name| std::env::var(name).ok()),
        sender.clone(),
        Arc::clone(&stopping),
    ));
    let source = hosted_hub.map_or(Source::Machine(Platform::current()), |data| {
        let processors = std::thread::available_parallelism().map_or(1, std::num::NonZero::get);
        let processors = u64::try_from(processors).unwrap_or(1);
        Source::HostedHub(Container::new(PathBuf::from("/"), data, processors))
    });
    let mut sampling = tokio::spawn(sample(source, sender));
    report::running(device);
    let outcome = tokio::select! {
        () = push(pusher, receiver) => Err(RunError::SamplingStopped),
        () = link_passed(&link) => Err(RunError::LinkAppeared),
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
    // Aborting drops a probe that is running, and its child with it (`kill_on_drop`).
    probing.abort();
    // A read in progress runs on a blocking thread, which an abort does not stop: the flag
    // ends it between two lines, so the daemon stops at once.
    stopping.store(true, Ordering::Relaxed);
    usage.abort();
    outcome
}

/// The pusher for the enrolled device, from the configuration and the token store. An old
/// enrolment moves to the socket of the link first, when it can ([`old_origin`]).
async fn prepare(home: &Home) -> Result<(Pusher, Uuid, LinkWatch), RunError> {
    let config = home.read_config()?.ok_or(RunError::NotEnrolled)?;
    let store = TokenStore::open(home);
    let credential = store.load()?.ok_or(RunError::NotEnrolled)?;
    #[cfg(unix)]
    let old_origin::Start {
        enrolled: old_origin::Enrolled { config, credential },
        watch: link,
        socket,
    } = old_origin::at_start(home, &store, old_origin::Enrolled { config, credential }).await?;
    #[cfg(windows)]
    let (link, socket) = ((), home.link_socket());
    if credential.hub != config.hub {
        return Err(RunError::OriginMismatch);
    }
    let hub = HubClient::reaching(config.hub.reach(), &socket, TIMEOUT)?;
    Ok((
        Pusher::new(hub, config.device, credential, store)
            .with_keys(Keeper::open(KeyStore::open(home))),
        config.device,
        link,
    ))
}

#[cfg(unix)]
type LinkWatch = old_origin::Watch;
#[cfg(windows)]
type LinkWatch = ();

/// Ends when the old enrolment of this daemon can move to the socket of the link.
#[cfg(unix)]
async fn link_passed(link: &LinkWatch) {
    link.passed().await;
}

#[cfg(windows)]
async fn link_passed(_: &LinkWatch) {
    std::future::pending::<()>().await;
}

/// Every 2 seconds: read, make a sample, and send the point each 10-second window closes.
async fn sample(mut source: Source, events: mpsc::Sender<Event>) {
    let mac = matches!(source, Source::Machine(Platform::Macos));
    let gpus = source.has_gpus();
    let mut sampler = Sampler::new();
    let mut drives = DriveWatch::new();
    let mut batteries = BatteryWatch::new();
    let mut watch = Watch::start(discover(mac, gpus, &events).await, Instant::now());
    let mut ticks = interval(SAMPLE_EVERY);
    ticks.set_missed_tick_behavior(MissedTickBehavior::Skip);
    loop {
        ticks.tick().await;
        if watch.due(Instant::now()) {
            watch.replace(discover(mac, gpus, &events).await, Instant::now());
        }
        let found = &watch.found;
        let now = Utc::now();
        let reading = source.reading(now, std::time::Instant::now()).await;
        if let Some(changed) = drives.changed(reading.drives.as_deref())
            && events.send(Event::Drives(changed)).await.is_err()
        {
            return;
        }
        if let Some(changed) = batteries.changed(reading.batteries.as_deref())
            && events.send(Event::Batteries(changed)).await.is_err()
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

/// At start and every 15 minutes: probes the tools and hands the inventory over when it
/// changed or went 24 hours unannounced (CAP-027). A probe run waits for the one before; a slow
/// one delays the next.
async fn probe(platform: Platform, events: mpsc::Sender<Event>) {
    let mut watch = InventoryWatch::new();
    let mut ticks = interval(PROBE_EVERY);
    ticks.set_missed_tick_behavior(MissedTickBehavior::Delay);
    loop {
        ticks.tick().await;
        let read = readers::inventory(platform).await;
        if let Some(changed) = watch.changed(read, std::time::Instant::now())
            && events.send(Event::Inventory(changed)).await.is_err()
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

/// Finds the GPUs and tells the pusher the whole set, `[]` included (K8 Section 4). With
/// `look` false (a hosted hub) it finds none.
async fn discover(mac: bool, look: bool, events: &mpsc::Sender<Event>) -> Discovery {
    let read = if look {
        gpu::read(gpu::Sources::ALL, mac).await
    } else {
        Vec::new()
    };
    let found = Discovery::new(read);
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
    use chrono::DateTime;
    use daemon_core::wire::DaemonToken;

    use super::*;
    use crate::config::Config;
    use crate::config::origin::Origin;
    use crate::token_store::Credential;

    fn setup(issued_by: &str) -> (tempfile::TempDir, Home) {
        enrolled("https://hub.example", issued_by)
    }

    /// A daemon's folder as an enrolment at `hub` left it, with a token that `issued_by` gave.
    fn enrolled(hub: &str, issued_by: &str) -> (tempfile::TempDir, Home) {
        let dir = tempfile::tempdir().expect("a folder");
        let home = Home::at(dir.path().join("daemon"));
        let hub = Origin::parse_kept(hub).expect("origin");
        let config = Config {
            hub,
            device: Uuid::from_u128(1),
            device_name: "pop-os".to_owned(),
        };
        home.write_config(&config).expect("config");
        let credential = Credential {
            token: DaemonToken::from_random([1; 32]),
            expires_at: DateTime::UNIX_EPOCH,
            hub: Origin::parse_kept(issued_by).expect("origin"),
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
        let sampling = tokio::spawn(sample(Source::Machine(Platform::Linux), sender));
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
    fn rol_018_a_daemon_enrolled_at_a_port_of_the_link_starts_after_its_update() {
        let (_dir, home) = enrolled("http://127.0.0.1:8443", "http://127.0.0.1:8443");
        assert!(prepare(&home).is_ok(), "the old files still read");
        let (_dir, home) = enrolled("http://127.0.0.1:8443", "http://127.0.0.1:9000");
        assert!(matches!(prepare(&home), Err(RunError::OriginMismatch)));
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
