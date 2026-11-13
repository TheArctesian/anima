//! `anima site list`: every site, with its subnet and wake relay. K1 has no pages for sites.

use super::documents::{SiteList, site_list};
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::Table;

/// Prints the sites as a table or, with `--json`, as the K1 `Site` fields selected.
///
/// # Errors
/// [`Failure`] for a hub refusal.
pub async fn run(invocation: &Invocation) -> Result<(), Failure> {
    let hub = invocation.hub()?;
    let answer = hub.run::<SiteList>(site_list::Variables).await?;
    invocation.output.data(&answer.sites, |sites| {
        let mut table = Table::new(vec!["NAME", "SUBNET", "WAKE RELAY", "ID"]);
        for site in sites {
            let relay = site.wake_relay.as_ref().map_or("-", |relay| &relay.name);
            table.row(vec![
                site.name.as_str().into(),
                site.subnet.as_str().into(),
                relay.into(),
                site.id.as_str().into(),
            ]);
        }
        table
    })
}
