//! Tests of the dispatcher (D-16, NOT-001, NOT-002, NOT-004, CR-14 item 5, LOG-007, P3-003).
//!
//! Local TLS servers stand in for every target, and a dispatcher's ticks are driven with chosen
//! instants, so the retry schedule runs in no time. The key and every secret are synthetic.

mod decisions;
mod kinds;
mod leases;
mod requests;
mod retries;
mod secrets;

use std::{io::Write as _, sync::Arc};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use chrono::{DateTime, DurationRound, TimeDelta, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::{Dispatcher, Sender};
use crate::{
    credentials::{Secret, Vault},
    events::Bus,
    ids::{ChannelId, CheckId},
    notifications::{
        ChannelFields, Delivery, NewOutbox, NotificationKind, OutboxId,
        channels::{
            AppriseMode, AppriseSecrets, AppriseSettings, EmailSecrets, EmailSettings, NtfySecrets,
            NtfySettings, Secrets, Settings, WebhookMethod, WebhookSecrets, WebhookSettings,
        },
        create_partitions_ahead,
        egress::{
            Egress, SmtpSecurity,
            tests::{
                local,
                servers::{Behaviour, Server, certificate, serve},
            },
        },
        sealing, store,
    },
};

/// Every secret a fixture channel holds. None may appear in a delivery, an event or the log.
pub(in crate::notifications) const SECRETS: [&str; 7] = [
    "synthetic-path-secret",
    "synthetic-header-secret",
    "synthetic-topic-secret",
    "synthetic-token-secret",
    "synthetic-password-secret",
    "synthetic-apprise-secret",
    "synthetic-key-secret",
];

pub(in crate::notifications) const NO_CONTENT: &[u8] =
    b"HTTP/1.1 204 No Content\r\nContent-Length: 0\r\n\r\n";

pub(in crate::notifications) fn vault() -> Vault {
    let mut key_file = tempfile::NamedTempFile::new().unwrap();
    writeln!(key_file, "{}", STANDARD.encode([7_u8; 32])).unwrap();
    Vault::from_key_file(key_file.path()).unwrap()
}

pub(in crate::notifications) fn secret(text: &str) -> Secret {
    Secret::from(text.to_owned())
}

/// A dispatcher of its own on `pool`, sending through `egress`.
pub(super) fn dispatcher(pool: &PgPool, egress: Egress) -> Arc<Dispatcher> {
    Arc::new(Dispatcher {
        db: pool.clone(),
        bus: Bus::new(64),
        sender: Sender::new(vault(), egress),
    })
}

/// A TLS server for `localhost` answering as `behaviour` says with the acceptor given, and an
/// egress that trusts it.
pub(in crate::notifications) async fn target(
    behaviour: impl FnOnce(tokio_rustls::TlsAcceptor) -> Behaviour,
) -> (Server, Egress) {
    let (der, acceptor) = certificate("localhost");
    let server = serve(behaviour(acceptor)).await;
    (server, local().trusting(der))
}

/// An HTTPS server that answers every request with `reply`.
pub(in crate::notifications) async fn answering(reply: &[u8]) -> (Server, Egress) {
    let reply = reply.to_vec();
    target(|acceptor| Behaviour::Http(reply, Some(acceptor))).await
}

/// The database's time to the microsecond, with the delivery partitions around it.
pub(in crate::notifications) async fn start(pool: &PgPool) -> DateTime<Utc> {
    let now: DateTime<Utc> = sqlx::query_scalar("SELECT pg_catalog.now()")
        .fetch_one(pool)
        .await
        .unwrap();
    let now = now.duration_trunc(TimeDelta::microseconds(1)).unwrap();
    create_partitions_ahead(pool, now).await.unwrap();
    now
}

pub(in crate::notifications) fn origin(server: &Server) -> String {
    format!("https://localhost:{}", server.port)
}

pub(in crate::notifications) fn webhook(server: &Server) -> (Settings, Secrets) {
    let settings = Settings::Webhook(WebhookSettings {
        origin: origin(server),
        method: WebhookMethod::Post,
        header_names: vec!["X-Token".to_owned()],
    });
    let secrets = Secrets::Webhook(WebhookSecrets {
        url: secret(&format!("{}/hooks/synthetic-path-secret", origin(server))),
        headers: vec![secret("synthetic-header-secret")],
    });
    (settings, secrets)
}

pub(in crate::notifications) fn ntfy(server: &Server) -> (Settings, Secrets) {
    let settings = Settings::Ntfy(NtfySettings {
        server: origin(server),
        priority: 4,
        token_stored: true,
    });
    let secrets = Secrets::Ntfy(NtfySecrets {
        topic: secret("synthetic-topic-secret"),
        token: Some(secret("synthetic-token-secret")),
    });
    (settings, secrets)
}

pub(in crate::notifications) fn email(server: &Server) -> (Settings, Secrets) {
    let settings = Settings::Email(EmailSettings {
        host: "localhost".to_owned(),
        port: server.port,
        security: SmtpSecurity::Implicit,
        username: Some("anima".to_owned()),
        from: "anima@example.test".to_owned(),
        to: vec!["ops@example.test".to_owned()],
        password_stored: true,
    });
    let secrets = Secrets::Email(EmailSecrets {
        password: secret("synthetic-password-secret"),
    });
    (settings, secrets)
}

pub(in crate::notifications) fn apprise(server: &Server, urls: &[&str]) -> (Settings, Secrets) {
    let settings = Settings::Apprise(AppriseSettings {
        server: origin(server),
        mode: AppriseMode::StatelessUrls,
        tags: vec!["ops".to_owned()],
    });
    let urls = urls.iter().map(|url| secret(url)).collect();
    (settings, Secrets::Apprise(AppriseSecrets::Urls(urls)))
}

pub(in crate::notifications) fn apprise_key(server: &Server) -> (Settings, Secrets) {
    let settings = Settings::Apprise(AppriseSettings {
        server: origin(server),
        mode: AppriseMode::ConfigKey,
        tags: Vec::new(),
    });
    let key = AppriseSecrets::Key(secret("synthetic-key-secret"));
    (settings, Secrets::Apprise(key))
}

/// A stateless Apprise URL the hub permits, holding a synthetic secret.
pub(in crate::notifications) const DISCORD: &str =
    "discord://synthetic-apprise-secret/synthetic-apprise-secret";

/// Stores a channel named `name` with `settings` and `secrets` sealed for it.
pub(in crate::notifications) async fn channel(
    pool: &PgPool,
    name: &str,
    (settings, secrets): (Settings, Secrets),
) -> ChannelId {
    let id = ChannelId::from(Uuid::now_v7());
    let sealed = sealing::seal(&vault(), id, &settings, &secrets).unwrap();
    let fields = ChannelFields {
        name: name.to_owned(),
        kind: settings.kind(),
        on_by_default: false,
        settings: settings.to_stored(),
        secrets: Some(sealed),
        title_template: None,
        body_template: None,
    };
    let mut conn = pool.acquire().await.unwrap();
    store::insert_channel(&mut conn, id, &fields, Utc::now())
        .await
        .unwrap();
    id
}

/// Queues a Down notification of `check` for `channel`, due `at`.
pub(in crate::notifications) async fn queue(
    pool: &PgPool,
    channel: ChannelId,
    check: Option<CheckId>,
    at: DateTime<Utc>,
) -> OutboxId {
    let id = OutboxId::from(Uuid::now_v7());
    let row = NewOutbox {
        id,
        channel,
        check,
        kind: NotificationKind::Down,
        payload: serde_json::json!({
            "device": "pve-01",
            "check": "web",
            "status": "down",
            "previous": "up",
            "message": "timed out",
            "at": "2026-11-08T10:00:00Z",
        }),
    };
    let mut conn = pool.acquire().await.unwrap();
    store::insert_outbox(&mut conn, &row, at).await.unwrap();
    id
}

/// An outbox row as the tests read it.
#[derive(Debug, PartialEq, Eq)]
pub(in crate::notifications) struct Row {
    pub(in crate::notifications) attempts: i32,
    pub(in crate::notifications) sent: bool,
    pub(in crate::notifications) given_up: bool,
    pub(in crate::notifications) next_attempt_at: DateTime<Utc>,
}

pub(in crate::notifications) async fn row(pool: &PgPool, id: OutboxId) -> Row {
    let (attempts, sent, given_up, next_attempt_at) = sqlx::query_as(
        "SELECT attempts, sent_at IS NOT NULL, given_up_at IS NOT NULL, next_attempt_at \
         FROM notifications.outbox WHERE id = $1",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .unwrap();
    Row {
        attempts,
        sent,
        given_up,
        next_attempt_at,
    }
}

/// Every delivery, oldest first.
pub(in crate::notifications) async fn deliveries(pool: &PgPool) -> Vec<Delivery> {
    let mut all = store::deliveries_page(pool, None, None, 1000)
        .await
        .unwrap();
    all.reverse();
    all
}

/// The messages of every Warning event, oldest first.
pub(in crate::notifications) async fn warnings(pool: &PgPool) -> Vec<String> {
    sqlx::query_scalar(
        "SELECT message FROM events.event_log WHERE severity = 'warning' ORDER BY at, id",
    )
    .fetch_all(pool)
    .await
    .unwrap()
}

/// Every event message and audit change, as one text to search for secrets.
pub(in crate::notifications) async fn recorded(pool: &PgPool) -> String {
    let events: Vec<String> = sqlx::query_scalar("SELECT message FROM events.event_log")
        .fetch_all(pool)
        .await
        .unwrap();
    let audits: Vec<String> = sqlx::query_scalar("SELECT change::text FROM events.audit_log")
        .fetch_all(pool)
        .await
        .unwrap();
    [events, audits].concat().join("\n")
}

/// Panics when `text` holds any fixture secret.
pub(in crate::notifications) fn assert_no_secret(text: &str) {
    for secret in SECRETS {
        assert!(!text.contains(secret), "{secret} leaked: {text}");
    }
}
