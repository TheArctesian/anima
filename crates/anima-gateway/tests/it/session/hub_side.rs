//! The hub's side of a session for the tests: the hub's own handshake, then instructions
//! read and sent as the hub's relay does.

use std::time::Duration;

use anima_gateway::session::{Ended, Settings, serve};
use guac_protocol::{
    ArgValues, ConnectParams, Decoder, Display, Handshake, HandshakeStep, Instruction, encode,
};
use tokio::io::{
    AsyncRead, AsyncReadExt as _, AsyncWrite, AsyncWriteExt as _, DuplexStream, duplex,
};
use tokio::task::JoinHandle;
use tokio::time::timeout;

const WAIT: Duration = Duration::from_secs(10);

pub(crate) struct HubSide<S> {
    stream: S,
    decoder: Decoder,
    waiting: std::collections::VecDeque<Instruction>,
}

impl<S: AsyncRead + AsyncWrite + Unpin> HubSide<S> {
    pub(crate) fn on(stream: S) -> Self {
        Self {
            stream,
            decoder: Decoder::new(),
            waiting: std::collections::VecDeque::new(),
        }
    }

    pub(crate) async fn send(&mut self, instruction: &Instruction) {
        self.stream
            .write_all(encode(instruction).as_bytes())
            .await
            .expect("sent to the gateway");
    }

    /// Sends `instruction`; `false` when the gateway has closed the connection.
    pub(crate) async fn try_send(&mut self, instruction: &Instruction) -> bool {
        self.stream
            .write_all(encode(instruction).as_bytes())
            .await
            .is_ok()
    }

    /// The gateway's next instruction; `None` when it closed the connection.
    pub(crate) async fn next(&mut self) -> Option<Instruction> {
        let mut buffer = [0; 8192];
        while self.waiting.is_empty() {
            let count = timeout(WAIT, self.stream.read(&mut buffer))
                .await
                .expect("the gateway answers in time")
                .ok()?;
            if count == 0 {
                return None;
            }
            self.waiting.extend(
                self.decoder
                    .feed(&buffer[..count])
                    .expect("the hub decodes it"),
            );
        }
        self.waiting.pop_front()
    }

    /// Whether the gateway sends nothing for `time`.
    pub(crate) async fn is_quiet_for(&mut self, time: Duration) -> bool {
        if !self.waiting.is_empty() {
            return false;
        }
        let mut buffer = [0; 8192];
        match timeout(time, self.stream.read(&mut buffer)).await {
            Err(_) => true,
            Ok(read) => {
                let count = read.unwrap_or(0);
                self.waiting.extend(
                    self.decoder
                        .feed(&buffer[..count])
                        .expect("the hub decodes it"),
                );
                false
            }
        }
    }

    /// Reads until an instruction with `opcode` comes, and gives it.
    pub(crate) async fn next_of(&mut self, opcode: &str) -> Instruction {
        loop {
            let instruction = self
                .next()
                .await
                .unwrap_or_else(|| panic!("the gateway closed before `{opcode}`"));
            if instruction.opcode == opcode {
                return instruction;
            }
        }
    }

    /// The hub's handshake for `protocol` with `values`, to `ready`.
    pub(crate) async fn open(
        &mut self,
        protocol: &str,
        values: ArgValues,
    ) -> Result<String, Instruction> {
        let params = ConnectParams {
            display: Display {
                width: 800,
                height: 600,
                dpi: 96,
            },
            timezone: "UTC".to_owned(),
            name: "operator".to_owned(),
            audio: &[],
            values,
        };
        let (mut handshake, select) = Handshake::new(protocol, params);
        self.send(&select).await;
        loop {
            let instruction = self
                .next()
                .await
                .expect("the gateway answers the handshake");
            if instruction.opcode == "error" {
                return Err(instruction);
            }
            match handshake
                .on_instruction(instruction)
                .expect("a handshake the hub accepts")
            {
                HandshakeStep::Send(instructions) => {
                    for instruction in &instructions {
                        self.send(instruction).await;
                    }
                }
                HandshakeStep::Ready { connection_id } => return Ok(connection_id),
            }
        }
    }
}

/// A session served in this process, and the hub's side of it.
pub(crate) fn session(settings: Settings) -> (HubSide<DuplexStream>, JoinHandle<Ended>) {
    let (hub, gateway) = duplex(1 << 20);
    let served = tokio::spawn(async move { serve(gateway, "$test", &settings).await });
    (HubSide::on(hub), served)
}

/// The `connect` values the hub sends for a device at `address`.
pub(crate) fn values(address: std::net::SocketAddr, extra: &[(&str, &str)]) -> ArgValues {
    let mut values = ArgValues::new();
    values.set("hostname", address.ip().to_string());
    values.set("port", address.port().to_string());
    for (name, value) in extra {
        values.set(*name, *value);
    }
    values
}
