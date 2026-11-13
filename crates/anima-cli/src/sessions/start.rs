//! An SSH session's tickets and its recorded end (K4 Sections 1, 8 and 9): `startSession`,
//! `openFiles` and `endSession`. Each token goes straight into a [`Ticket`], which never
//! prints; the answers that hold one derive neither `Debug` nor `Serialize` (CMD-014).

use std::time::Duration;

use super::documents::{
    SessionEnd, SessionOpenFiles, SessionStart, session_end, session_open_files, session_start,
};
use super::terminal::ending::EndReason;
use crate::failure::Failure;
use crate::hub::tunnel::{Ticket, Tunnel};
use crate::hub::{Code, Hub, HubError};
use crate::output::text::enum_name;

/// K1's `DisplayInput.dpi` default; the terminal tunnel does not use it (K4 Section 8.4).
const DPI: i64 = 96;

/// A started SSH session and the ticket for its terminal tunnel.
#[derive(Debug)]
pub struct Started {
    pub session: String,
    pub ticket: Ticket,
    /// A Wake-on-LAN packet went first, so the opening may take up to 120 seconds (K4 Section 1).
    pub wake_sent: bool,
}

impl Started {
    /// How long opening the terminal tunnel may take: the hub's opening limit (8 seconds, or
    /// 120 after a wake) with room for the network.
    #[must_use]
    pub const fn opening_wait(&self) -> Duration {
        if self.wake_sent {
            Duration::from_secs(150)
        } else {
            Duration::from_secs(30)
        }
    }
}

/// Starts an SSH session on `device_id` with a display of `pixels` (width, height).
///
/// # Errors
/// The hub's refusal; FORBIDDEN, for an API key, says to sign in.
pub async fn start_ssh(
    hub: &Hub,
    device_id: String,
    pixels: (i64, i64),
) -> Result<Started, Failure> {
    let input = session_start::StartSessionInput {
        device_id,
        protocol: session_start::Protocol::SSH,
        display: session_start::DisplayInput {
            width_px: pixels.0,
            height_px: pixels.1,
            dpi: Some(DPI),
            timezone: None,
        },
    };
    let answer = hub
        .run::<SessionStart>(session_start::Variables { input })
        .await
        .map_err(login_needed)?
        .start_session;
    let ticket =
        Ticket::new(answer.token, &answer.tunnel_path, Tunnel::Terminal).map_err(HubError::from)?;
    Ok(Started {
        session: answer.session_id,
        ticket,
        wake_sent: answer.wake_sent,
    })
}

/// A files ticket for the live SSH session `session`, whose terminal tunnel is open.
///
/// # Errors
/// As [`start_ssh`].
pub async fn open_files(hub: &Hub, session: &str) -> Result<Ticket, Failure> {
    let variables = session_open_files::Variables {
        session_id: session.to_owned(),
    };
    let answer = hub
        .run::<SessionOpenFiles>(variables)
        .await
        .map_err(login_needed)?
        .open_files;
    Ok(Ticket::new(answer.token, &answer.tunnel_path, Tunnel::Files).map_err(HubError::from)?)
}

/// Ends `session` (a session already ended comes back unchanged) and returns the reason the
/// hub recorded, which is authoritative (K4 Section 8.4); `None` when the hub cannot say.
pub async fn end(hub: &Hub, session: &str) -> Option<EndReason> {
    let variables = session_end::Variables {
        id: session.to_owned(),
    };
    let ended = hub.run::<SessionEnd>(variables).await.ok()?.end_session;
    ended
        .end_reason
        .and_then(|reason| EndReason::parse(&enum_name(&reason)))
}

/// FORBIDDEN here means an API key: sessions and files need the login token (INT-003).
fn login_needed(error: HubError) -> Failure {
    if error.code() != Code::Forbidden {
        return error.into();
    }
    Failure::Hub(HubError::Refused {
        code: Code::Forbidden,
        message: "Sessions and files need a login token, and this is an API key: sign in with \
                  `anima auth login`."
            .to_owned(),
        fields: Vec::new(),
    })
}
