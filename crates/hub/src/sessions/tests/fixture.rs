//! The world both tunnels' wire tests start from, and what a session leaves behind: the
//! operator, `docker-host` with an accepted host key and a stored password, the display a
//! client asks for, the end row with its audit entries and events, and a capture of the
//! process log (LOG-007).

use std::{
    io,
    sync::{Arc, Mutex},
    time::Duration,
};

use serde_json::Value;
use sqlx::{PgPool, postgres::PgPoolOptions};
use tokio::sync::oneshot;

use crate::{
    events::{ClientAddr, Operator},
    ids::{OperatorId, SessionId},
    sessions::{
        DisplayRequest, Live, PendingTicket, Protocol, StartRequest,
        tickets::{TOKEN_BYTES, TicketToken},
    },
};

/// The world `ssh` tests start from is this one's start too.
pub(in crate::sessions) use crate::ssh::tests::fixture::{
    DOCKER_HOST, LOOPBACK, OPERATOR, PASSWORD, add_operator, docker_host, move_device, now,
    prepare, vault,
};

pub(in crate::sessions) fn operator() -> Operator {
    Operator {
        id: OPERATOR.parse::<OperatorId>().unwrap(),
        name: "operator".to_owned(),
        actor: crate::events::Actor::for_tests(),
    }
}

pub(in crate::sessions) fn address(text: &str) -> ClientAddr {
    ClientAddr(text.parse().unwrap())
}

/// A pool of its own on the test database for the route. sqlx test pools share 20 connections
/// across all running tests; the session task writes while the test reads, and two
/// connections per test starve that share when many tests run at once.
pub(in crate::sessions) async fn own_pool(pool: &PgPool) -> PgPool {
    PgPoolOptions::new()
        .max_connections(2)
        .connect_with((*pool.connect_options()).clone())
        .await
        .unwrap()
}

/// A loopback port nothing listens on, as a device that is still waking up, and the sender
/// that wakes it: once the test sends, the port accepts, and forwards each connection to the
/// loopback port `forward` when there is one. The device answers when the test says, not
/// after a time, so a loaded machine cannot move the answer across an opening limit. Dropping
/// the sender leaves the device asleep.
pub(in crate::sessions) fn waking_port(forward: Option<u16>) -> (u16, oneshot::Sender<()>) {
    let reserved = std::net::TcpListener::bind((LOOPBACK, 0)).unwrap();
    let port = reserved.local_addr().unwrap().port();
    drop(reserved);
    let (answer, answered) = oneshot::channel();
    tokio::spawn(async move {
        if answered.await.is_err() {
            return;
        }
        let listener = tokio::net::TcpListener::bind((LOOPBACK, port))
            .await
            .unwrap();
        while let Ok((mut client, _)) = listener.accept().await {
            let Some(forward) = forward else { continue };
            tokio::spawn(async move {
                let mut device = tokio::net::TcpStream::connect((LOOPBACK, forward)).await?;
                tokio::io::copy_bidirectional(&mut client, &mut device).await
            });
        }
    });
    (port, answer)
}

/// Wakes the device `pause` after the opening began, then returns. The opening signals
/// `began` in the poll that starts its limit, so the device answers at least `pause` after the
/// limit started, however late the machine schedules either side.
pub(in crate::sessions) async fn answer_after(
    began: oneshot::Receiver<()>,
    pause: Duration,
    answer: oneshot::Sender<()>,
) {
    began.await.unwrap();
    tokio::time::sleep(pause).await;
    answer.send(()).unwrap();
}

/// Puts `session`'s pending ticket back as one whose start sent a Wake-on-LAN packet, under
/// a new token, and returns the token's text.
pub(in crate::sessions) fn reissued_woken(live: &Live, session: SessionId) -> String {
    let ticket = live.withdraw(session).unwrap();
    let token = TicketToken::from_random([3; TOKEN_BYTES]);
    let woken = PendingTicket {
        wake_sent: true,
        ..ticket
    };
    live.issue(token.hash(), woken).unwrap();
    token.expose().to_owned()
}

