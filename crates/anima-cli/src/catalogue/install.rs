//! `anima catalogue install <id> --device <d>`: asks the hub to install one entry on a
//! device (R-234). The hub refuses with a fixed reason until task H6 exists, and reaches no
//! machine; the command prints the reason and ends with exit status 1.

use super::documents::{CatalogueInstallAsk, catalogue_install_ask};
use crate::devices::get::resolve;
use crate::failure::{Failure, exit};
use crate::hub::Hub;
use crate::invocation::Invocation;
use crate::output::Table;
use crate::output::text::enum_name;

pub type Answer = catalogue_install_ask::CatalogueInstallAskInstallCatalogueEntry;

#[derive(Debug, clap::Args)]
pub struct InstallArgs {
    /// The identifier of the entry, as `anima catalogue list` shows it.
    id: String,
    /// The device's ID or its exact name.
    #[arg(long)]
    device: String,
    /// The digest of the image (`sha256:...`, as `anima catalogue show` gives it): a
    /// generated entry is installed only when the request names it (OD-HOST-50).
    #[arg(long)]
    digest: Option<String>,
}

/// The answer of the hub to the install of `entry` on `device` (an ID or an exact name),
/// with the digest that the request names.
///
/// # Errors
/// [`Failure::NotFound`] for an unknown device; the hub's refusal.
pub async fn ask(
    hub: &Hub,
    entry: &str,
    device: &str,
    digest: Option<String>,
) -> Result<Answer, Failure> {
    let variables = catalogue_install_ask::Variables {
        entry_id: entry.to_owned(),
        device_id: resolve(hub, device).await?,
        digest,
    };
    Ok(hub
        .run::<CatalogueInstallAsk>(variables)
        .await?
        .install_catalogue_entry)
}

impl InstallArgs {
    /// Prints the answer, or with `--json` the K1 `CatalogueInstall`.
    ///
    /// # Errors
    /// [`Failure::Ended`] with exit status 1 when the hub refuses the install; the errors of
    /// [`ask`].
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let answer = ask(&invocation.hub()?, &self.id, &self.device, self.digest).await?;
        if invocation.output.json() {
            invocation.output.data(&answer, |_| Table::record())?;
        }
        if answer.needs_open_to_network {
            invocation
                .output
                .note("The entry needs the setting Open to the network at the machine.");
        }
        match (answer.refusal, answer.reason) {
            (Some(refusal), reason) => Err(Failure::Ended {
                message: reason.unwrap_or_else(|| enum_name(&refusal)),
                status: exit::FAILURE,
            }),
            (None, _) => invocation.output.line("The hub accepted the install."),
        }
    }
}
