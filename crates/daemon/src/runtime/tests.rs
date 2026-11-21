//! The run on Linux: the first announcement, the watch of the GPUs and the files that
//! `prepare` reads.

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

#[tokio::test]
async fn rol_018_a_daemon_enrolled_at_a_port_of_the_link_starts_after_its_update() {
    let (_dir, home) = enrolled("http://127.0.0.1:8443", "http://127.0.0.1:8443");
    assert!(prepare(&home).await.is_ok(), "the old files still read");
    let (_dir, home) = enrolled("http://127.0.0.1:8443", "http://127.0.0.1:9000");
    assert!(matches!(
        prepare(&home).await,
        Err(RunError::OriginMismatch)
    ));
}

#[tokio::test]
async fn met_020_a_token_issued_by_another_hub_is_not_sent_to_the_configured_one() {
    let (_dir, home) = setup("https://other.example");
    assert!(matches!(
        prepare(&home).await,
        Err(RunError::OriginMismatch)
    ));
}

#[tokio::test]
async fn met_020_a_token_issued_by_the_configured_hub_is_used() {
    let (_dir, home) = setup("https://hub.example");
    assert!(prepare(&home).await.is_ok());
}
