//! `anima device credential`: the passwords and keys the hub keeps to sign in to a device
//! (CMD-025). The hub never returns one; these commands only store and clear.

pub mod clear;
pub mod documents;
pub mod input;
pub mod set_password;
pub mod set_ssh;
pub mod stored;

use crate::failure::Failure;
use crate::invocation::Invocation;

#[derive(Debug, clap::Subcommand)]
pub enum CredentialCommand {
    /// Store a password or private key, from a prompt, standard input or a file.
    #[command(subcommand)]
    Set(SetCommand),
    /// Clear a stored credential.
    #[command(subcommand)]
    Clear(ClearCommand),
}

#[derive(Debug, clap::Subcommand)]
pub enum SetCommand {
    /// The SSH password or private key.
    Ssh(set_ssh::SshArgs),
    /// The RDP password.
    Rdp(set_password::PasswordArgs),
    /// The VNC password.
    Vnc(set_password::PasswordArgs),
}

#[derive(Debug, clap::Subcommand)]
pub enum ClearCommand {
    /// The SSH credential.
    Ssh(clear::ClearArgs),
    /// The RDP credential.
    Rdp(clear::ClearArgs),
    /// The VNC credential.
    Vnc(clear::ClearArgs),
}

impl CredentialCommand {
    /// Runs the command.
    ///
    /// # Errors
    /// [`Failure`] for a refusal, an unknown device or a secret that cannot be read.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        match self {
            Self::Set(SetCommand::Ssh(args)) => args.run(invocation).await,
            Self::Set(SetCommand::Rdp(args)) => args.run_rdp(invocation).await,
            Self::Set(SetCommand::Vnc(args)) => args.run_vnc(invocation).await,
            Self::Clear(ClearCommand::Ssh(args)) => args.ssh(invocation).await,
            Self::Clear(ClearCommand::Rdp(args)) => args.rdp(invocation).await,
            Self::Clear(ClearCommand::Vnc(args)) => args.vnc(invocation).await,
        }
    }
}
