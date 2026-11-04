//! A `graphql-transport-ws` connection on `/graphql` that serves only while the token it signed
//! in with holds (K7 "Authentication transport", R-020, API-003, H-6(b)).
//!
//! The client signs in with the `connection_init` payload `{ "authorization": "Bearer <token>" }`;
//! the URL is never read. A missing or invalid token closes the socket with 4403. After sign-in
//! the socket closes with 4403 at the token's `expires_at` by the hub clock, and at the next
//! re-check (every 15 seconds) after the token is deleted by `logout`, `operator set` or
//! `prune`. Re-checking the stored token, rather than a bus signal, also sees `operator set`,
//! which runs in another process, and needs no new `events::Signal` (K3).
//!
//! Every write is bounded: it gives way to the sign-in deadline or the lease's end, and a write
//! that takes longer than [`WRITE_TIMEOUT`] closes the socket, so a client that stops reading
//! cannot hold it open (H-6(b)).

mod intake;

use std::{
    future::Future,
    pin::{Pin, pin},
    sync::Arc,
    time::Duration,
};

use async_graphql::{
    Data, Executor,
    futures_util::{Sink, SinkExt as _, Stream, StreamExt as _},
    http::{WebSocket as Protocol, WebSocketProtocols, WsMessage},
};
use axum::{
    extract::ws::{CloseFrame, Message, WebSocket, WebSocketUpgrade},
    http::StatusCode,
    response::{IntoResponse as _, Response},
};
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use tokio::sync::oneshot;
use uuid::Uuid;

use self::intake::Intake;
use super::{ClientAddr, Error, identity::identify_connection, store, tokens};
use crate::clock::Clock;

/// The only WebSocket subprotocol served (API-002).
pub const SUBPROTOCOL: &str = "graphql-transport-ws";
/// The largest message, and frame, a client may send (H-6(b)). A bigger one ends the connection
/// before it is parsed or any token in it is looked up.
pub const MESSAGE_LIMIT: usize = 64 * 1024;
/// How often an open connection looks its token up again.
const RECHECK: Duration = Duration::from_secs(15);
/// How long a client may take to send `connection_init`.
const INIT_TIMEOUT: Duration = Duration::from_secs(10);
/// The longest one write to the client may take.
const WRITE_TIMEOUT: Duration = Duration::from_secs(10);
/// How much the hub buffers for a client before it writes to the socket.
const WRITE_BUFFER: usize = 128 * 1024;
/// The most the hub buffers for a client that does not read (H-6(b)). Without it, the pong
/// queued for every Ping a client sends grows without bound; with it, a full buffer keeps one
/// pending pong, and a message that does not fit ends the connection. It leaves room for the
/// largest answer the hub sends, a full page of 200 log entries.
const WRITE_BUFFER_LIMIT: usize = 2 * 1024 * 1024;

/// What a connection needs to check its sign-in.
#[derive(Clone)]
pub struct SocketAccess {
    db: PgPool,
    clock: Arc<dyn Clock>,
    client: Option<ClientAddr>,
    recheck: Duration,
    init_timeout: Duration,
}

impl SocketAccess {
    /// Checks for a connection from `client`, with the time from `clock`.
    #[must_use]
    pub fn new(db: PgPool, clock: Arc<dyn Clock>, client: Option<ClientAddr>) -> Self {
        Self {
            db,
            clock,
            client,
            recheck: RECHECK,
            init_timeout: INIT_TIMEOUT,
        }
    }

    /// The same checks with another re-check interval and `connection_init` timeout, for tests
    /// of expiry, logout and silence. It exists only with the `test-support` feature, which no
    /// release build turns on.
    #[cfg(feature = "test-support")]
    #[must_use]
    pub fn with_timings(self, recheck: Duration, init_timeout: Duration) -> Self {
        Self {
            recheck,
            init_timeout,
            ..self
        }
    }
}

/// Upgrades the request to a `graphql-transport-ws` connection served by `executor`, or answers
/// 400 when the client does not offer that subprotocol.
pub fn accept_socket<E: Executor>(
    upgrade: WebSocketUpgrade,
    executor: E,
    access: SocketAccess,
) -> Response {
    let upgrade = upgrade
        .protocols([SUBPROTOCOL])
        .max_message_size(MESSAGE_LIMIT)
        .max_frame_size(MESSAGE_LIMIT)
        .write_buffer_size(WRITE_BUFFER)
        .max_write_buffer_size(WRITE_BUFFER_LIMIT);
    if upgrade.selected_protocol().is_none() {
        let refusal = "Use the graphql-transport-ws subprotocol.";
        return (StatusCode::BAD_REQUEST, refusal).into_response();
    }
    upgrade.on_upgrade(move |socket| serve(socket, executor, access))
}

