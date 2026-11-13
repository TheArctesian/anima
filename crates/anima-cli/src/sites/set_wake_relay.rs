//! `anima site set-wake-relay`: the device that sends Wake-on-LAN packets at a site (D-2).

use super::documents::{SiteSetWakeRelay, site_set_wake_relay};
use super::shown::Shown;
use crate::devices::get::resolve;
use crate::devices::list::site_id;
use crate::failure::Failure;
use crate::invocation::Invocation;

#[derive(Debug, clap::Args)]
#[command(group = clap::ArgGroup::new("relay").required(true).multiple(false).args(["device", "clear"]))]
pub struct SetWakeRelayArgs {
    /// The site, by name or ID.
    site: String,
    /// The device that sends the packets, by name or ID.
    #[arg(long)]
    device: Option<String>,
    /// Give the site no wake relay.
    #[arg(long)]
    clear: bool,
}

impl SetWakeRelayArgs {
    /// Prints the site with its new relay.
    ///
    /// # Errors
    /// [`Failure::NotFound`] for an unknown site or device; the hub's refusal, such as a relay
    /// at another site.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let device_id = match &self.device {
            Some(device) => Some(resolve(&hub, device).await?),
            None => None,
        };
        let variables = site_set_wake_relay::Variables {
            site_id: site_id(&hub, &self.site).await?,
            device_id,
        };
        let site = hub
            .run::<SiteSetWakeRelay>(variables)
            .await?
            .set_site_wake_relay;
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
