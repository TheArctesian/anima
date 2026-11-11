//! Approving the route a site's router waits with, through the broker (R-055, VP-7, D-23).
//! The client then waits on `vpnChanged` for the router to serve the subnet (BD-16).

use async_graphql::{Context, ID, Object, Result};
use serde_json::json;

use super::mutations::{broker_error, broker_of, entry, policy_site, site_of, write};
use crate::{
    access::Authenticated,
    api_errors::{ErrorCode, FieldError, coded, invalid, schema_dependency},
    devices::Site,
    events::{AuditAction, Trail, TrailEntry},
    vpn::{Feed, VpnNode, broker::ipv4_of},
};

#[derive(Default)]
pub struct RouteMutation;

#[Object(name = "VpnRouteMutation")]
impl RouteMutation {
    /// Approves the waiting route of the site's router. Login or browser token only (D-23).
    #[graphql(guard = "Authenticated")]
    async fn approve_site_route(&self, ctx: &Context<'_>, site_id: ID) -> Result<bool> {
        let (broker, trail) = (broker_of(ctx)?, Trail::from_context(ctx)?);
        let site = site_of(ctx, &site_id).await?;
        let (router, node, name) = waiting_router(ctx, &site)?;
        broker
            .approve_route(node, &name)
            .await
            .map_err(|error| broker_error(error, Some("siteId")))?;
        write(ctx, trail, route_approved(&site, &router)).await?;
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

/// The audit entry and event of an approved route: the site, the router and the subnet.
fn route_approved(site: &Site, router: &VpnNode) -> TrailEntry {
    let route = site.subnet.to_string();
    entry(
        AuditAction::Accept,
        format!(
            "Approved the route {route} of {} for the site {}.",
            router.name, site.name
        ),
        json!({ "site": site.name, "router": router.name, "route": route }),
    )
}
