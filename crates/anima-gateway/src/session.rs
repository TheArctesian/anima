//! One session of the gateway: the hub's connection on one side, the device on the other.
//! `serve` runs it from the hub's first byte to the end; a worker process runs one session
//! and nothing else, so a fault ends that session only.
//!
//! I/O edge. Nothing here logs; the caller logs how the session ended. No end reason holds
//! a value from the hub, the device or an account (LOG-007).

mod hub;
mod input;
mod opening;
mod running;

use std::time::Duration;

use guac_protocol::{Instruction, encode};
use tokio::io::{AsyncRead, AsyncReadExt as _, AsyncWrite, AsyncWriteExt as _};
use tokio::net::TcpStream;
use tokio::time::timeout;

use crate::frames::Pace;
use hub::Hub;
use opening::First;

/// Guacamole statuses the gateway sends (guacamole-server `protocol-types.h`). The hub
/// ends the session `AUTH_FAILED` for `CLIENT_UNAUTHORIZED` and `GATEWAY_ERROR` for the
/// others (K4 Section 5).
mod status {
    pub(super) const SERVER_ERROR: u16 = 0x0200;
    pub(super) const UPSTREAM_ERROR: u16 = 0x0203;
    pub(super) const UPSTREAM_NOT_FOUND: u16 = 0x0207;
    pub(super) const CLIENT_UNAUTHORIZED: u16 = 0x0301;
    pub(super) const CLIENT_TIMEOUT: u16 = 0x0308;
}

/// The waits and limits of a session.
#[derive(Clone, Debug)]
pub struct Settings {
    /// From the hub's first byte to `connect`.
    pub handshake_wait: Duration,
    /// Each of: connecting to the device, its offer, its sign-in.
    pub device_wait: Duration,
    /// For the operator to answer a prompt for a credential.
    pub answer_wait: Duration,
    pub pace: Pace,
    /// Another gateway (guacd) that serves the protocols this one does not, as `host:port`.
    pub other_gateway: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            handshake_wait: Duration::from_secs(10),
            device_wait: Duration::from_secs(8),
            answer_wait: Duration::from_secs(120),
            pace: Pace::default(),
            other_gateway: None,
        }
    }
}

/// How a session ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Ended {
    #[error("the hub closed the session")]
    ByHub,
    #[error("the device closed the session")]
    ByDevice,
    #[error("the handshake was refused or did not finish in time")]
    Refused,
    #[error("the device could not be reached")]
    DeviceUnreachable,
    #[error("the device did not answer as a VNC server")]
    NotVnc,
    #[error("the device offers no sign-in method the gateway has")]
    NoSignInMethod,
    #[error("the operator did not answer the prompt in time")]
    NoAnswer,
    #[error("the device refused the sign-in")]
    SignInRefused,
    #[error("the sign-in did not finish")]
    SignInFailed,
    #[error("the device sent what a VNC server may not send")]
    DeviceFault,
    #[error("the system gave no random numbers")]
    NoRandomNumbers,
    #[error("the gateway could not make a frame")]
    Fault,
    #[error("the session went to the other gateway")]
    ByOtherGateway,
}

/// Runs one session on the hub's connection. `connection_id` is the session's ID for
/// `ready`: new and random for each session (GW-014).
pub async fn serve<S>(stream: S, connection_id: &str, settings: &Settings) -> Ended
where
    S: AsyncRead + AsyncWrite + Send + 'static,
{
    let mut hub = Hub::new(stream);
    let first = timeout(
        settings.handshake_wait,
        opening::handshake(&mut hub, connection_id),
    )
    .await;
    let opening = match first {
        Ok(Ok(First::Vnc(opening))) => opening,
        Ok(Ok(First::Other(select))) => return other_gateway(&mut hub, &select, settings).await,
        Ok(Err(ended)) => return ended,
        Err(_) => return opening::refuse(&mut hub, status::CLIENT_TIMEOUT, Ended::Refused).await,
    };
    match opening::sign_in(&mut hub, opening, settings).await {
        Ok(opened) => running::run(&mut hub, opened, settings).await,
        Err(ended) => ended,
    }
}

/// Passes a session of another protocol to the other gateway, when there is one: the
/// hub's instructions as they are, and the other gateway's bytes as they are.
async fn other_gateway<S>(hub: &mut Hub<S>, select: &Instruction, settings: &Settings) -> Ended
where
    S: AsyncRead + AsyncWrite + Send + 'static,
{
    let not_served = crate::handshake::Refused::NotServed.status();
    let Some(address) = &settings.other_gateway else {
        return opening::refuse(hub, not_served, Ended::Refused).await;
    };
    let connecting = timeout(settings.device_wait, TcpStream::connect(address)).await;
    let Ok(Ok(mut other)) = connecting else {
        return opening::refuse(hub, status::UPSTREAM_NOT_FOUND, Ended::Refused).await;
    };
    if other.write_all(encode(select).as_bytes()).await.is_err() {
        return Ended::ByOtherGateway;
    }
    let mut buffer = vec![0; 64 * 1024];
    loop {
        let passed = tokio::select! {
            instruction = hub.next() => match instruction {
                Some(instruction) => other.write_all(encode(&instruction).as_bytes()).await.is_ok(),
                None => return Ended::ByHub,
            },
            read = other.read(&mut buffer) => match read {
                Ok(count @ 1..) => hub.send_wire(&buffer[..count]).await.is_ok(),
                _ => false,
            },
        };
        if !passed {
            return Ended::ByOtherGateway;
        }
    }
}
