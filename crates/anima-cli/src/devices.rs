//! `anima device`: the devices of the hub (CMD-002). The worked example for every other noun;
//! see the crate README.

pub mod create;
pub mod credentials;
pub mod delete;
pub mod documents;
pub mod get;
pub mod list;
pub mod metrics;
pub mod saved;
pub mod settings;
pub mod test;
pub mod trust;
pub mod update;
pub mod uptime;
pub mod wake;

use crate::failure::Failure;
use crate::invocation::Invocation;

#[derive(Debug, clap::Subcommand)]
pub enum DeviceCommand {
    /// List devices, with filters and pages.
    List(list::ListArgs),
    /// Show one device by ID or exact name.
    Get(get::GetArgs),
    /// Add a device to a site.
    Create(Box<create::CreateArgs>),
    /// Change a device's name, address, tags or access settings.
    Update(Box<update::UpdateArgs>),
    /// Delete a device with its credentials, checks and history.
    Delete(delete::DeleteArgs),
    /// Send a Wake-on-LAN packet from the site's relay.
    Wake(wake::WakeArgs),
    /// Test whether the hub can reach a device and sign in.
    Test(test::TestArgs),
    /// Show a device's metric series.
    Metrics(metrics::MetricsArgs),
    /// Show a device's uptime over a window.
    Uptime(uptime::UptimeArgs),
    /// Store or clear the credentials the hub signs in with.
    #[command(subcommand)]
    Credential(credentials::CredentialCommand),
    /// Accept a host key or certificate by its fingerprint.
    #[command(subcommand)]
    Trust(trust::TrustCommand),
}

impl DeviceCommand {
    /// Runs the command.
    ///
    /// # Errors
    /// [`Failure`] for a refusal, an unknown device or a missing token.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        match self {
            Self::List(args) => args.run(invocation).await,
            Self::Get(args) => args.run(invocation).await,
            Self::Create(args) => args.run(invocation).await,
            Self::Update(args) => args.run(invocation).await,
            Self::Delete(args) => args.run(invocation).await,
            Self::Wake(args) => args.run(invocation).await,
            Self::Test(args) => args.run(invocation).await,
            Self::Metrics(args) => args.run(invocation).await,
            Self::Uptime(args) => args.run(invocation).await,
            Self::Credential(command) => command.run(invocation).await,
            Self::Trust(command) => command.run(invocation).await,
        }
    }
}
