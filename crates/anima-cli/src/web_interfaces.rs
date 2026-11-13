//! `anima web`: the web pages of devices, such as admin consoles (CMD-002). Their pixels stay
//! in the desktop application; these commands manage the list.

pub mod accept_certificate;
pub mod create;
pub mod delete;
pub mod documents;
pub mod fields;
pub mod list;
pub mod shown;
pub mod suggestion;
pub mod update;

use crate::failure::Failure;
use crate::invocation::Invocation;

#[derive(Debug, clap::Subcommand)]
pub enum WebCommand {
    /// List a device's web interfaces.
    List(list::ListArgs),
    /// Add a web interface to a device.
    Create(create::CreateArgs),
    /// Replace a web interface's address, label and place to open.
    Update(update::UpdateArgs),
    /// Delete a web interface.
    Delete(delete::DeleteArgs),
    /// Take or turn down a suggested web interface.
    #[command(subcommand)]
    Suggestion(suggestion::SuggestionCommand),
    /// Pin a web interface's certificate by its fingerprint.
    AcceptCertificate(accept_certificate::AcceptCertificateArgs),
}

impl WebCommand {
    /// Runs the command.
    ///
    /// # Errors
    /// [`Failure`] for a refusal, an unknown device or web interface, or a missing token.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        match self {
            Self::List(args) => args.run(invocation).await,
            Self::Create(args) => args.run(invocation).await,
            Self::Update(args) => args.run(invocation).await,
            Self::Delete(args) => args.run(invocation).await,
            Self::Suggestion(command) => command.run(invocation).await,
            Self::AcceptCertificate(args) => args.run(invocation).await,
        }
    }
}
