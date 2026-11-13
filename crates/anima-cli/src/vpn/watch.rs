//! `anima vpn status --watch`: one line for each new VPN picture (K1 `vpnChanged`).

use graphql_client::GraphQLQuery;

use crate::failure::Failure;
use crate::hub::scalars::DateTime;
use crate::hub::subscription::Subscription;
use crate::invocation::Invocation;
use crate::output::stream::{self, Reader};
use crate::output::text::enum_name;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/vpn/watch.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct VpnChanged;

/// Prints each picture as a line (time, fresh or stale, broker, nodes online, each site's
/// state and path) or, with `--json`, as one K1 `VpnPicture` object per line.
///
/// # Errors
/// [`Failure`] when the hub refuses the token or the connection is lost.
pub async fn run(invocation: &Invocation) -> Result<(), Failure> {
    let hub = invocation.hub()?;
    invocation
        .output
        .note("Waiting for the next picture; the hub sends one about every minute.");
    let mut pictures = Subscription::open::<VpnChanged>(&hub, vpn_changed::Variables).await?;
    while let Some(answer) = pictures.next().await {
        let picture = answer?.vpn_changed;
        if stream::emit(invocation.output.json(), &picture, line)? == Reader::Gone {
            break;
        }
    }
    Ok(())
}

fn line(picture: &vpn_changed::VpnChangedVpnChanged) -> String {
    let online = picture.nodes.iter().filter(|node| node.online).count();
    let sites: Vec<String> = picture
        .sites
        .iter()
        .map(|entry| {
            format!(
                "{} {} {}",
                entry.site.name,
                enum_name(&entry.state),
                enum_name(&entry.path)
            )
        })
        .collect();
    format!(
        "{}  {}  broker {}  nodes {online}/{} online  sites: {}",
        picture.read_at,
        if picture.fresh { "fresh" } else { "stale" },
        enum_name(&picture.broker),
        picture.nodes.len(),
        if sites.is_empty() {
            "none".to_owned()
        } else {
            sites.join(", ")
        }
    )
}
