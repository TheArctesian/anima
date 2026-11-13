//! `anima site create`: a site is a network the hub reaches through the VPN.

use super::documents::{SiteCreate, site_create};
use super::shown::Shown;
use crate::failure::Failure;
use crate::invocation::Invocation;

#[derive(Debug, clap::Args)]
pub struct CreateArgs {
    /// The site's name.
    name: String,
    /// The site's network, such as 10.10.0.0/24.
    #[arg(long)]
    subnet: String,
}

impl CreateArgs {
    /// Prints the new site.
    ///
    /// # Errors
    /// The hub's refusal, such as `VALIDATION` for a subnet that is not a network.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let variables = site_create::Variables {
            name: self.name,
            subnet: self.subnet,
        };
        let site = hub.run::<SiteCreate>(variables).await?.create_site;
        invocation.output.data(&site, |site| {
            Shown {
                id: &site.id,
                name: &site.name,
                subnet: &site.subnet,
                wake_relay: site.wake_relay.as_ref().map(|relay| relay.name.as_str()),
            }
            .table()
        })
    }
}
