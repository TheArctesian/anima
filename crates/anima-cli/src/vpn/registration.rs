//! `anima vpn register <auth id>` and `anima vpn renew <name>`: both answer with the node the
//! broker registered or renewed (vpn.md 6B and Section 7).

use super::documents::{VpnRegister, VpnRenew, vpn_register, vpn_renew};
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::Table;

#[derive(Debug, clap::Args)]
pub struct RegisterArgs {
    /// The auth ID the new computer shows on its own screen.
    auth_id: String,
}

#[derive(Debug, clap::Args)]
pub struct RenewArgs {
    /// The node's name, as `anima vpn status` shows it.
    name: String,
}

impl RegisterArgs {
    /// Registers the computer and prints its name, address and key expiry.
    ///
    /// # Errors
    /// [`Failure`] for a hub refusal (an unknown auth ID, the broker's limits).
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let answer = hub
            .run::<VpnRegister>(vpn_register::Variables {
                auth_id: self.auth_id,
            })
            .await?;
        invocation.output.data(&answer.register_vpn_node, |node| {
            table(&node.name, &node.address, node.key_expiry.as_deref())
        })
    }
}

impl RenewArgs {
    /// Sets a new key expiry and prints the node.
    ///
    /// # Errors
    /// [`Failure`] for a hub refusal (an unknown or tagged node).
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let answer = hub
            .run::<VpnRenew>(vpn_renew::Variables { name: self.name })
            .await?;
        invocation.output.data(&answer.renew_vpn_key, |node| {
            table(&node.name, &node.address, node.key_expiry.as_deref())
        })
    }
}

fn table(name: &str, address: &str, key_expiry: Option<&str>) -> Table {
    let mut table = Table::record();
    table.pair("Name", name);
    table.pair("Address", address);
    table.pair("Key expiry", key_expiry.unwrap_or("never"));
    table
}