/// The token a connection signed in with, and when it expires.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Lease {
    token: Uuid,
    expires_at: DateTime<Utc>,
}

/// The outcome of `connection_init`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Verdict {
    Leased(Lease),
    Refused,
    Unavailable,
}

/// Why the hub closes a connection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Closing {
    /// The client closed the connection or went away; there is nobody to tell.
    ClientLeft,
    /// A message could not be read, for example because it is bigger than [`MESSAGE_LIMIT`].
    Unreadable,
    /// No `connection_init` in time ([`INIT_TIMEOUT`]).
    InitTimeout,
    /// The token is missing, not valid, expired or deleted.
    Forbidden,
    /// The token could not be checked; the connection fails closed.
    Unavailable,
    /// A write took longer than [`WRITE_TIMEOUT`]: the client does not read, so no close frame
    /// is sent either.
    Stalled,
    /// A `subscribe` reused the id of a running operation.
    DuplicateOperation,
    /// A `subscribe` would run more than 32 operations at once.
    TooManyOperations,
    /// The protocol closed the connection with this code.
    Protocol(u16),
}

impl Closing {
    /// The close code and reason sent to the client. Reasons are fixed texts: the protocol's
    /// own reason may quote the client's message, token included.
    const fn frame(self) -> Option<(u16, &'static str)> {
        match self {
            Self::ClientLeft | Self::Stalled => None,
            Self::Unreadable => Some((1008, "Message refused")),
            Self::DuplicateOperation => Some((4409, "Subscriber already exists")),
            Self::TooManyOperations => Some((1008, "Too many operations")),
            Self::InitTimeout => Some((4408, "Connection initialisation timeout")),
            Self::Forbidden => Some((4403, "Forbidden")),
            Self::Unavailable => Some((1011, "Sign-in cannot be checked")),
            Self::Protocol(1002) => Some((4400, "Invalid message")),
            // async-graphql closes with 1011 when `subscribe` comes before `connection_ack`.
            Self::Protocol(1011) => Some((4401, "Unauthorized")),
            Self::Protocol(4429) => Some((4429, "Too many initialisation requests")),
            Self::Protocol(code) => Some((code, "")),
        }
    }
}

/// Serves one upgraded connection until it ends, then sends the close frame that says why.
async fn serve<E: Executor>(socket: WebSocket, executor: E, access: SocketAccess) {
    let (mut sink, source) = socket.split();
    let intake = Intake::default();
    let input = intake.clone().read(source);
    let (verdict, decided) = oneshot::channel();
    let checks = access.clone();
    let sent = intake.clone();
    let protocol = Protocol::new(executor, input, WebSocketProtocols::GraphQLWS)
        .on_connection_init(move |payload| sign_in(checks, payload, verdict))
        .inspect(move |message| sent.sent(message));
    let closing = intake.reason(converse(&mut sink, protocol, decided, &access).await);
    if let Some((code, reason)) = closing.frame() {
        let frame = CloseFrame {
            code,
            reason: reason.into(),
        };
        // Bounded like every write. It fails only when the client is gone or does not read,
        // and then there is nobody to tell.
        let close = sink.send(Message::Close(Some(frame)));
        let _ = tokio::time::timeout(WRITE_TIMEOUT, close).await;
    }
}

/// The `connection_init` handler: resolves the payload's bearer token, never logging it, and
/// reports the verdict to the connection.
async fn sign_in(
    access: SocketAccess,
    payload: serde_json::Value,
    verdict: oneshot::Sender<Verdict>,
) -> async_graphql::Result<Data> {
    let identified = identify_connection(&access.db, &payload, access.clock.now()).await;
    drop(payload);
    let (decision, data) = match identified {
        Ok(Some(identity)) => {
            let lease = Lease {
                token: identity.token(),
                expires_at: identity.expires_at(),
            };
            let mut data = identity.into_connection_data();
            if let Some(client) = access.client {
                data.insert(client);
            }
            (Verdict::Leased(lease), Ok(data))
        }
        Ok(None) => (Verdict::Refused, Err("Forbidden".into())),
        Err(error) => {
            error.log();
            (Verdict::Unavailable, Err("Unavailable".into()))
        }
    };
    // Fails only when the connection is gone, and then nobody waits for the verdict.
    let _ = verdict.send(decision);
    data
}

