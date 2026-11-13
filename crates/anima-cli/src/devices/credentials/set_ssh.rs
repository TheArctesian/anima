//! `anima device credential set ssh`: a password or a private key for the hub to sign in with.
//! The hub keeps it and never returns it (K1).

use std::path::PathBuf;

use zeroize::Zeroize as _;

use super::documents::ssh_credential_set::SshCredentialInput as Input;
use super::documents::{SshCredentialSet, ssh_credential_set};
use super::input::{from_file_only, password, send_wiping};
use super::stored::table;
use crate::devices::get::resolve;
use crate::failure::Failure;
use crate::invocation::Invocation;

#[derive(Debug, clap::Args)]
pub struct SshArgs {
    /// The device's ID or its exact name.
    device: String,
    /// The secret is a private key (PEM), not a password. It needs `--from-file`.
    #[arg(long)]
    private_key: bool,
    /// Read the password or key from this file, or from standard input for `-`. The file must
    /// be mode 0600 and yours. Without it, a password is asked for at the terminal.
    #[arg(long, value_name = "PATH")]
    from_file: Option<PathBuf>,
    /// Ask for the key's passphrase at the terminal.
    #[arg(long, requires = "private_key", conflicts_with = "passphrase_file")]
    ask_passphrase: bool,
    /// Read the key's passphrase from this file, under the same rules as `--from-file`.
    #[arg(long, requires = "private_key", value_name = "PATH")]
    passphrase_file: Option<PathBuf>,
}

impl SshArgs {
    /// Stores the credential and prints whether the device now has one.
    ///
    /// # Errors
    /// [`Failure::Usage`] when a secret cannot be asked for; [`Failure::Input`] for an unreadable
    /// file; [`Failure::NotFound`] for an unknown device; the hub's refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let device_id = resolve(&hub, &self.device).await?;
        let variables = ssh_credential_set::Variables {
            device_id,
            credential: self.credential(invocation)?,
        };
        let answer = send_wiping::<SshCredentialSet>(&hub, variables, |variables| {
            wipe(&mut variables.credential);
        })
        .await?;
        invocation.output.data(&answer.set_ssh_credential, |ssh| {
            table("SSH", ssh.enabled, ssh.port, ssh.credential_stored)
        })
    }

    /// The `@oneOf` input: the password, or the key with its passphrase.
    fn credential(&self, invocation: &Invocation) -> Result<Input, Failure> {
        let prompter = invocation.prompter;
        let from_file = self.from_file.as_deref();
        if !self.private_key {
            let password = password(prompter, "The SSH password", from_file)?;
            return Ok(Input::Password(password.as_str().to_owned()));
        }
        let key = from_file_only("The SSH private key", from_file)?;
        let passphrase = if self.ask_passphrase {
            Some(prompter.ask_secret("Key passphrase", "--passphrase-file")?)
        } else if let Some(path) = &self.passphrase_file {
            Some(password(prompter, "The key passphrase", Some(path))?)
        } else {
            None
        };
        Ok(Input::PrivateKey(ssh_credential_set::PrivateKeyInput {
            key: key.as_str().to_owned(),
            passphrase: passphrase.map(|passphrase| passphrase.as_str().to_owned()),
        }))
    }
}

fn wipe(credential: &mut Input) {
    match credential {
        Input::Password(password) => password.zeroize(),
        Input::PrivateKey(key) => {
            key.key.zeroize();
            key.passphrase.zeroize();
        }
    }
}
