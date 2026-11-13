//! `anima vpn approve-route --site <site>`: approves the waiting route of the site's router.

use super::documents::{VpnApproveRoute, vpn_approve_route};
use crate::devices::list::site_id;
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::Table;

#[derive(Debug, clap::Args)]
pub struct ApproveRouteArgs {
    /// The site, by name or ID.
    #[arg(long)]
    site: String,
}

impl ApproveRouteArgs {
    /// Approves the route and prints the site.
    ///
    /// # Errors
    /// [`Failure::NotFound`] for an unknown site; the hub's refusal (no router waits).
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let site_id = site_id(&hub, &self.site).await?;
        let answer = hub
            .run::<VpnApproveRoute>(vpn_approve_route::Variables {
                site_id: site_id.clone(),
            })
            .await?;
        invocation
            .output
            .data(&answer.approve_site_route, |approved| {
                let mut table = Table::record();
                table.pair("Site", site_id.as_str());
                table.pair("Route approved", if *approved { "yes" } else { "no" });
                table
            })
    }
}
