//! The nouns an operator works on: the machines and how they are reached.

use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::{agents, checks, devices, maintenance, sessions, sites, tags, web_interfaces};

#[derive(Debug, clap::Subcommand)]
pub enum FleetCommand {
    /// Devices (list, get, create, update, delete, wake, test, metrics, uptime, credential, trust).
    Device {
        #[command(subcommand)]
        command: devices::DeviceCommand,
    },
    /// Sites (list, create, set-wake-relay).
    Site {
        #[command(subcommand)]
        command: sites::SiteCommand,
    },
    /// Tags (list).
    Tag {
        #[command(subcommand)]
        command: tags::TagCommand,
    },
    /// Web interfaces of devices (list, create, update, delete, suggestion, accept-certificate).
    Web {
        #[command(subcommand)]
        command: web_interfaces::WebCommand,
    },
    /// Metrics agents (status, enrol, revoke).
    Agent {
        #[command(subcommand)]
        command: agents::AgentCommand,
    },
    /// Checks (list, get, create, update, delete, pause, resume, heartbeats, defaults, notifications).
    Check {
        #[command(subcommand)]
        command: checks::CheckCommand,
    },
    /// Maintenance windows (list, create, update, delete, start, stop).
    Maintenance {
        #[command(subcommand)]
        command: maintenance::MaintenanceCommand,
    },
    /// Remote sessions (list, end).
    Session {
        #[command(subcommand)]
        command: sessions::SessionCommand,
    },
    /// A terminal on a device through the hub (login token only).
    Ssh(sessions::terminal::SshArgs),
    /// Files on a device through the hub (ls, get, put; login token only).
    Files {
        #[command(subcommand)]
        command: sessions::files::FilesCommand,
    },
}

impl FleetCommand {
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        match self {
            Self::Device { command } => command.run(invocation).await,
            Self::Site { command } => command.run(invocation).await,
            Self::Tag { command } => command.run(invocation).await,
            Self::Web { command } => command.run(invocation).await,
            Self::Agent { command } => command.run(invocation).await,
            Self::Check { command } => command.run(invocation).await,
            Self::Maintenance { command } => command.run(invocation).await,
            Self::Session { command } => command.run(invocation).await,
            Self::Ssh(args) => args.run(invocation).await,
            Self::Files { command } => command.run(invocation).await,
        }
    }
}
