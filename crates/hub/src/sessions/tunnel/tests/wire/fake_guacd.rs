//! An in-process guacd: a TCP listener that speaks the guacd 1.6.0 RDP or VNC handshake from
//! the recorded `args` fixtures, reports what the hub sent, then relays on the test's command.

use std::{collections::HashMap, future};

use guac_protocol::{Decoder, Instruction, encode};
use tokio::{
    io::{AsyncReadExt as _, AsyncWriteExt as _},
    net::{TcpListener, TcpStream},
    sync::{mpsc, oneshot},
};

/// guacd 1.6.0's `args` for `rdp` and `vnc`, recorded from `guacamole/guacd:1.6.0`.
pub(super) const RDP_ARGS: &[u8] = include_bytes!("../fixtures/guacd-1.6.0-rdp-args.guac");
pub(super) const VNC_ARGS: &[u8] = include_bytes!("../fixtures/guacd-1.6.0-vnc-args.guac");

/// The connection ID the fake guacd gives in `ready`: it must never leave the hub (GW-014).
pub(super) const CONNECTION_ID: &str = "$5e0c7a1d-connection-id-never-sent";

/// How the fake guacd behaves.
#[derive(Clone, Copy)]
pub(super) enum Script {
    /// Answers `select` with `args`, reads the hub's reply, sends `ready` and `with_ready` in
    /// one write (WP-08 H-5), then relays.
    Serve {
        args: &'static [u8],
        with_ready: &'static str,
    },
    /// Closes the connection after reading `select`.
    DropAfterSelect,
    /// After `select`, sends nothing until the test's first act; then sends `args` and
    /// reports as the handshake what the hub sends up to `connect` or until it closes.
    StallBeforeArgs { args: &'static [u8] },
}

/// What the test tells the fake guacd after `ready`.
pub(super) enum Act {
    Send(String),
    /// Closes the connection with a FIN.
    Close,
    /// Resets the connection (RST), so the hub's next read fails.
    Reset,
    /// Stops reading and keeps the connection open, so the hub's writes stall.
    Stall,
}

/// A running fake guacd: its address, the protocol `select` named, the handshake the hub sent,
/// the instructions the hub relayed after `ready`, and the way to make it act.
pub(super) struct FakeGuacd {
    pub(super) address: String,
    pub(super) selected: oneshot::Receiver<String>,
    pub(super) handshake: oneshot::Receiver<Vec<Instruction>>,
    pub(super) from_hub: mpsc::UnboundedReceiver<Instruction>,
    pub(super) act: mpsc::UnboundedSender<Act>,
}

/// Starts a fake guacd for one connection.
pub(super) async fn fake_guacd(script: Script) -> FakeGuacd {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap().to_string();
    let (selected_tx, selected) = oneshot::channel();
    let (handshake_tx, handshake) = oneshot::channel();
    let (from_hub_tx, from_hub) = mpsc::unbounded_channel();
    let (act, mut acts) = mpsc::unbounded_channel();
    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut peer = Peer {
            stream,
            decoder: Decoder::new(),
            queue: Vec::new(),
        };
        let mut select = peer.next().await.expect("select");
        assert_eq!(select.opcode, "select");
        assert_eq!(select.args.len(), 1, "one protocol");
        let _ = selected_tx.send(select.args.remove(0));
        match script {
            Script::Serve { args, with_ready } => {
                peer.stream.write_all(args).await.unwrap();
                let _ = handshake_tx.send(peer.until_connect().await);
                let ready = encode(&Instruction::new("ready", [CONNECTION_ID]));
                let first = format!("{ready}{with_ready}");
                peer.stream.write_all(first.as_bytes()).await.unwrap();
                peer.relay(acts, from_hub_tx).await;
            }
            Script::DropAfterSelect => {}
            Script::StallBeforeArgs { args } => {
                let _ = acts.recv().await;
                // The hub may have closed already; then nothing more comes.
                let _ = peer.stream.write_all(args).await;
                let _ = handshake_tx.send(peer.until_connect().await);
            }
        }
    });
    FakeGuacd {
        address,
        selected,
        handshake,
        from_hub,
        act,
    }
}

struct Peer {
    stream: TcpStream,
    decoder: Decoder,
    queue: Vec<Instruction>,
}

impl Peer {
    /// The next instruction from the hub, or `None` at EOF or a broken connection.
    async fn next(&mut self) -> Option<Instruction> {
        let mut buffer = [0; 4096];
        while self.queue.is_empty() {
            let read = self.stream.read(&mut buffer).await.ok()?;
            if read == 0 {
                return None;
            }
            self.queue = self.decoder.feed(&buffer[..read]).unwrap();
        }
        Some(self.queue.remove(0))
    }

    /// Every instruction up to and including `connect`, or until the hub closes.
    async fn until_connect(&mut self) -> Vec<Instruction> {
        let mut sent = Vec::new();
        while let Some(instruction) = self.next().await {
            let done = instruction.opcode == "connect";
            sent.push(instruction);
            if done {
                break;
            }
        }
        sent
    }

    /// Passes the hub's instructions to the test and acts as the test asks, until the hub
    /// leaves or the test closes.
    async fn relay(
        mut self,
        mut acts: mpsc::UnboundedReceiver<Act>,
        from_hub: mpsc::UnboundedSender<Instruction>,
    ) {
        for queued in std::mem::take(&mut self.queue) {
            let _ = from_hub.send(queued);
        }
        loop {
            let stalled = tokio::select! {
                act = acts.recv() => match act {
                    Some(Act::Send(text)) => {
                        self.stream.write_all(text.as_bytes()).await.unwrap();
                        false
                    }
                    Some(Act::Reset) => {
                        self.stream.set_zero_linger().unwrap();
                        return;
                    }
                    Some(Act::Stall) => true,
                    Some(Act::Close) | None => return,
                },
                instruction = self.next() => match instruction {
                    Some(instruction) => { let _ = from_hub.send(instruction); false }
                    None => return,
                },
            };
            if stalled {
                future::pending::<()>().await;
            }
        }
    }
}

/// The `connect` value of each `args` name.
pub(super) fn connect_values(args: &[u8], connect: &Instruction) -> HashMap<String, String> {
    let names = Decoder::new().feed(args).unwrap().remove(0).args;
    assert_eq!(connect.opcode, "connect");
    assert_eq!(connect.args[0], "VERSION_1_5_0");
    assert_eq!(connect.args.len(), names.len(), "one value per name");
    names[1..]
        .iter()
        .cloned()
        .zip(connect.args[1..].iter().cloned())
        .collect()
}
