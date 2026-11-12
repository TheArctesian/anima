//! Criterion 22 in one log (G-13, G-14): the whole hub, in process, takes every merged Phase 3
//! secret, on its success and its error paths, and its process log at TRACE for every target
//! holds none of them in any form (LOG-007, SEC-009, P3-004).
//!
//! A secret the operator types is a canary planted for this run, `canary-<kind>-<32 hex>`; a
//! secret the hub makes (a token, a code, a key) is read from its answer. The log is this
//! thread's: the test runs on one thread, so the hub's spawned tasks write here too. The
//! password hash runs on a blocking thread and is not seen; it logs nothing.

mod agents;
mod channels;
mod credentials;
mod edge;
mod forms;
mod keys;
mod operator;
mod vpn;

use anima_hub::events::process_log;
use chrono::{DateTime, DurationRound as _, TimeDelta, Utc};
use sqlx::PgPool;

use self::{edge::WholeHub, forms::Leak};
use crate::logs::Capture;

/// The secrets of one run, each with its kind and the part that only it holds.
#[derive(Default)]
pub struct Secrets(Vec<Secret>);

/// One secret: its kind, its whole text, and the random part that only this secret holds.
pub struct Secret {
    kind: &'static str,
    text: String,
    random: String,
}

impl Secrets {
    /// Plants a new canary of `kind` and returns it.
    pub fn plant(&mut self, kind: &'static str) -> String {
        self.plant_with(kind, |random| format!("canary-{kind}-{random}"))
    }

    /// Plants a secret of `kind` that `shape` makes from 32 random hex digits, for a secret
    /// whose form the hub checks; returns it.
    pub fn plant_with(&mut self, kind: &'static str, shape: impl Fn(&str) -> String) -> String {
        let mut bytes = [0_u8; 16];
        getrandom::fill(&mut bytes).unwrap();
        let random = bytes.into_iter().fold(String::new(), forms::hex);
        let text = shape(&random);
        // The longest start of the random digits that `shape` kept.
        let kept = (0..=random.len())
            .rev()
            .find(|&n| text.contains(&random[..n]));
        let random = random[..kept.unwrap_or(0)].to_owned();
        self.0.push(Secret {
            kind,
            text: text.clone(),
            random,
        });
        text
    }

    /// Keeps `text`, a secret the hub made, as one of `kind`.
    pub fn keep(&mut self, kind: &'static str, text: &str) {
        assert!(!text.is_empty(), "no {kind}");
        self.0.push(Secret {
            kind,
            text: text.to_owned(),
            random: text.to_owned(),
        });
    }

    /// The kinds kept, in order.
    fn kinds(&self) -> Vec<&'static str> {
        self.0.iter().map(|secret| secret.kind).collect()
    }
}

/// The database's clock to a multiple of 10 s: deliveries and pushed points go to partitions
/// that exist only near it.
async fn database_now(pool: &PgPool) -> DateTime<Utc> {
    let now: DateTime<Utc> = sqlx::query_scalar("select pg_catalog.now()")
        .fetch_one(pool)
        .await
        .unwrap();
    now.duration_trunc(TimeDelta::seconds(10)).unwrap()
}

/// The kinds every run must hold: each Phase 3 secret the hub takes or makes.
const EXPECTED: [&str; 35] = [
    "password",
    "wrong-password",
    "url-password",
    "browser-cookie",
    "totp-secret",
    "totp-secret-hex",
    "totp-code",
    "login-token",
    "api-key",
    "unknown-api-key",
    "agent-code",
    "agent-token",
    "agent-next-token",
    "webhook-path",
    "webhook-query",
    "webhook-header",
    "webhook-new-header",
    "refused-webhook-path",
    "ntfy-topic",
    "ntfy-token",
    "smtp-password",
    "smtp-recipient",
    "apprise-url",
    "apprise-key",
    "ssh-password",
    "ssh-key",
    "ssh-passphrase",
    "unreadable-ssh",
    "tested-ssh-password",
    "rdp-password",
    "vnc-password",
    "vpn-invite-key",
    "vpn-router-key",
    "vpn-unreadable-reply",
    "vpn-auth-id",
];

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn log_007_sec_009_p3_004_no_phase_3_secret_reaches_the_process_log(pool: PgPool) {
    let capture = Capture::default();
    let writer = capture.clone();
    let subscriber = process_log::subscriber("trace", move || writer.clone()).unwrap();
    // As in agent::secrecy: a second dispatcher keeps another test thread from caching this
    // test's callsites as disabled.
    let _second = tracing::Dispatch::new(tracing::subscriber::NoSubscriber::default());
    let guard = tracing::subscriber::set_default(subscriber);
    tracing::callsite::rebuild_interest_cache();

    let now = database_now(&pool).await;
    let mut secrets = Secrets::default();
    let broker = vpn::FakeBroker::start(&mut secrets);
    let feed = anima_hub::vpn::Feed::new();
    let hub = WholeHub::new(&pool, now, (feed.clone(), broker.broker())).await;
    let token = operator::sign_in(&hub, &pool, now, &mut secrets).await;
    keys::api_key(&hub, &token, now, &mut secrets).await;
    agents::enrol(&hub, &token, &mut secrets).await;
    channels::send_tests(&hub, &token, &mut secrets).await;
    credentials::set(&hub, &token, &mut secrets).await;
    vpn::change(&hub, &token, (&pool, &feed, &broker), now).await;
    // A line after the requests proves the log reached the buffer: an empty capture is no pass.
    tracing::error!("capture control line");
    drop(guard);

    let mut kinds = secrets.kinds();
    kinds.sort_unstable();
    let mut expected = EXPECTED.to_vec();
    expected.sort_unstable();
    assert_eq!(kinds, expected, "every secret was driven through the hub");
    let log = capture.text();
    assert!(log.contains("capture control line"), "the log was captured");
    assert!(log.contains(r#""level":"TRACE""#), "the log is at TRACE");
    for route in ["/graphql", "/metrics", "/agent/v1/enrol", "/agent/v1/renew"] {
        assert!(
            log.contains(&format!(r#""path":"{route}""#)),
            "{route} is logged"
        );
    }
    // The log is not printed: it holds the secrets when this fails.
    let leaks: Vec<Leak> = secrets
        .0
        .iter()
        .flat_map(|s| forms::leaks(s, &log))
        .collect();
    let leaks: Vec<String> = leaks.iter().map(ToString::to_string).collect();
    assert!(
        leaks.is_empty(),
        "secrets in the process log:\n{}",
        leaks.join("\n")
    );
}
