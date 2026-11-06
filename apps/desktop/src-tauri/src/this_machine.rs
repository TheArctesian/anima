//! What the desktop app can learn about the computer it runs on, so the Add device form can be
//! filled in for it (R-058, R-060): its name, the OS user, the OS, its address on the Anima VPN
//! (NET-001, R-052) and whether SSH answers there. The web view passes nothing in.
//!
//! None of it is secret, but the user name and the address are not logged (LOG-007).

mod address;
mod name;

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
    fn current() -> Option<Self> {
        if cfg!(target_os = "macos") {
            Some(Self::Macos)
        } else if cfg!(target_os = "linux") {
            Some(Self::Linux)
        } else {
            None
        }
    }
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
    /// The IPv4 address inside 100.64.0.0/10, or `None` when this computer is not on the VPN.
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

/// Reads this computer from the OS. Blocks for up to the name and SSH patience.
fn read(os: OperatingSystem) -> ThisMachine {
    let vpn_address = address::vpn_address_among(
        if_addrs::get_if_addrs()
            .unwrap_or_default()
            .iter()
            .map(if_addrs::Interface::ip),
    );
    let ssh_listening = vpn_address
        .is_some_and(|ip| address::accepts_connection((ip, SSH_PORT).into(), SSH_PATIENCE));
    ThisMachine {
        name: name::computer_name(os),
        user: whoami::fallible::username().unwrap_or_default(),
        os,
        vpn_address,
        ssh_listening,
    }
}

/// Reads this computer's name, user, OS, VPN address and whether SSH answers on it.
#[tauri::command]
pub async fn this_machine_read() -> Result<ThisMachine, ThisMachineError> {
    let Some(os) = OperatingSystem::current() else {
        return Err(ThisMachineError::Unavailable);
    };
    let machine = tauri::async_runtime::spawn_blocking(move || read(os))
        .await
        .map_err(|_| ThisMachineError::Failed);
    if let Err(error) = &machine {
        log::warn!("this computer could not be read: {}", error.code());
    }
    machine
}
