//! `anima device trust accept-host-key`: trusts the SSH host key with this fingerprint, and no
//! other (SEC-006).

use super::documents::{HostKeyAccept, host_key_accept};
use super::fingerprint;
use crate::devices::get::resolve;
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::Table;

#[derive(Debug, clap::Args)]
pub struct HostKeyArgs {
    /// The device's ID or its exact name.
    device: String,
    /// The SHA-256 fingerprint of the key, as `anima device test` shows it.
    #[arg(value_parser = fingerprint)]
    fingerprint: String,
}

impl HostKeyArgs {
    /// Trusts the key and prints what was accepted.
    ///
    /// # Errors
    /// [`Failure::NotFound`] for an unknown device; the hub's `CONFLICT` (exit 6) when the
    /// fingerprint is not the key the hub saw.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let variables = host_key_accept::Variables {
            device_id: resolve(&hub, &self.device).await?,
            fingerprint_sha256: self.fingerprint,
        };
        let key = hub.run::<HostKeyAccept>(variables).await?.accept_host_key;
        invocation
            .output
            .note(&format!("Accepted the SSH host key of {}.", self.device));
        invocation.output.data(&key, |key| {
            let mut table = Table::record();
            table.pair("Algorithm", key.algorithm.as_str());
            table.pair("Fingerprint", key.fingerprint_sha256.as_str());
            table.pair("First seen", key.first_seen_at.as_str());
            table
        })
    }
}
