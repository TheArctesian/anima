use std::{
    pin::{Pin, pin},
    sync::Arc,
    task::{Context, Poll},
    time::Duration,
};

use async_graphql::{
    futures_util::{Sink, Stream, StreamExt as _, future, sink, stream},
    http::WsMessage,
};
use axum::extract::ws::Message;
use chrono::{DateTime, TimeDelta, TimeZone as _, Utc};
use sqlx::PgPool;
use tokio::{sync::oneshot, time::Instant};
use uuid::Uuid;

use super::{
    Closing, INIT_TIMEOUT, Lease, SocketAccess, Verdict, WRITE_TIMEOUT, converse, lease_of,
    next_check, signed_in,
};
use crate::clock::Clock;

fn now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2030, 5, 5, 12, 0, 0).unwrap()
}

const RECHECK: Duration = Duration::from_secs(15);

#[test]
fn r_020_a_lease_is_checked_every_interval_and_at_its_expiry() {
    let far = now() + TimeDelta::days(30);
    assert_eq!(next_check(far, now(), RECHECK), Some(RECHECK));
    let soon = now() + TimeDelta::milliseconds(1500);
    assert_eq!(
        next_check(soon, now(), RECHECK),
        Some(Duration::from_millis(1500))
    );
    let exactly = now() + TimeDelta::seconds(15);
    assert_eq!(next_check(exactly, now(), RECHECK), Some(RECHECK));
}

#[test]
fn r_020_a_lease_ends_at_its_expiry() {
    assert_eq!(next_check(now(), now(), RECHECK), None);
    assert_eq!(
        next_check(now() - TimeDelta::seconds(1), now(), RECHECK),
        None
    );
    let just_before = now() + TimeDelta::microseconds(1);
    assert_eq!(
        next_check(just_before, now(), RECHECK),
        Some(Duration::from_micros(1))
    );
}

#[test]
fn r_020_only_a_granted_lease_opens_the_connection() {
    let lease = Lease {
        token: Uuid::now_v7(),
        expires_at: now(),
    };
    assert_eq!(lease_of(Some(Verdict::Leased(lease))), Ok(lease));
    assert_eq!(lease_of(Some(Verdict::Refused)), Err(Closing::Forbidden));
    assert_eq!(lease_of(None), Err(Closing::Forbidden));
    assert_eq!(
        lease_of(Some(Verdict::Unavailable)),
        Err(Closing::Unavailable)
    );
}

#[test]
fn r_020_close_frames_have_fixed_reasons_and_graphql_ws_codes() {
    let frames = [
        (Closing::ClientLeft, None),
        (Closing::Stalled, None),
        (
            Closing::DuplicateOperation,
            Some((4409, "Subscriber already exists")),
        ),
        (
            Closing::TooManyOperations,
            Some((1008, "Too many operations")),
        ),
        (Closing::Unreadable, Some((1008, "Message refused"))),
        (
            Closing::InitTimeout,
            Some((4408, "Connection initialisation timeout")),
        ),
        (Closing::Forbidden, Some((4403, "Forbidden"))),
        (
            Closing::Unavailable,
            Some((1011, "Sign-in cannot be checked")),
        ),
        (Closing::Protocol(1002), Some((4400, "Invalid message"))),
        (Closing::Protocol(1011), Some((4401, "Unauthorized"))),
        (
            Closing::Protocol(4429),
            Some((4429, "Too many initialisation requests")),
        ),
        (Closing::Protocol(3008), Some((3008, ""))),
    ];
    for (closing, frame) in frames {
        assert_eq!(closing.frame(), frame, "{closing:?}");
    }
}

/// A client that never reads: every write stays pending.
struct Unread;

impl Sink<Message> for Unread {
    type Error = axum::Error;

    fn poll_ready(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Pending
    }

    fn start_send(self: Pin<&mut Self>, _: Message) -> Result<(), Self::Error> {
        Ok(())
    }

    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Pending
    }

    fn poll_close(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Pending
    }
}

/// The hub clock moved by tokio's paused time, so that expiry follows the test's timeline.
struct PausedClock {
    start: Instant,
}

impl Clock for PausedClock {
    fn now(&self) -> DateTime<Utc> {
        now() + TimeDelta::from_std(self.start.elapsed()).unwrap()
    }
}

/// Checks that never reach a database: these connections end before any look-up.
fn unreachable_access() -> SocketAccess {
    SocketAccess {
        db: PgPool::connect_lazy("postgres://anima@127.0.0.1:9/never").unwrap(),
        clock: Arc::new(PausedClock {
            start: Instant::now(),
        }),
        client: None,
        cookie: None,
        recheck: Duration::from_secs(15),
        init_timeout: INIT_TIMEOUT,
    }
}

/// One protocol message for the client, then nothing more.
fn one_message_then_silence() -> impl Stream<Item = WsMessage> {
    stream::once(future::ready(WsMessage::Text(r#"{"type":"pong"}"#.into())))
        .chain(stream::pending())
}

/// The bound that keeps a broken connection from hanging the test.
const TEST_BOUND: Duration = Duration::from_secs(600);

async fn sign_in_ends(client: impl Sink<Message> + Unpin) -> (Closing, Duration) {
    let started = Instant::now();
    let (_verdict, decided) = oneshot::channel::<Verdict>();
    let mut client = client;
    let mut protocol = pin!(one_message_then_silence());
    let ended = signed_in(&mut client, &mut protocol, decided, Duration::from_secs(3));
    let ended = tokio::time::timeout(TEST_BOUND, ended)
        .await
        .expect("bounded");
    (ended.unwrap_err(), started.elapsed())
}

async fn lease_ends(client: impl Sink<Message> + Unpin, left: TimeDelta) -> (Closing, Duration) {
    let started = Instant::now();
    let (verdict, decided) = oneshot::channel();
    let lease = Lease {
        token: Uuid::now_v7(),
        expires_at: now() + left,
    };
    verdict.send(Verdict::Leased(lease)).unwrap();
    let access = unreachable_access();
    let mut client = client;
    let ended = converse(&mut client, one_message_then_silence(), decided, &access);
    let ended = tokio::time::timeout(TEST_BOUND, ended)
        .await
        .expect("bounded");
    (ended, started.elapsed())
}

#[tokio::test(start_paused = true)]
async fn h_6_a_client_that_does_not_read_still_meets_the_sign_in_deadline() {
    let ended = sign_in_ends(Unread).await;
    assert_eq!(ended, (Closing::InitTimeout, Duration::from_secs(3)));
}

#[tokio::test(start_paused = true)]
async fn r_020_a_client_that_does_not_read_still_loses_its_lease_at_expiry() {
    let ended = lease_ends(Unread, TimeDelta::seconds(1)).await;
    assert_eq!(ended, (Closing::Forbidden, Duration::from_secs(1)));
}

#[tokio::test(start_paused = true)]
async fn h_6_a_write_the_client_does_not_take_closes_the_connection() {
    let ended = lease_ends(Unread, TimeDelta::days(1)).await;
    assert_eq!(ended, (Closing::Stalled, WRITE_TIMEOUT));
}

#[tokio::test(start_paused = true)]
async fn r_020_a_client_that_reads_ends_the_same_way() {
    let ended = sign_in_ends(sink::drain()).await;
    assert_eq!(ended, (Closing::InitTimeout, Duration::from_secs(3)));
    let ended = lease_ends(sink::drain(), TimeDelta::seconds(1)).await;
    assert_eq!(ended, (Closing::Forbidden, Duration::from_secs(1)));
}
