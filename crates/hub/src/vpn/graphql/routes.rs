//! Approving the route a site's router waits with, through the broker (R-055, VP-7, D-23).
//! The client then waits on `vpnChanged` for the router to serve the subnet (BD-16).

use async_graphql::{Context, ID, Object, Result};
use serde_json::json;

use super::{
    change::broker_change,
    mutations::{broker_of, entry, policy_site, site_of},
};
use crate::{
    access::{Authenticated, operator_only},
    api_errors::{ErrorCode, FieldError, coded, invalid, schema_dependency},
    devices::Site,
    events::{AuditAction, TrailEntry},
    vpn::{Feed, VpnNode, broker::ipv4_of},
};

#[derive(Default)]
pub struct RouteMutation;

#[Object(name = "VpnRouteMutation")]
impl RouteMutation {
    /// Approves the waiting route of the site's router. Login or browser token only (D-23).
    #[graphql(guard = "Authenticated")]
    async fn approve_site_route(&self, ctx: &Context<'_>, site_id: ID) -> Result<bool> {
        operator_only(ctx, "approveSiteRoute")?;
        let broker = broker_of(ctx)?;
        let site = site_of(ctx, &site_id).await?;
        let (router, node, name) = waiting_router(ctx, &site)?;
        let call = async move { broker.approve_route(node, &name).await };
        let entry = move |&(): &()| route_approved(&site, &router, node);
        broker_change(ctx, call, Some("siteId"), entry).await?;
        Ok(true)
    }
}

/// The router that waits for `site`'s subnet in the latest picture, its broker id and the
/// site's policy name; `VALIDATION` on `siteId` when no router waits.
fn waiting_router(ctx: &Context<'_>, site: &Site) -> Result<(VpnNode, u64, String)> {
    let picture = schema_dependency::<Feed>(ctx)?.latest().ok_or_else(|| {
        coded(
            "The hub has not read the VPN yet. Try again in a minute.",
            ErrorCode::Unavailable,
        )
    })?;
    let none_waits = || {
        invalid(&[FieldError::new(
            "siteId",
            "No router of this site waits for approval.",
        )])
    };
    let router = picture
        .sites
        .iter()
        .find(|path| path.site == site.id)
        .and_then(|path| path.waiting_router.clone())
        .ok_or_else(none_waits)?;
    let listed = picture.roster.as_ref().ok_or_else(none_waits)?;
    let node = listed
        .nodes
        .iter()
        .find(|node| ipv4_of(node) == Some(router.address))
        .map(|node| node.id)
        .ok_or_else(none_waits)?;
    let name = policy_site(listed, site)?;
    Ok((router, node, name))
}

/// The audit entry and event of an approved route: the site, the router, the broker id the hub
/// sent for it and the subnet.
fn route_approved(site: &Site, router: &VpnNode, sent: u64) -> TrailEntry {
    let route = site.subnet.to_string();
    entry(
        AuditAction::Accept,
        format!(
            "Approved the route {route} of {} for the site {}.",
            router.name, site.name
        ),
        json!({ "site": site.name, "router": router.name, "brokerNode": sent, "route": route }),
    )
}
