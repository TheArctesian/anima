//! The hub's connection to guacd up to `ready` (K4 Section 3, GW-010): connect, then run the
//! `guac-protocol` handshake over the socket.
//!
//! The handshake's `connect` holds the credential: it is written once, then wiped, and never
//! logged (WP-08 H-9, LOG-007). guacd's error text stays here (D-11).

use guac_protocol::{
    ConnectParams, Decoder, Handshake, HandshakeError, HandshakeStep, Instruction,
    MAX_INSTRUCTION_BYTES, encode,
};
use tokio::{
    io::{AsyncReadExt as _, AsyncWriteExt as _},
    net::TcpStream,
    time::timeout,
};
use zeroize::{Zeroize, Zeroizing};

use super::{opening::NotOpened, route::Timing};

/// Bytes read from guacd at a time: one of its largest instructions.
pub(super) const READ_BYTES: usize = MAX_INSTRUCTION_BYTES;

/// guacd's ID for the connection, from `ready`. Anyone who reaches guacd with it can join
/// the session, so it is kept with the live tunnel only, never sent, logged or stored
/// (GW-014). No `Debug`, `Display` or `Clone`.
#[expect(
    dead_code,
    reason = "GW-014: held for the tunnel's life; Phase 1 has no shared sessions to read it"
)]
pub(super) struct ConnectionId(String);

/// guacd after `ready`: the socket, its decoder, the instructions that arrived in the same
/// read as `ready` (WP-08 H-5) and the connection ID.
pub(super) struct Connected {
    pub(super) stream: TcpStream,
    pub(super) decoder: Decoder,
    pub(super) after_ready: Vec<Instruction>,
    pub(super) connection_id: ConnectionId,
}

/// Connects to guacd at `address` within `timing.connect`, then completes the handshake that
/// selects `protocol` (a hub constant: `rdp` or `vnc`) for `params` within
/// `timing.handshake`.
///
/// # Errors
/// [`NotOpened::Unreachable`], [`NotOpened::HandshakeTimeout`], or how the handshake failed.
pub(super) async fn connect(
    address: &str,
    protocol: &'static str,
    params: ConnectParams,
    timing: Timing,
) -> Result<Connected, NotOpened> {
    let stream = timeout(timing.connect, TcpStream::connect(address))
        .await
        .map_err(|_| NotOpened::Unreachable)?
        .map_err(|_| NotOpened::Unreachable)?;
    // Keystrokes and screen updates are small; do not hold them back (Nagle).
    stream
        .set_nodelay(true)
        .map_err(|_| NotOpened::Unreachable)?;
    timeout(timing.handshake, handshake(stream, protocol, params))
        .await
        .map_err(|_| NotOpened::HandshakeTimeout)?
}

async fn handshake(
    mut stream: TcpStream,
    protocol: &'static str,
    params: ConnectParams,
) -> Result<Connected, NotOpened> {
    let (mut handshake, select) = Handshake::new(protocol, params);
    send(&mut stream, vec![select]).await?;
    let mut decoder = Decoder::new();
    let mut buffer = vec![0; READ_BYTES];
    loop {
        let read = stream
            .read(&mut buffer)
            .await
            .map_err(|_| NotOpened::GatewayClosed)?;
        if read == 0 {
            return Err(NotOpened::GatewayClosed);
        }
        let mut instructions = decoder
            .feed(&buffer[..read])
            .map_err(NotOpened::Garbled)?
            .into_iter();
        while let Some(instruction) = instructions.next() {
            match handshake
                .on_instruction(instruction)
                .map_err(|error| refusal(&error))?
            {
                HandshakeStep::Send(reply) => send(&mut stream, reply).await?,
                HandshakeStep::Ready { connection_id } => {
                    return Ok(Connected {
                        stream,
                        decoder,
                        after_ready: instructions.collect(),
                        connection_id: ConnectionId(connection_id),
                    });
                }
            }
        }
    }
}

/// Writes `instructions` to guacd in order, wiping each one and its wire text once written.
/// (The `ArgValues` copy inside the handshake is not wiped: guac-protocol, issue #1.)
async fn send(stream: &mut TcpStream, instructions: Vec<Instruction>) -> Result<(), NotOpened> {
    for mut instruction in instructions {
        let wire = Zeroizing::new(encode(&instruction));
        instruction.args.iter_mut().for_each(Zeroize::zeroize);
        stream
            .write_all(wire.as_bytes())
            .await
            .map_err(|_| NotOpened::GatewayClosed)?;
    }
    Ok(())
}

/// The handshake error without guacd's text (D-11): only its status survives.
fn refusal(error: &HandshakeError) -> NotOpened {
    match error {
        HandshakeError::Gateway { status, .. } => NotOpened::Refused { status: *status },
        _ => NotOpened::Broken,
    }
}