/// The SSH session a client asks for on `docker-host`.
pub(in crate::sessions) fn request() -> StartRequest {
    StartRequest {
        device: docker_host(),
        protocol: Protocol::Ssh,
        display: DisplayRequest {
            width_px: 1280,
            height_px: 800,
            dpi: 96,
            timezone: Some("Europe/Berlin".to_owned()),
        },
        wake: None,
    }
}

/// What a session left behind once its row ended.
#[derive(Debug)]
pub(in crate::sessions) struct Ended {
    pub(in crate::sessions) reason: String,
    /// `(operator name, client address, change)` of each `session.end` entry.
    pub(in crate::sessions) audits: Vec<(String, String, Value)>,
    /// `(severity, source, device name, message)` of each event.
    pub(in crate::sessions) events: Vec<(String, String, Option<String>, String)>,
}

/// Waits up to five seconds for the session's row to end, then reads what the end wrote.
pub(in crate::sessions) async fn ended(pool: &PgPool, session: SessionId) -> Ended {
    for _ in 0..250 {
        let reason: Option<String> =
            sqlx::query_scalar("select end_reason from sessions.sessions where id = $1")
                .bind(session)
                .fetch_one(pool)
                .await
                .unwrap();
        if let Some(reason) = reason {
            // The audit entry and event commit with the row.
            return Ended {
                reason,
                audits: end_audits(pool, session).await,
                events: events(pool).await,
            };
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("the session did not end");
}

async fn end_audits(pool: &PgPool, session: SessionId) -> Vec<(String, String, Value)> {
    sqlx::query_as(
        "select operator_name, host(client_addr), change from events.audit_log \
         where action = 'end' and target_type = 'session' and target_id = $1",
    )
    .bind(session.to_string())
    .fetch_all(pool)
    .await
    .unwrap()
}

async fn events(pool: &PgPool) -> Vec<(String, String, Option<String>, String)> {
    sqlx::query_as("select severity, source, device_name, message from events.event_log")
        .fetch_all(pool)
        .await
        .unwrap()
}

/// The end row's reason and its one audit entry (by `client`) and one event (`severity`).
pub(in crate::sessions) fn ended_once(ended: &Ended, reason: &str, severity: &str, client: &str) {
    assert_eq!(ended.reason, reason);
    let audit = (
        "operator".to_owned(),
        client.to_owned(),
        serde_json::json!({ "reason": reason }),
    );
    assert_eq!(ended.audits, [audit], "one session.end entry");
    assert_eq!(ended.events.len(), 1, "one event: {:?}", ended.events);
    let (got_severity, source, device, _) = &ended.events[0];
    assert_eq!(
        (got_severity.as_str(), source.as_str()),
        (severity, "session")
    );
    assert_eq!(device.as_deref(), Some("docker-host"));
}

/// The `(operator_name, actor_kind)` of the session's `session.end` entries and the
/// `actor_kind` of every event (LOG-012, LOG-013).
pub(in crate::sessions) async fn end_kinds(
    pool: &PgPool,
    session: SessionId,
) -> (Vec<(String, Option<String>)>, Vec<Option<String>>) {
    let audits = sqlx::query_as(
        "select operator_name, actor_kind from events.audit_log \
         where action = 'end' and target_type = 'session' and target_id = $1",
    )
    .bind(session.to_string())
    .fetch_all(pool)
    .await
    .unwrap();
    let events = sqlx::query_scalar("select actor_kind from events.event_log")
        .fetch_all(pool)
        .await
        .unwrap();
    (audits, events)
}

/// Every text the database holds about sessions, audit and events, as one string.
pub(in crate::sessions) async fn stored_text(pool: &PgPool) -> String {
    let rows: Vec<String> = sqlx::query_scalar(
        "select row_to_json(s)::text from sessions.sessions s \
         union all select row_to_json(a)::text from events.audit_log a \
         union all select row_to_json(e)::text from events.event_log e",
    )
    .fetch_all(pool)
    .await
    .unwrap();
    rows.join("\n")
}

/// Collects everything the process log writes.
#[derive(Clone, Default)]
pub(in crate::sessions) struct Capture(pub(in crate::sessions) Arc<Mutex<Vec<u8>>>);

impl io::Write for Capture {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
