//! No code and no token reaches a log line or an error message (LOG-007, MET-013).

use std::sync::{Arc, LazyLock, Mutex};

use agent_core::wire::{EnrolmentCode, RefusalCode};
use anima_agent::cli::{self, CliError};
use anima_agent::config::home::Home;
use anima_agent::config::origin::Origin;
use anima_agent::hub::{HubClient, TIMEOUT};
use anima_agent::hub::answer::Cause;
use anima_agent::hub::enrolment::{EnrolError, Enrolment};
use anima_agent::push::Event;
use anima_agent::runtime::RunError;
use anima_agent::token_store::TokenStore;

use crate::fake_hub::{FakeHub, Reply};
use crate::kit::{at, points, rig, token};

const CODE: &str = "anima_enrol_AAAAAAAAAAAAAAAAAAAAAA";

/// One global subscriber for the whole test binary: a per-thread one misses callsites that
/// another test thread registered first, so a test would see no line at all.
static LOG: LazyLock<Captured> = LazyLock::new(|| {
    let captured = Captured::default();
    let writer = captured.clone();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(move || writer.clone())
        .with_env_filter(tracing_subscriber::EnvFilter::new("trace"))
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("the only global subscriber");
    captured
});

#[derive(Clone, Default)]
struct Captured(Arc<Mutex<Vec<u8>>>);

impl Captured {
    fn text(&self) -> String {
        String::from_utf8_lossy(&self.0.lock().expect("lock")).into_owned()
    }
}

impl std::io::Write for Captured {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().expect("lock").extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// At TRACE, as `RUST_LOG=trace` gives the binary, through enrolment, a refused push, a
/// renewal and a recovery.
#[tokio::test]
async fn log_007_trace_logs_hold_no_token_and_no_code() {
    LazyLock::force(&LOG);
    let renewed_before = LOG.text().matches("renewed").count();
    let hub = FakeHub::start().await;
    let enrolled = serde_json::json!({
        "device": "00000000-0000-0000-0000-0000000000d1", "deviceName": "pop-os",
        "token": token(3).expose(), "expiresAt": "2027-11-08T10:00:00Z",
    });
    hub.queue_enrol(Reply::json(200, &enrolled.to_string()));
    let dir = tempfile::tempdir().expect("a folder");
    let home = Home::at(dir.path().join("agent"));
    let store = TokenStore::open(&home);
    let client = HubClient::new(hub.url.clone(), TIMEOUT).expect("a client");
    Enrolment {
        hub: &client,
        origin: Origin::parse("https://hub.example").expect("origin"),
        home: &home,
        store: &store,
        now: at(0),
    }
    .spend(EnrolmentCode::parse(CODE).expect("code"))
    .await
    .expect("enrolled");
    let mut rig = rig(&hub, 1, chrono::TimeDelta::days(10));
    hub.queue_push([
        Reply::refuse(RefusalCode::Unavailable),
        Reply::refuse(RefusalCode::TokenRefused),
    ]);
    hub.queue_renew(Reply::json(
        200,
        &format!(
            r#"{{"token":"{}","expiresAt":"2027-11-08T10:00:00Z"}}"#,
            token(9).expose()
        ),
    ));
    for point in points(1, 3) {
        rig.pusher.accept(Event::Point(point));
    }
    for round in 0..4 {
        rig.pusher.push_due(at(40 + round * 700)).await;
    }
    let text = LOG.text();
    assert!(
        text.matches("renewed").count() > renewed_before,
        "the renewal was logged: {text}"
    );
    assert!(!text.contains("anima_agent_"), "a token reached the log");
    assert!(!text.contains("anima_enrol_"), "a code reached the log");
}

#[test]
fn log_007_no_error_message_holds_a_token_or_a_code() {
    let mut messages = vec![
        EnrolError::Refused.to_string(),
        EnrolError::Failed(Cause::Network).to_string(),
        EnrolError::Failed(Cause::Status(401)).to_string(),
        RunError::NotEnrolled.to_string(),
        RunError::SamplingStopped.to_string(),
        RunError::OriginMismatch.to_string(),
        CliError::Usage.to_string(),
        CliError::NotACode.to_string(),
    ];
    for cause in [
        Cause::Network,
        Cause::Timeout,
        Cause::Status(500),
        Cause::Unreadable,
    ] {
        messages.push(cause.to_string());
    }
    let Err(bad) = cli::parse(["enrol".to_owned(), format!("{CODE}x")]) else {
        panic!("a malformed code is refused");
    };
    messages.push(bad.to_string());
    for message in messages {
        assert!(!message.contains(CODE), "{message}");
        assert!(!message.contains(token(1).expose()), "{message}");
    }
}
