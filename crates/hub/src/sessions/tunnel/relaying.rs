//! The tunnel task (K4 Sections 4 and 5): one task per session that waits for the upgraded
//! socket, relays instructions both ways under the pure rules in `relay`, `client` and
//! `gateway`, and ends the session once.
//!
//! One `select!` over the end signal, the client socket, the guacd socket and two timers; each
//! branch is cancel safe (`recv`, `read`, `ended`, `sleep_until`), writes happen in the branch
//! body with a time limit, and no lock is held. Memory is bounded: one read buffer, the
//! decoders' one partial instruction each, and one 64 KiB client message at a time.

use std::{io, sync::Arc};

use axum::extract::ws::{CloseFrame, Message, Utf8Bytes, WebSocket};
use futures_util::SinkExt;
use guac_protocol::{Decoder, Instruction, encode};
use tokio::{
    io::{AsyncReadExt as _, AsyncWriteExt as _},
    net::TcpStream,
    sync::oneshot,
    time::{Instant, sleep_until, timeout},
};
use uuid::Uuid;
use zeroize::Zeroizing;

use super::{
    ending::{Cause, End},
    gateway::{host_key_changed, tunnel_opened},
    guacd::{Connected, ConnectionId, READ_BYTES},
    opening::Opening,
    record::finish,
    relay::{ClientStep, GatewayStep, Relay},
    route::{Route, Timing},
};
use crate::sessions::{EndCause, Redeemed, Tunnel};

/// What the hub tells guacd when it leaves first.
const DISCONNECT: &[u8] = b"10.disconnect;";

/// Runs the session's tunnel to its end, then records the end and drops the registration.
pub(super) async fn run(
    route: Arc<Route>,
    mut redeemed: Redeemed,
    opening: Opening,
    socket: oneshot::Receiver<WebSocket>,
) {
    let timing = route.timing;
    let end = match upgraded(&mut redeemed.tunnel, socket, timing).await {
        Err(end) => {
            if let Opening::Gateway(connected) = opening {
                leave_gateway(connected.stream, timing).await;
            }
            end
        }
        Ok(client) => match opening {
            Opening::HostKeyChanged => host_key_mismatch(client, timing).await,
            Opening::Gateway(connected) => {
                relay(client, connected, &mut redeemed.tunnel, timing).await
            }
        },
    };
    finish(&route, redeemed, &end).await;
}

/// Tells the client the host key changed and closes 4001. guacd was never reached (K4
/// Section 6); the mismatch is the cause even if the client is gone before it hears of it.
async fn host_key_mismatch(mut client: WebSocket, timing: Timing) -> End {
    let _ = send_text(&mut client, &host_key_changed(), timing).await;
    close_client(client, Cause::HostKeyMismatch, timing).await;
    Cause::HostKeyMismatch.into()
}

/// Relays between the client and guacd until the first cause, then closes both.
async fn relay(
    client: WebSocket,
    connected: Connected,
    tunnel: &mut Tunnel,
    timing: Timing,
) -> End {
    let now = Instant::now();
    let mut link = Link {
        client,
        gateway: connected.stream,
        from_client: Decoder::new(),
        from_gateway: connected.decoder,
        relay: Relay::default(),
        timing,
        buffer: vec![0; READ_BYTES],
        gateway_quiet: now + timing.gateway_silence,
        client_quiet: now + timing.client_idle,
        told_gateway: false,
        _connection_id: connected.connection_id,
    };
    let end = link.run(connected.after_ready, tunnel).await;
    link.close(end.cause).await;
    end
}

/// The upgraded socket, unless the session ends first: the end signal, a failed upgrade (the
/// client left) or no upgrade within the handshake wait.
async fn upgraded(
    tunnel: &mut Tunnel,
    socket: oneshot::Receiver<WebSocket>,
    timing: Timing,
) -> Result<WebSocket, End> {
    tokio::select! {
        signal = tunnel.ended() => Err(End::from(signal)),
        socket = timeout(timing.handshake, socket) => match socket {
            Ok(Ok(socket)) => Ok(socket),
            Ok(Err(_)) => Err(Cause::OperatorClosed.into()),
            Err(_) => Err(Cause::Timeout.into()),
        },
    }
}

