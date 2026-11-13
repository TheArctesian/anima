//! `anima device credential clear ssh|rdp|vnc`: the hub forgets the stored credential.

use super::documents::{
    RdpCredentialClear, SshCredentialClear, VncCredentialClear, rdp_credential_clear,
    ssh_credential_clear, vnc_credential_clear,
};
use super::stored::table;
use crate::devices::get::resolve;
use crate::failure::Failure;
use crate::hub::Hub;
use crate::invocation::Invocation;

#[derive(Debug, clap::Args)]
pub struct ClearArgs {
    /// The device's ID or its exact name.
    device: String,
}

impl ClearArgs {
    /// Asks for confirmation (`--yes` skips it), clears the SSH credential and prints the result.
    ///
    /// # Errors
    /// [`Failure::NotFound`] for an unknown device; [`Failure::Usage`] when it cannot ask; the
    /// hub's refusal.
    pub async fn ssh(self, invocation: &Invocation) -> Result<(), Failure> {
        let Some((hub, device_id)) = self.confirmed(invocation, "SSH").await? else {
            return Ok(());
        };
        let variables = ssh_credential_clear::Variables { device_id };
        let answer = hub.run::<SshCredentialClear>(variables).await?;
        invocation.output.data(&answer.clear_ssh_credential, |ssh| {
            table("SSH", ssh.enabled, ssh.port, ssh.credential_stored)
        })
    }

    /// As [`ClearArgs::ssh`], for RDP.
    ///
    /// # Errors
    /// As [`ClearArgs::ssh`].
    pub async fn rdp(self, invocation: &Invocation) -> Result<(), Failure> {
        let Some((hub, device_id)) = self.confirmed(invocation, "RDP").await? else {
            return Ok(());
        };
        let variables = rdp_credential_clear::Variables { device_id };
        let answer = hub.run::<RdpCredentialClear>(variables).await?;
        invocation.output.data(&answer.clear_rdp_credential, |rdp| {
            table("RDP", rdp.enabled, rdp.port, rdp.credential_stored)
        })
    }

    /// As [`ClearArgs::ssh`], for VNC.
    ///
    /// # Errors
    /// As [`ClearArgs::ssh`].
    pub async fn vnc(self, invocation: &Invocation) -> Result<(), Failure> {
        let Some((hub, device_id)) = self.confirmed(invocation, "VNC").await? else {
            return Ok(());
        };
        let variables = vnc_credential_clear::Variables { device_id };
        let answer = hub.run::<VncCredentialClear>(variables).await?;
        invocation.output.data(&answer.clear_vnc_credential, |vnc| {
            table("VNC", vnc.enabled, vnc.port, vnc.credential_stored)
        })
    }

    /// The hub and the device's ID when the operator agrees, or `None` after saying nothing
    /// was cleared.
    async fn confirmed(
        &self,
        invocation: &Invocation,
        protocol: &str,
    ) -> Result<Option<(Hub, String)>, Failure> {
        let hub = invocation.hub()?;
        let device_id = resolve(&hub, &self.device).await?;
        let question = format!("Clear the stored {protocol} credential of {}?", self.device);
        if !invocation.prompter.confirm(&question)? {
            invocation.output.note("Nothing was cleared.");
            return Ok(None);
        }
        Ok(Some((hub, device_id)))
    }
}
