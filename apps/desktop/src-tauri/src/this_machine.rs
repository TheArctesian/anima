//! What the desktop app can learn about the computer it runs on, so the Add device form can be
//! filled in for it (R-058, R-060): its name, the OS user, the OS, its address on the Anima VPN
//! (NET-001, R-052) as the Anima VPN client reports it (ADR-005) and whether SSH answers there.
//! The web view passes in only the hub's VPN login server (`vpnLoginServer`).
//!
//! None of it is secret, but the user name and the address are not logged (LOG-007).

mod address;
mod name;
mod vpn_client;

#[cfg(test)]
mod tests;

use std::{net::Ipv4Addr, time::Duration};

use serde::{Serialize, Serializer, ser::SerializeStruct};
use tauri_plugin_log::log;

/// How long SSH on the VPN address may take to accept a connection.
const SSH_PATIENCE: Duration = Duration::from_millis(300);
const SSH_PORT: u16 = 22;

/// K1 `OperatingSystem`, for the systems the desktop app runs on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum OperatingSystem {
    Macos,
    Linux,
}

impl OperatingSystem {
    /// The system this build runs on; `None` off macOS and Linux.
    pub(crate) fn current() -> Option<Self> {
        if cfg!(target_os = "macos") {
            Some(Self::Macos)
        } else if cfg!(target_os = "linux") {
            Some(Self::Linux)
        } else {
            None
        }
    }
}

/// The name of this computer, as `this_machine_read` gives it; empty when the OS gives none.
pub(crate) fn computer_name() -> String {
    OperatingSystem::current()
        .map(name::computer_name)
        .unwrap_or_default()
}

/// This computer, as the Add device form needs it. Crosses to the client in camelCase.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThisMachine {
    /// The computer name; empty when the OS gives none.
    pub name: String,
    /// The OS user name of this process; empty when the OS gives none.
    pub user: String,
    pub os: OperatingSystem,
    /// This computer's own IPv4 address on the Anima VPN, as its Tailscale client logged in to
    /// the hub's login server reports it (ADR-005). `None` when that client is missing, slow,
    /// stopped, logged out or signed in to another tailnet.
    pub vpn_address: Option<Ipv4Addr>,
    /// Whether TCP port 22 accepted a connection on `vpn_address`; false without one.
    pub ssh_listening: bool,
}

/// Why this computer could not be read. Carries no OS error text.
///
/// - `unavailable`: the app runs on a system other than macOS or Linux.
/// - `failed`: the read did not finish.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ThisMachineError {
    #[error("this computer cannot be read on this system")]
    Unavailable,
    #[error("this computer could not be read")]
    Failed,
}

impl ThisMachineError {
    /// Stable code the client switches on.
    fn code(self) -> &'static str {
        match self {
            Self::Unavailable => "unavailable",
            Self::Failed => "failed",
        }
    }
}

/// Serialises as `{ "code": ..., "message": ... }` with the fixed `Display` text.
impl Serialize for ThisMachineError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut out = serializer.serialize_struct("ThisMachineError", 2)?;
        out.serialize_field("code", self.code())?;
        out.serialize_field("message", &self.to_string())?;
        out.end()
    }
}

/// Reads this computer from the OS, with its VPN address on `login_server`. Blocks for up to
/// the name, VPN client and SSH patience.
fn read(os: OperatingSystem, login_server: &str) -> ThisMachine {
    let vpn_address = vpn_client::client_program(os).and_then(|program| {
        vpn_client::own_address(program, login_server, vpn_client::CLIENT_PATIENCE)
    });
    let ssh_listening = address::ssh_answers(vpn_address, SSH_PORT, SSH_PATIENCE);
    ThisMachine {
        name: name::computer_name(os),
        user: whoami::username().unwrap_or_default(),
        os,
        vpn_address,
        ssh_listening,
    }
}

/// Reads this computer's name, user, OS, VPN address and whether SSH answers on it.
/// `login_server` is the hub's `vpnLoginServer`: only a client logged in to it gives the VPN
/// address.
#[tauri::command]
pub async fn this_machine_read(login_server: String) -> Result<ThisMachine, ThisMachineError> {
    let Some(os) = OperatingSystem::current() else {
        return Err(ThisMachineError::Unavailable);
    };
    let machine = tauri::async_runtime::spawn_blocking(move || read(os, &login_server))
        .await
        .map_err(|_| ThisMachineError::Failed);
    if let Err(error) = &machine {
        log::warn!("this computer could not be read: {}", error.code());
    }
    machine
}