/// One step of the loop: whichever came first.
enum Turn {
    Signalled(EndCause),
    Client(Option<Result<Message, axum::Error>>),
    Gateway(io::Result<usize>),
    GatewaySilent,
    ClientIdle,
}

/// The two sockets of one open tunnel and the relay state between them.
struct Link {
    client: WebSocket,
    gateway: TcpStream,
    from_client: Decoder,
    from_gateway: Decoder,
    relay: Relay,
    timing: Timing,
    /// The one buffer guacd is read into.
    buffer: Vec<u8>,
    /// When guacd's silence earns the client a `nop`.
    gateway_quiet: Instant,
    /// When the client's silence ends the session.
    client_quiet: Instant,
    /// The client's own `disconnect` already went to guacd.
    told_gateway: bool,
    /// Kept for the tunnel's life only (GW-014).
    _connection_id: ConnectionId,
}

impl Link {
    /// Relays until the first cause to end. The client gets the open instruction first, with
    /// a new UUID that is not guacd's connection ID (K4 Section 4, GW-014), then whatever
    /// guacd sent with `ready` (WP-08 H-5).
    async fn run(&mut self, after_ready: Vec<Instruction>, tunnel: &mut Tunnel) -> End {
        let opened = tunnel_opened(&Uuid::now_v7().to_string());
        if let Err(end) = send_text(&mut self.client, &opened, self.timing).await {
            return end;
        }
        if let Some(end) = self.relay_gateway(after_ready).await {
            return end;
        }
        self.gateway_quiet = Instant::now() + self.timing.gateway_silence;
        self.client_quiet = Instant::now() + self.timing.client_idle;
        loop {
            let turn = self.next_turn(tunnel).await;
            if let Some(end) = self.on_turn(turn).await {
                return end;
            }
        }
    }

    /// Waits for whichever comes first. Every branch is cancel safe.
    async fn next_turn(&mut self, tunnel: &mut Tunnel) -> Turn {
        tokio::select! {
            signal = tunnel.ended() => Turn::Signalled(signal),
            message = self.client.recv() => Turn::Client(message),
            read = self.gateway.read(&mut self.buffer) => Turn::Gateway(read),
            () = sleep_until(self.gateway_quiet) => Turn::GatewaySilent,
            () = sleep_until(self.client_quiet) => Turn::ClientIdle,
        }
    }

    /// Acts on one turn; `Some` when the session ends. Any client message restarts the idle
    /// wait; any guacd read restarts the silence wait, and so does the `nop` it causes.
    async fn on_turn(&mut self, turn: Turn) -> Option<End> {
        match turn {
            Turn::Signalled(signal) => Some(End::from(signal)),
            Turn::Client(message) => {
                self.client_quiet = Instant::now() + self.timing.client_idle;
                self.on_client(message).await
            }
            Turn::Gateway(read) => {
                self.gateway_quiet = Instant::now() + self.timing.gateway_silence;
                self.on_gateway(read).await
            }
            Turn::GatewaySilent => {
                self.gateway_quiet = Instant::now() + self.timing.gateway_silence;
                let nop = Instruction::new("nop", Vec::<String>::new());
                send_text(&mut self.client, &nop, self.timing).await.err()
            }
            Turn::ClientIdle => Some(Cause::Timeout.into()),
        }
    }

    /// One WebSocket message from the client. Text carries instructions; anything else that
    /// is not a ping, pong or close breaks the protocol (GW-011). A closed or broken socket
    /// is the client leaving.
    async fn on_client(&mut self, message: Option<Result<Message, axum::Error>>) -> Option<End> {
        let text = match message {
            Some(Ok(Message::Text(text))) => text,
            Some(Ok(Message::Ping(_) | Message::Pong(_))) => return None,
            Some(Ok(Message::Binary(_))) => return Some(Cause::InvalidInstruction.into()),
            Some(Ok(Message::Close(_)) | Err(_)) | None => {
                return Some(Cause::OperatorClosed.into());
            }
        };
        let Ok(instructions) = self.from_client.feed(text.as_bytes()) else {
            return Some(Cause::InvalidInstruction.into());
        };
        for instruction in instructions {
            let ended = match self.relay.on_client(instruction) {
                ClientStep::Forward(instruction) => self.forward(&instruction).await.err(),
                ClientStep::Reply(instruction) => {
                    send_text(&mut self.client, &instruction, self.timing)
                        .await
                        .err()
                }
                ClientStep::Drop => None,
                ClientStep::End { forward, cause } => {
                    if let Some(instruction) = forward {
                        self.told_gateway = self.forward(&instruction).await.is_ok();
                    }
                    Some(cause.into())
                }
            };
            if ended.is_some() {
                return ended;
            }
        }
        None
    }

