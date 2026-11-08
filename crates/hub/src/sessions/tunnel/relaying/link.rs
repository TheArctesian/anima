//! The open tunnel (K4 Sections 4 and 5): relays instructions both ways under the pure rules
//! in `relay`, `client` and `gateway` until the first cause, then closes both sockets.
//!
//! One `select!` over the end signal, the client socket, the guacd socket and two timers; each
//! branch is cancel safe (`recv`, `read`, `ended`, `sleep_until`), writes happen in the branch
//! body under `writes`' limits, and no lock is held. Memory is bounded: one read buffer, the
//! decoders' one partial instruction each, and one 64 KiB client message at a time.

use std::io;

use axum::extract::ws::{Message, WebSocket};
use guac_protocol::{Decoder, Instruction, encode};
use tokio::{
    io::AsyncReadExt as _,
    net::TcpStream,
    time::{Instant, sleep_until},
};
use uuid::Uuid;
use zeroize::Zeroizing;

use super::{
    super::{
        ending::{Cause, End, GATEWAY_LOST},
        gateway::tunnel_opened,
        guacd::{ConnectionId, READ_BYTES},
        opening::Opened,
        relay::{ClientStep, GatewayStep, Relay},
        route::Timing,
    },
    writes::{leave_gateway, send_text, write_gateway},
};
use crate::sessions::{
    EndCause, Tunnel,
    end::unreadable,
    upgrade::{Quiet, close},
};

/// Relays between the client and guacd under the opened session's policy until the first
/// cause, then closes both.
pub(super) async fn relay(
    client: WebSocket,
    opened: Opened,
    tunnel: &mut Tunnel,
    timing: Timing,
) -> End {
    let Opened { connected, policy } = opened;
    let now = Instant::now();
    let mut link = Link {
        client,
        gateway: connected.stream,
        tunnel,
        from_client: Decoder::new(),
        from_gateway: connected.decoder,
        relay: Relay::new(policy),
        timing,
        buffer: vec![0; READ_BYTES],
        gateway_quiet: Quiet::new(timing.gateway_silence, now),
        client_quiet: Quiet::new(timing.client_idle, now),
        told_gateway: false,
        _connection_id: connected.connection_id,
    };
    let end = link.run(connected.after_ready).await;
    link.close(end.cause).await;
    end
}

/// One step of the loop: whichever came first.
enum Turn {
    Signalled(EndCause),
    Client(Option<Result<Message, axum::Error>>),
    Gateway(io::Result<usize>),
    GatewaySilent,
    ClientIdle,
}

/// The two sockets of one open tunnel, its end signal and the relay state between them.
struct Link<'a> {
    client: WebSocket,
    gateway: TcpStream,
    tunnel: &'a mut Tunnel,
    from_client: Decoder,
    from_gateway: Decoder,
    relay: Relay,
    timing: Timing,
    /// The one buffer guacd is read into.
    buffer: Vec<u8>,
    /// When guacd's silence earns the client a `nop`.
    gateway_quiet: Quiet,
    /// When the client's silence ends the session.
    client_quiet: Quiet,
    /// The client's own `disconnect` already went to guacd.
    told_gateway: bool,
    /// Kept for the tunnel's life only (GW-014).
    _connection_id: ConnectionId,
}

