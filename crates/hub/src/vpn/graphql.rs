//! The `vpn` query and the `vpnChanged` subscription (VPN-005, VPN-009, K1). Each view borrows
//! from the shared picture, so a picture is never copied for each field.

use std::sync::Arc;

use async_graphql::{
    Context, Object, Result, Subscription,
    dataloader::DataLoader,
    futures_util::stream::{self, Stream},
};
use chrono::{DateTime, Utc};

use super::{
    Feed,
    model::{BrokerState, KeyState, Picture, Role, RouteState, SitePath, VpnNode, VpnPath},
};
use crate::{
    access::Authenticated,
    api_errors::{internal, schema_dependency},
    devices::{Device, DevicesById, Site, SitesById},
    scalars::{Cidr, IpAddr},
};

#[derive(Default)]
pub struct Query;

#[Object(name = "VpnQuery")]
impl Query {
    /// The VPN picture; null when the hub has no `LocalAPI` proxy or has not read it yet
    /// (VPN-005).
    #[graphql(guard = "Authenticated")]
    #[expect(
        clippy::unused_async,
        clippy::unused_async_trait_impl,
        reason = "the Object macro requires async resolvers"
    )]
    async fn vpn(&self, ctx: &Context<'_>) -> Result<Option<PictureView>> {
        Ok(schema_dependency::<Feed>(ctx)?.latest().map(PictureView))
    }
}

#[derive(Default)]
pub struct Subscription;

#[Subscription(name = "VpnSubscription")]
impl Subscription {
    /// Each new VPN picture, about once a minute.
    #[graphql(guard = "Authenticated")]
    #[expect(
        clippy::unused_async,
        clippy::unused_async_trait_impl,
        reason = "the Subscription macro requires async resolvers"
    )]
    async fn vpn_changed(&self, ctx: &Context<'_>) -> Result<impl Stream<Item = PictureView>> {
        let receiver = schema_dependency::<Feed>(ctx)?.subscribe();
        Ok(stream::unfold(receiver, |mut receiver| async move {
            loop {
                receiver.changed().await.ok()?;
                let latest = receiver.borrow_and_update().clone();
                if let Some(picture) = latest {
                    return Some((PictureView(picture), receiver));
                }
            }
        }))
    }
}

/// One picture, shared by every view of its parts.
pub struct PictureView(Arc<Picture>);

#[Object(name = "VpnPicture")]
impl PictureView {
    /// When the hub last read its VPN status.
    async fn read_at(&self) -> DateTime<Utc> {
        self.0.read_at
    }

    /// False when the last read failed; the picture is then from readAt.
    async fn fresh(&self) -> bool {
        self.0.fresh
    }

    async fn hub(&self) -> NodeView {
        NodeView {
            picture: self.0.clone(),
            which: Which::Hub,
        }
    }

    /// Whether the hub can reach the broker that makes keys and approves routes (VP-7).
    async fn broker(&self) -> BrokerState {
        self.0.broker
    }

    /// One entry for each site, in site name order (VPN-009). A site deleted since the read
    /// is left out.
    async fn sites(&self, ctx: &Context<'_>) -> Result<Vec<SitePathView>> {
        let ids: Vec<_> = self.0.sites.iter().map(|site| site.site).collect();
        let mut loaded = schema_dependency::<DataLoader<SitesById>>(ctx)?
            .load_many(ids)
            .await
            .map_err(internal)?;
        Ok(self
            .0
            .sites
            .iter()
            .enumerate()
            .filter_map(|(index, path)| {
                loaded.remove(&path.site).map(|site| SitePathView {
                    picture: self.0.clone(),
                    index,
                    site,
                })
            })
            .collect())
    }

    /// Every peer of the hub, in name order. A node that is not a peer of the hub is not here
    /// (VPN-011).
    async fn nodes(&self) -> Vec<NodeView> {
        (0..self.0.nodes.len())
            .map(|index| NodeView {
                picture: self.0.clone(),
                which: Which::Peer(index),
            })
            .collect()
    }
}

/// How the hub reaches one site.
pub struct SitePathView {
    picture: Arc<Picture>,
    index: usize,
    site: Site,
}

impl SitePathView {
    fn entry(&self) -> &SitePath {
        &self.picture.sites[self.index]
    }

    fn router_node(&self) -> Option<&VpnNode> {
        self.entry().router.map(|index| &self.picture.nodes[index])
    }
}