    /// One read from guacd: EOF is the remote end closing, a read error or bytes that do not
    /// decode are a gateway error.
    async fn on_gateway(&mut self, read: io::Result<usize>) -> Option<End> {
        let count = match read {
            Ok(0) => return Some(Cause::RemoteClosed.into()),
            Ok(count) => count,
            Err(_) => return Some(Cause::GatewayError.into()),
        };
        match self.from_gateway.feed(&self.buffer[..count]) {
            Ok(instructions) => self.relay_gateway(instructions).await,
            Err(_) => Some(Cause::GatewayError.into()),
        }
    }

    /// guacd's instructions to the client, each as one text message, under the gateway rule.
    async fn relay_gateway(&mut self, instructions: Vec<Instruction>) -> Option<End> {
        for instruction in instructions {
            match self.relay.on_gateway(instruction) {
                GatewayStep::Forward(instruction) => {
                    if let Err(end) = send_text(&mut self.client, &instruction, self.timing).await {
                        return Some(end);
                    }
                }
                GatewayStep::Drop => {}
                GatewayStep::End { send, cause } => {
                    // The cause stands even if the client is gone by now: it came first.
                    let _ = send_text(&mut self.client, &send, self.timing).await;
                    return Some(cause.into());
                }
            }
        }
        None
    }

    /// Writes one client instruction to guacd. A write that fails or stalls past the idle
    /// wait is a gateway error. The wire text may hold what the operator typed: wiped.
    async fn forward(&mut self, instruction: &Instruction) -> Result<(), End> {
        let wire = Zeroizing::new(encode(instruction));
        match timeout(
            self.timing.client_idle,
            self.gateway.write_all(wire.as_bytes()),
        )
        .await
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(_)) | Err(_) => Err(Cause::GatewayError.into()),
        }
    }

    /// Closes both sockets: guacd is told to disconnect unless the client already did, and
    /// the client gets the end table's close code.
    async fn close(self, cause: Cause) {
        let Self {
            client,
            gateway,
            timing,
            told_gateway,
            ..
        } = self;
        if told_gateway {
            drop(gateway);
        } else {
            leave_gateway(gateway, timing).await;
        }
        close_client(client, cause, timing).await;
    }
}

/// Sends one instruction to the client as one text message. A failed send is the client
/// leaving; one that stalls past the idle wait is a timeout.
async fn send_text(
    client: &mut WebSocket,
    instruction: &Instruction,
    timing: Timing,
) -> Result<(), End> {
    let message = Message::Text(Utf8Bytes::from(encode(instruction)));
    match timeout(timing.client_idle, client.send(message)).await {
        Ok(Ok(())) => Ok(()),
        Ok(Err(_)) => Err(Cause::OperatorClosed.into()),
        Err(_) => Err(Cause::Timeout.into()),
    }
}

/// Sends the close frame with the cause's code (K4 Section 5), then flushes and closes the
/// socket. When the client closed first, the WebSocket library refuses a second close frame
/// and its echo of the client's frame is what goes out (RFC 6455 Section 5.5.1); closing the
/// sink flushes it. Best effort: the client may be gone, and the session ends either way.
async fn close_client(mut client: WebSocket, cause: Cause, timing: Timing) {
    let frame = CloseFrame {
        code: cause.ending().close_code,
        reason: Utf8Bytes::default(),
    };
    let _ = timeout(timing.client_idle, async {
        let _ = client.send(Message::Close(Some(frame))).await;
        SinkExt::close(&mut client).await
    })
    .await;
}

/// Tells guacd the hub is leaving, then closes the connection. Best effort: guacd may be
/// gone, and dropping the socket ends its side either way.
async fn leave_gateway(mut gateway: TcpStream, timing: Timing) {
    let _ = timeout(timing.client_idle, async {
        gateway.write_all(DISCONNECT).await?;
        gateway.shutdown().await
    })
    .await;
}