impl Link<'_> {
    /// Relays until the first cause to end. The client gets the open instruction first, with
    /// a new UUID that is not guacd's connection ID (K4 Section 4, GW-014), then whatever
    /// guacd sent with `ready` (WP-08 H-5).
    async fn run(&mut self, after_ready: Vec<Instruction>) -> End {
        let opened = tunnel_opened(&Uuid::now_v7().to_string());
        if let Err(end) = self.tell_client(&opened).await {
            return end;
        }
        if let Some(end) = self.relay_gateway(after_ready).await {
            return end;
        }
        loop {
            let turn = self.next_turn().await;
            if let Some(end) = self.on_turn(turn).await {
                return end;
            }
        }
    }

    /// Waits for whichever comes first. Every branch is cancel safe.
    async fn next_turn(&mut self) -> Turn {
        tokio::select! {
            signal = self.tunnel.ended() => Turn::Signalled(signal),
            message = self.client.recv() => Turn::Client(message),
            read = self.gateway.read(&mut self.buffer) => Turn::Gateway(read),
            () = sleep_until(self.gateway_quiet.due()) => Turn::GatewaySilent,
            () = sleep_until(self.client_quiet.due()) => Turn::ClientIdle,
        }
    }

    /// Acts on one turn; `Some` when the session ends. Any client message restarts the idle
    /// wait, a WebSocket ping or pong included (the browser's keep-alive counts as activity);
    /// any guacd read restarts the silence wait, and so does the `nop` it causes. Each
    /// wait restarts once the turn is handled, so the time the hub spends writing to the
    /// other side, held back by it, never counts as this side's silence.
    async fn on_turn(&mut self, turn: Turn) -> Option<End> {
        match turn {
            Turn::Signalled(signal) => Some(End::from(signal)),
            Turn::Client(message) => {
                let ended = self.on_client(message).await;
                self.client_quiet.restart(Instant::now());
                ended
            }
            Turn::Gateway(read) => {
                let ended = self.on_gateway(read).await;
                self.gateway_quiet.restart(Instant::now());
                ended
            }
            Turn::GatewaySilent => {
                self.gateway_quiet.restart(Instant::now());
                let nop = Instruction::new("nop", Vec::<String>::new());
                self.tell_client(&nop).await.err()
            }
            Turn::ClientIdle => Some(Cause::Timeout.into()),
        }
    }

    /// One WebSocket message from the client. Text carries instructions; anything else that
    /// is not a ping, pong or close breaks the protocol (GW-011).
    async fn on_client(&mut self, message: Option<Result<Message, axum::Error>>) -> Option<End> {
        let text = match message {
            Some(Ok(Message::Text(text))) => text,
            Some(Ok(Message::Ping(_) | Message::Pong(_))) => return None,
            Some(Ok(Message::Binary(_))) => return Some(Cause::InvalidInstruction.into()),
            Some(Ok(Message::Close(_))) | None => return Some(Cause::OperatorClosed.into()),
            Some(Err(error)) => return Some(unreadable(error).into()),
        };
        let Ok(instructions) = self.from_client.feed(text.as_bytes()) else {
            return Some(Cause::InvalidInstruction.into());
        };
        for instruction in instructions {
            let ended = match self.relay.on_client(instruction) {
                ClientStep::Forward(instruction) => self.forward(&instruction).await.err(),
                ClientStep::Reply(instruction) => self.tell_client(&instruction).await.err(),
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
            Err(_) => return Some(End::explained(Cause::GatewayError, GATEWAY_LOST)),
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
                    if let Err(end) = self.tell_client(&instruction).await {
                        return Some(end);
                    }
                }
                GatewayStep::Drop => {}
                GatewayStep::Refuse(ack) => {
                    if let Err(end) = self.forward(&ack).await {
                        return Some(end);
                    }
                }
                GatewayStep::End { send, cause } => {
                    // The cause stands even if the client is gone by now: it came first.
                    let _ = self.tell_client(&send).await;
                    return Some(cause.into());
                }
            }
        }
        None
    }

    /// Sends one instruction to the client (`writes::send_text`).
    async fn tell_client(&mut self, instruction: &Instruction) -> Result<(), End> {
        let wait = self.timing.client_idle;
        send_text(&mut self.client, instruction, self.tunnel, wait).await
    }

    /// Writes one instruction to guacd (`writes::write_gateway`). The encoded copy may
    /// hold what the operator typed, so it is zeroed once written; the decoded instruction
    /// is dropped by the caller without zeroing.
    async fn forward(&mut self, instruction: &Instruction) -> Result<(), End> {
        let wire = Zeroizing::new(encode(instruction));
        let wait = self.timing.client_idle;
        write_gateway(&mut self.gateway, wire.as_bytes(), self.tunnel, wait).await
    }

    /// Closes both sockets side by side: guacd is told to disconnect unless the client
    /// already did, and the client gets the end table's close code.
    async fn close(self, cause: Cause) {
        let Self {
            client,
            gateway,
            told_gateway,
            ..
        } = self;
        let gateway = async move {
            if !told_gateway {
                leave_gateway(gateway).await;
            }
        };
        tokio::join!(gateway, close(client, None, cause));
    }
}