#[Object(name = "SitePath")]
impl SitePathView {
    async fn site(&self) -> &Site {
        &self.site
    }

    /// The subnet router that serves the site subnet, or null.
    async fn router(&self) -> Option<NodeView> {
        self.entry().router.map(|index| NodeView {
            picture: self.picture.clone(),
            which: Which::Peer(index),
        })
    }

    /// A router of the site that offers the site subnet and waits for its approval, from the
    /// broker; null when none waits or the broker is not READY (BD-16).
    async fn waiting_router(&self) -> Option<NodeView> {
        self.entry().waiting_router.as_ref().map(|_| NodeView {
            picture: self.picture.clone(),
            which: Which::WaitingRouter(self.index),
        })
    }

    async fn state(&self) -> RouteState {
        self.entry().state
    }

    async fn path(&self) -> VpnPath {
        self.router_node()
            .map_or(VpnPath::Unknown, |node| node.path)
    }

    /// The DERP region, when the path is RELAYED.
    async fn relay_region(&self) -> Option<&str> {
        self.router_node()
            .and_then(|node| node.relay_region.as_deref())
    }

    /// True when the router's device has a Tailscale ping check, which keeps the path known.
    async fn has_ping_check(&self) -> bool {
        self.entry().has_ping_check
    }
}

/// Which node of a picture a view shows.
#[derive(Clone, Copy)]
enum Which {
    Hub,
    /// The peer at this index of `nodes`.
    Peer(usize),
    /// The waiting router of the site at this index of `sites`.
    WaitingRouter(usize),
}

/// One node of a picture.
pub struct NodeView {
    picture: Arc<Picture>,
    which: Which,
}

impl NodeView {
    fn node(&self) -> &VpnNode {
        match self.which {
            Which::Hub => &self.picture.hub,
            Which::Peer(index) => &self.picture.nodes[index],
            Which::WaitingRouter(site) => self.picture.sites[site]
                .waiting_router
                .as_ref()
                .expect("a waiting router view is made only for a site that has one"),
        }
    }
}

/// One node of the VPN. Name and OS come from the node and are cleaned (VPN-008).
#[Object(name = "VpnNode")]
impl NodeView {
    async fn name(&self) -> &str {
        &self.node().name
    }

    async fn address(&self) -> IpAddr {
        IpAddr(self.node().address.into())
    }

    async fn os(&self) -> &str {
        &self.node().os
    }

    async fn role(&self) -> Role {
        self.node().role
    }

    async fn tags(&self) -> &[String] {
        &self.node().tags
    }

    /// The subnet routes Headscale lets this node serve.
    #[expect(
        clippy::unused_async,
        clippy::unused_async_trait_impl,
        reason = "the Object macro requires async resolvers"
    )]
    async fn routes(&self) -> Result<Vec<Cidr>> {
        let routes = self.node().routes.iter().map(|route| Cidr::new(*route));
        routes.collect::<Result<_, _>>().map_err(internal)
    }

    /// The routes the node offers that wait for approval, from the broker (BD-16).
    #[expect(
        clippy::unused_async,
        clippy::unused_async_trait_impl,
        reason = "the Object macro requires async resolvers"
    )]
    async fn waiting_routes(&self) -> Result<Vec<Cidr>> {
        let routes = self
            .node()
            .waiting_routes
            .iter()
            .map(|route| Cidr::new(*route));
        routes.collect::<Result<_, _>>().map_err(internal)
    }

    async fn online(&self) -> bool {
        self.node().online
    }

    async fn last_seen(&self) -> Option<DateTime<Utc>> {
        self.node().last_seen
    }

    async fn key_expiry(&self) -> Option<DateTime<Utc>> {
        self.node().key_expiry
    }

    async fn key_state(&self) -> KeyState {
        self.node().key_state
    }

    async fn path(&self) -> VpnPath {
        self.node().path
    }

    async fn relay_region(&self) -> Option<&str> {
        self.node().relay_region.as_deref()
    }

    /// The Anima device on this node, or null.
    async fn device(&self, ctx: &Context<'_>) -> Result<Option<Device>> {
        let Some((id, _)) = self.node().device else {
            return Ok(None);
        };
        schema_dependency::<DataLoader<DevicesById>>(ctx)?
            .load_one(id)
            .await
            .map_err(internal)
    }

    async fn in_anima(&self) -> bool {
        self.node().in_anima
    }
}
