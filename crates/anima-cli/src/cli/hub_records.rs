//! The nouns for what the hub keeps and decides: its records and its settings.

use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::{builders, events, keys, notifications, releases, retention, vpn};

#[derive(Debug, clap::Subcommand)]
pub enum HubRecordCommand {
    /// Events (list, watch).
    Event {
        #[command(subcommand)]
        command: events::EventCommand,
    },
    /// The audit trail (list).
    Audit {
        #[command(subcommand)]
        command: events::AuditCommand,
    },
    /// Notification channels, settings and deliveries.
    Notify {
        #[command(subcommand)]
        command: notifications::NotifyCommand,
    },
    /// How long the hub keeps records (list, set).
    Retention {
        #[command(subcommand)]
        command: retention::RetentionCommand,
    },
    /// API keys (list, create, revoke).
    Key {
        #[command(subcommand)]
        command: keys::KeyCommand,
    },
    /// Releases of the desktop application (list, next-version, create, cancel, log).
    Release {
        #[command(subcommand)]
        command: releases::ReleaseCommand,
    },
    /// Release builders (list, enrol, revoke).
    Builder {
        #[command(subcommand)]
        command: builders::BuilderCommand,
    },
    /// The VPN (status, invite, router-key, register, renew, approve-route).
    Vpn {
        #[command(subcommand)]
        command: vpn::VpnCommand,
    },
}

impl HubRecordCommand {
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        match self {
            Self::Event { command } => command.run(invocation).await,
            Self::Audit { command } => command.run(invocation).await,
            Self::Notify { command } => command.run(invocation).await,
            Self::Retention { command } => command.run(invocation).await,
            Self::Key { command } => command.run(invocation).await,
            Self::Release { command } => command.run(invocation).await,
            Self::Builder { command } => command.run(invocation).await,
            Self::Vpn { command } => command.run(invocation).await,
        }
    }
}
