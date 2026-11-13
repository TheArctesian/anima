//! `anima device credential set rdp|vnc`: the password the hub signs in to the display with.

use std::path::PathBuf;

use zeroize::Zeroize as _;

use super::documents::{
    RdpCredentialSet, VncCredentialSet, rdp_credential_set, vnc_credential_set,
};
use super::input::{password, send_wiping};
use super::stored::table;
use crate::devices::get::resolve;
use crate::failure::Failure;
use crate::invocation::Invocation;

#[derive(Debug, clap::Args)]
pub struct PasswordArgs {
    /// The device's ID or its exact name.
    device: String,
    /// Read the password from this file, or from standard input for `-`. The file must be mode
    /// 0600 and yours. Without it, the password is asked for at the terminal.
    #[arg(long, value_name = "PATH")]
    from_file: Option<PathBuf>,
}

impl PasswordArgs {
    /// Stores the RDP password and prints whether the device now has a credential.
    ///
    /// # Errors
    /// [`Failure::Usage`] when the password cannot be asked for; [`Failure::Input`] for an
    /// unreadable file; [`Failure::NotFound`] for an unknown device; the hub's refusal.
    pub async fn run_rdp(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let device_id = resolve(&hub, &self.device).await?;
        let secret = self.password(invocation)?;
        let variables = rdp_credential_set::Variables {
            device_id,
            credential: rdp_credential_set::PasswordCredentialInput {
                password: secret.as_str().to_owned(),
            },
        };
        let answer = send_wiping::<RdpCredentialSet>(&hub, variables, |variables| {
            variables.credential.password.zeroize();
        })
        .await?;
        invocation.output.data(&answer.set_rdp_credential, |rdp| {
            table("RDP", rdp.enabled, rdp.port, rdp.credential_stored)
        })
    }

    /// Stores the VNC password and prints whether the device now has a credential.
    ///
    /// # Errors
    /// As [`PasswordArgs::run_rdp`].
    pub async fn run_vnc(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let device_id = resolve(&hub, &self.device).await?;
        let secret = self.password(invocation)?;
        let variables = vnc_credential_set::Variables {
            device_id,
            credential: vnc_credential_set::PasswordCredentialInput {
                password: secret.as_str().to_owned(),
            },
        };
        let answer = send_wiping::<VncCredentialSet>(&hub, variables, |variables| {
            variables.credential.password.zeroize();
        })
        .await?;
        invocation.output.data(&answer.set_vnc_credential, |vnc| {
            table("VNC", vnc.enabled, vnc.port, vnc.credential_stored)
        })
    }

    fn password(&self, invocation: &Invocation) -> Result<zeroize::Zeroizing<String>, Failure> {
        password(
            invocation.prompter,
            "The password",
            self.from_file.as_deref(),
        )
    }
}