/// Relays the protocol's messages to the client until the connection ends; returns why.
async fn converse<S: Sink<Message> + Unpin>(
    sink: &mut S,
    protocol: impl Stream<Item = WsMessage>,
    decided: oneshot::Receiver<Verdict>,
    access: &SocketAccess,
) -> Closing {
    let mut protocol = pin!(protocol);
    let lease = match signed_in(sink, &mut protocol, decided, access.init_timeout).await {
        Ok(lease) => lease,
        Err(closing) => return closing,
    };
    let mut lapse = pin!(lapse(access, lease));
    loop {
        let message = tokio::select! {
            biased;
            closing = &mut lapse => return closing,
            message = protocol.next() => message,
        };
        if let Err(closing) = relay(sink, message, None, lapse.as_mut()).await {
            return closing;
        }
    }
}

/// Relays messages until `connection_init` is decided; the lease, or why the connection ends.
async fn signed_in<S: Sink<Message> + Unpin>(
    sink: &mut S,
    protocol: &mut Pin<&mut impl Stream<Item = WsMessage>>,
    mut decided: oneshot::Receiver<Verdict>,
    init_timeout: Duration,
) -> Result<Lease, Closing> {
    let mut deadline = pin!(async move {
        tokio::time::sleep(init_timeout).await;
        Closing::InitTimeout
    });
    loop {
        let message = tokio::select! {
            biased;
            verdict = &mut decided => return lease_of(verdict.ok()),
            closing = &mut deadline => return Err(closing),
            message = protocol.next() => message,
        };
        relay(sink, message, Some(&mut decided), deadline.as_mut()).await?;
    }
}

/// The lease a verdict grants; a missing verdict refuses (fail closed).
fn lease_of(verdict: Option<Verdict>) -> Result<Lease, Closing> {
    match verdict {
        Some(Verdict::Leased(lease)) => Ok(lease),
        Some(Verdict::Unavailable) => Err(Closing::Unavailable),
        Some(Verdict::Refused) | None => Err(Closing::Forbidden),
    }
}

/// Sends one protocol message to the client unless `stop` ends the connection first; `Err` with
/// the reason when the connection ends. A close that follows a refused `connection_init`
/// becomes the refusal.
async fn relay<S: Sink<Message> + Unpin>(
    sink: &mut S,
    message: Option<WsMessage>,
    decided: Option<&mut oneshot::Receiver<Verdict>>,
    stop: Pin<&mut impl Future<Output = Closing>>,
) -> Result<(), Closing> {
    match message {
        None => Err(Closing::ClientLeft),
        Some(WsMessage::Close(code, _)) => {
            Err(match decided.and_then(|decided| decided.try_recv().ok()) {
                Some(Verdict::Refused) => Closing::Forbidden,
                Some(Verdict::Unavailable) => Closing::Unavailable,
                Some(Verdict::Leased(_)) | None => Closing::Protocol(code),
            })
        }
        Some(WsMessage::Text(text)) => deliver(sink, Message::Text(text.into()), stop).await,
    }
}

/// Writes `message` within [`WRITE_TIMEOUT`], unless `stop` ends the connection first.
async fn deliver<S: Sink<Message> + Unpin>(
    sink: &mut S,
    message: Message,
    stop: Pin<&mut impl Future<Output = Closing>>,
) -> Result<(), Closing> {
    tokio::select! {
        biased;
        closing = stop => Err(closing),
        written = tokio::time::timeout(WRITE_TIMEOUT, sink.send(message)) => match written {
            Ok(Ok(())) => Ok(()),
            Ok(Err(_)) => Err(Closing::ClientLeft),
            Err(_) => Err(Closing::Stalled),
        },
    }
}

/// Resolves when the lease ends: at its expiry by the clock, when its token is gone, or when
/// the token cannot be checked.
async fn lapse(access: &SocketAccess, lease: Lease) -> Closing {
    loop {
        let Some(wait) = next_check(lease.expires_at, access.clock.now(), access.recheck) else {
            return Closing::Forbidden;
        };
        tokio::time::sleep(wait).await;
        if !tokens::usable(lease.expires_at, access.clock.now()) {
            return Closing::Forbidden;
        }
        match store::token_exists(&access.db, lease.token).await {
            Ok(true) => {}
            Ok(false) => return Closing::Forbidden,
            Err(error) => {
                Error::from(error).log();
                return Closing::Unavailable;
            }
        }
    }
}

/// How long until a lease that expires at `expires_at` is checked again: `recheck`, or less
/// when it expires sooner; `None` once it has expired at `now`.
fn next_check(
    expires_at: DateTime<Utc>,
    now: DateTime<Utc>,
    recheck: Duration,
) -> Option<Duration> {
    if !tokens::usable(expires_at, now) {
        return None;
    }
    let left = (expires_at - now).to_std().unwrap_or(Duration::ZERO);
    Some(left.min(recheck))
}

#[cfg(test)]
mod tests;
