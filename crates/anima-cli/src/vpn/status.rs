//! `anima vpn status`: the VPN as the hub's own node sees it (VPN-005, VPN-008).

use std::fmt::Write as _;

use super::documents::{VpnStatus, vpn_status};
use super::watch;
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::Table;
use crate::output::text::enum_name;

#[derive(Debug, clap::Args)]
pub struct StatusArgs {
    /// Print a line for each new picture until interrupted, instead of the picture now.
    #[arg(long)]
    watch: bool,
}

impl StatusArgs {
    /// Prints the picture as label and value lines or, with `--json`, as the K1 `VpnPicture`
    /// (`null` while the hub has none).
    ///
    /// # Errors
    /// [`Failure`] for a hub refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        if self.watch {
            return watch::run(invocation).await;
        }
        let hub = invocation.hub()?;
        let answer = hub.run::<VpnStatus>(vpn_status::Variables).await?;
        invocation
            .output
            .data(&answer.vpn, |picture| table(picture.as_ref()))
    }
}

fn table(picture: Option<&vpn_status::VpnStatusVpn>) -> Table {
    let mut table = Table::record();
    let Some(picture) = picture else {
        table.pair(
            "VPN",
            "no picture yet: the hub has no LocalAPI proxy or has not read the VPN",
        );
        return table;
    };
    let fresh = if picture.fresh { "fresh" } else { "stale" };
    table.pair("Read", format!("{} ({fresh})", picture.read_at));
    table.pair("Broker", enum_name(&picture.broker));
    table.pair("Hub", node(&picture.hub));
    for entry in &picture.sites {
        let router = entry
            .router
            .as_ref()
            .map_or("no router", |r| r.name.as_str());
        let mut text = format!(
            "{}, {router}, {}",
            enum_name(&entry.state),
            enum_name(&entry.path)
        );
        if let Some(waiting) = &entry.waiting_router {
            let _ = write!(text, ", route of {} waits for approval", waiting.name);
        }
        table.pair(&format!("Site {}", entry.site.name), text);
    }
    for peer in &picture.nodes {
        table.pair(&format!("Node {}", peer.name), node(peer));
    }
    table
}

fn node(node: &vpn_status::VpnNodeRow) -> String {
    let online = if node.online { "online" } else { "offline" };
    let expiry = node.key_expiry.as_deref().unwrap_or("no expiry");
    let mut text = format!(
        "{} {}, {online}, {}, key {} ({expiry})",
        enum_name(&node.role),
        node.address,
        enum_name(&node.path),
        enum_name(&node.key_state)
    );
    if !node.waiting_routes.is_empty() {
        let _ = write!(text, ", waiting routes {}", node.waiting_routes.join(" "));
    }
    if !node.in_anima {
        text.push_str(", not in Anima");
    }
    text
}
