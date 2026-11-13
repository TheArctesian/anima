//! `anima vpn approve-route --site <site>`: approves the waiting route of the site's router.

use super::documents::{VpnApproveRoute, vpn_approve_route};
use crate::devices::documents::{SiteNames, site_names};
use crate::failure::{Failure, Hint};
use crate::hub::Hub;
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
        let site_id = resolve_site(&hub, &self.site).await?;
        let answer = hub
            .run::<VpnApproveRoute>(vpn_approve_route::Variables {
                site_id: site_id.clone(),
            })
            .await?;
        invocation.output.data(&answer, |answer| {
            let mut table = Table::record();
            table.pair("Site", site_id.as_str());
            table.pair(
                "Route approved",
                if answer.approve_site_route {
                    "yes"
                } else {
                    "no"
                },
            );
            table
        })
    }
}

/// The ID of the site `site` names: an ID as it is, or the site with exactly that name.
///
/// # Errors
/// [`Failure::NotFound`] when no site has the name; the hub's refusal.
pub async fn resolve_site(hub: &Hub, site: &str) -> Result<String, Failure> {
    if uuid::Uuid::parse_str(site).is_ok() {
        return Ok(site.to_owned());
    }
    let sites = hub.run::<SiteNames>(site_names::Variables).await?.sites;
    sites
        .into_iter()
        .find(|candidate| candidate.name == site)
        .map(|found| found.id)
        .ok_or_else(|| Failure::NotFound {
            message: format!("There is no site named {site}."),
            hint: Some(Hint::run("anima site list")),
        })
}
