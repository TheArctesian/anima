//! The VPN mutations: keys, registration and renewal through the broker (R-055, VP-7, VP-10,
//! D-23). Each checks the token kind again behind the backstop, calls the broker, then writes
//! one audit entry and one event in its own transaction (`change`); if that write fails the
//! broker's change stays and the broker's log has it. Neither entry, no error and no log line
//! holds a pre-auth key or an auth ID (LOG-007).

use async_graphql::{Context, Error as GraphQlError, ID, Object, Result, dataloader::DataLoader};
use chrono::{DateTime, SecondsFormat, Utc};
use serde_json::{Value, json};

use super::change::broker_change;
use crate::{
    api_errors::{ErrorCode, FieldError, coded, internal, invalid, schema_dependency},
    fleet::{
        access::{Authenticated, operator_only},
        devices::{Site, SitesById},
        events::{AuditAction, AuditTarget, Severity, Source, TrailEntry},
        vpn::{
            Error, Feed, Refusal,
            broker::{Broker, BrokerView, NewKey, Registered},
            picture::{on_hub_networks, on_tailnet},
            steps, waiting,
        },
    },
    ids::SiteId,
    scalars::IpAddr,
};

const ON_TAILNET: &str =
    "This site is on the VPN's own address range. It is reached directly and has no router.";
const ON_HUB_NETWORKS: &str = "This site's subnet overlaps the hub's own networks \
     (172.30.0.0/16). It cannot have a router; give the site another subnet.";

/// A one-hour key, the commands that use it and when it stops working (VPN-001, VPN-003).
pub struct VpnKey {
    key: NewKey,
    steps: Vec<String>,
}

#[Object]
impl VpnKey {
    /// The pre-auth key: shown once, never stored by the hub (LOG-007).
    async fn key(&self) -> &str {
        self.key.key.expose()
    }

    async fn expires_at(&self) -> DateTime<Utc> {
        self.key.expires_at
    }

    /// The commands for the new computer, in order; none holds the key.
    async fn steps(&self) -> &[String] {
        &self.steps
    }
}

/// A node that the broker registered or renewed.
pub struct VpnRegistration(Registered);

#[Object]
impl VpnRegistration {
    async fn name(&self) -> String {
        crate::text::clean(&self.0.name, 63)
    }

    async fn address(&self) -> IpAddr {
        IpAddr(self.0.address.into())
    }

    async fn key_expiry(&self) -> Option<DateTime<Utc>> {
        self.0.key_expiry
    }
}

#[derive(Default)]
pub struct Mutation;

#[Object(name = "VpnMutation")]
impl Mutation {
    /// A one-use, one-hour key for a Linux computer of the operator (VPN-003). Login or browser
    /// token only (D-23).
    #[graphql(guard = "Authenticated")]
    async fn create_vpn_invite(&self, ctx: &Context<'_>) -> Result<VpnKey> {
        operator_only(ctx, "createVpnInvite")?;
        let broker = broker_of(ctx)?;
        let call = async move { broker.personal_key().await };
        let key = broker_change(ctx, call, None, |key| key_made(None, key)).await?;
        Ok(VpnKey {
            key,
            steps: steps::computer(),
        })
    }

    /// A one-use, one-hour key for the subnet router of a site (VPN-001). Login or browser token
    /// only (D-23).
    #[graphql(guard = "Authenticated")]
    async fn create_router_key(&self, ctx: &Context<'_>, site_id: ID) -> Result<VpnKey> {
        operator_only(ctx, "createRouterKey")?;
        let broker = broker_of(ctx)?;
        let site = router_site_of(ctx, &site_id).await?;
        let name = policy_site(&roster(ctx)?, &site)?;
        let steps = steps::router(&name, site.subnet);
        let call = async move { broker.router_key(&name).await };
        let entry = move |key: &NewKey| key_made(Some(&site), key);
        let key = broker_change(ctx, call, Some("siteId"), entry).await?;
        Ok(VpnKey { key, steps })
    }

    /// Registers the computer that shows this auth ID on its own screen (vpn.md 6B). Login or
    /// browser token only (D-23).
    #[graphql(guard = "Authenticated")]
    async fn register_vpn_node(
        &self,
        ctx: &Context<'_>,
        auth_id: String,
    ) -> Result<VpnRegistration> {
        operator_only(ctx, "registerVpnNode")?;
        let broker = broker_of(ctx)?;
        if !is_auth_id(&auth_id) {
            return Err(invalid(&[FieldError::new("authId", AUTH_ID_FORM)]));
        }
        let call = async move { broker.register(&auth_id).await };
        let entry = |node: &Registered| node_changed(AuditAction::Accept, node, None);
        let node = broker_change(ctx, call, Some("authId"), entry).await?;
        Ok(VpnRegistration(node))
    }

    /// Sets the key expiry of an untagged operator node to a staggered date 90 to 180 days from
    /// now (vpn.md Section 7). Login or browser token only (D-23).
    #[graphql(guard = "Authenticated")]
    async fn renew_vpn_key(&self, ctx: &Context<'_>, name: String) -> Result<VpnRegistration> {
        operator_only(ctx, "renewVpnKey")?;
        let broker = broker_of(ctx)?;
        let id = operator_node(&roster(ctx)?, &name)?;
        let call = async move { broker.renew(id).await };
        let entry = move |node: &Registered| node_changed(AuditAction::Update, node, Some(id));
        let node = broker_change(ctx, call, Some("name"), entry).await?;
        Ok(VpnRegistration(node))
    }
}

const AUTH_ID_FORM: &str =
    "Type the ID as the new computer shows it: hskey-authreq- and 24 letters, digits, - or _.";
/// What every Headscale 0.29.7 auth ID starts with (vpn-build.md Section 3, fact 11).
const AUTH_ID_PREFIX: &str = "hskey-authreq-";

/// Whether `text` is an auth ID as the broker checks it: `^hskey-authreq-[A-Za-z0-9_-]{24}$`.
fn is_auth_id(text: &str) -> bool {
    text.strip_prefix(AUTH_ID_PREFIX).is_some_and(|rest| {
        rest.len() == 24
            && rest
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
    })
}

/// The hub's broker, sharing its turn; `UNAVAILABLE` when `ANIMA_VPN_BROKER` is not set.
pub(super) fn broker_of(ctx: &Context<'_>) -> Result<Broker> {
    schema_dependency::<Option<Broker>>(ctx)?
        .clone()
        .ok_or_else(|| {
            coded(
                "Enrolment needs the broker (ANIMA_VPN_BROKER).",
                ErrorCode::Unavailable,
            )
        })
}

/// The broker's node list of the latest read; `UNAVAILABLE` before the broker answered once.
pub(super) fn roster(ctx: &Context<'_>) -> Result<BrokerView> {
    schema_dependency::<Feed>(ctx)?
        .latest()
        .and_then(|picture| picture.roster.clone())
        .ok_or_else(|| {
            coded(
                "The hub has not read the broker's node list yet. Try again in a minute.",
                ErrorCode::Unavailable,
            )
        })
}

/// The site an id argument names; `VALIDATION` for a bad id, `NOT_FOUND` for no site.
pub(super) async fn site_of(ctx: &Context<'_>, id: &ID) -> Result<Site> {
    let id: SiteId = id
        .parse()
        .map_err(|_| invalid(&[FieldError::new("siteId", "That id is not valid.")]))?;
    schema_dependency::<DataLoader<SitesById>>(ctx)?
        .load_one(id)
        .await
        .map_err(internal)?
        .ok_or_else(|| coded("No such site.", ErrorCode::NotFound))
}

/// The site an id argument names, when it has a subnet router: `VALIDATION` on `siteId` for a
/// site on the tailnet's own range, which is reached directly over the VPN, and for one over the
/// hub's own networks, whose router could pose as the hub's containers (VP-13).
pub(super) async fn router_site_of(ctx: &Context<'_>, id: &ID) -> Result<Site> {
    let site = site_of(ctx, id).await?;
    if on_tailnet(site.subnet) {
        return Err(invalid(&[FieldError::new("siteId", ON_TAILNET)]));
    }
    if on_hub_networks(site.subnet) {
        return Err(invalid(&[FieldError::new("siteId", ON_HUB_NETWORKS)]));
    }
    Ok(site)
}

/// The VPN policy's name for `site`, by its subnet.
pub(super) fn policy_site(roster: &BrokerView, site: &Site) -> Result<String> {
    waiting::policy_site(roster, site.subnet)
        .map(str::to_owned)
        .ok_or_else(|| {
            invalid(&[FieldError::new(
                "siteId",
                "This site's subnet is not in the VPN policy.",
            )])
        })
}

/// The broker id of the untagged `operator` node called `name`.
fn operator_node(roster: &BrokerView, name: &str) -> Result<u64> {
    roster
        .nodes
        .iter()
        .find(|node| node.name == name && node.user == "operator" && node.tags.is_empty())
        .map(|node| node.id)
        .ok_or_else(|| {
            invalid(&[FieldError::new(
                "name",
                "No untagged computer of the operator has this name.",
            )])
        })
}

/// The K7 error of a failed broker call. A refusal about the input is `VALIDATION` on `field`;
/// with no input field it is the hub's own mistake, so `INTERNAL`.
pub(super) fn broker_error(error: Error, field: Option<&'static str>) -> GraphQlError {
    let message = match &error {
        Error::BrokerRefused { refusal, detail } => match refusal {
            Refusal::RateLimited => return coded(RATE_LIMITED, ErrorCode::Conflict),
            Refusal::NodeLimit => return coded(NODE_LIMIT, ErrorCode::Conflict),
            Refusal::HeadscaleFailed => return unavailable(&error, HEADSCALE_FAILED),
            Refusal::BadRequest => "The broker refused the request.".to_owned(),
            Refusal::UnknownSite => "This site or its tag is not in the VPN policy.".to_owned(),
            // The broker says why: a subnet another router already serves, an auth ID of a
            // tagged node, a node that is not the operator's.
            Refusal::NotAllowed => format!("The broker does not allow this: {detail}."),
            Refusal::Undone => format!("The broker undid the change: {detail}."),
        },
        Error::NotInstalled => return unavailable(&error, NOT_INSTALLED),
        Error::BrokerUnreadable | Error::BrokerTimedOut | Error::BrokerConnection => {
            return unavailable(&error, NO_ANSWER);
        }
        Error::BrokerBusy => return unavailable(&error, BUSY),
        _ => return internal(error),
    };
    match field {
        Some(field) => invalid(&[FieldError::new(field, message)]),
        None => internal(error),
    }
}

const RATE_LIMITED: &str =
    "The broker allows 3 keys and 3 registrations in 24 hours. Try again later.";
const NODE_LIMIT: &str = "The VPN is at its node limit (max_nodes in /etc/anima/broker.conf).";
const HEADSCALE_FAILED: &str = "Headscale or a broker file failed on the VPS. Try again.";
const NOT_INSTALLED: &str = "The broker is not installed on the VPS (vpn.md Section 11).";
/// Also a closed connection with no reply: the broker drops a request that waits too long
/// behind its node audit.
const NO_ANSWER: &str =
    "The broker did not answer; it may be busy checking its nodes. Try again in a minute.";
/// The hub's own earlier call still had the broker; this one was never sent.
const BUSY: &str = "The broker is busy with another request. Try again in a minute.";

/// `UNAVAILABLE`, with the fault word in the process log (never a reply, LOG-007).
fn unavailable(error: &Error, message: &str) -> GraphQlError {
    tracing::warn!(fault = error.fault(), "VPN broker call failed");
    coded(message, ErrorCode::Unavailable)
}

/// The audit entry and event of a new key: its site and expiry, never the key (LOG-007).
fn key_made(site: Option<&Site>, key: &NewKey) -> TrailEntry {
    let expires_at = key.expires_at.to_rfc3339_opts(SecondsFormat::Secs, true);
    let message = match site {
        Some(site) => format!("Made a one-hour VPN key for the router of {}.", site.name),
        None => "Made a one-hour VPN key for a Linux computer.".to_owned(),
    };
    entry(
        AuditAction::Create,
        message,
        json!({ "site": site.map(|site| &site.name), "expiresAt": expires_at }),
    )
}

/// The audit entry and event of a registered or renewed node, with the broker id the hub sent
/// for a renewal.
fn node_changed(action: AuditAction, node: &Registered, sent: Option<u64>) -> TrailEntry {
    let name = crate::text::clean(&node.name, 63);
    let message = match action {
        AuditAction::Accept => format!("Registered {name} on the VPN."),
        _ => format!("Renewed the VPN key of {name}."),
    };
    let expiry = node
        .key_expiry
        .map(|at| at.to_rfc3339_opts(SecondsFormat::Secs, true));
    let mut change =
        json!({ "node": name, "address": node.address.to_string(), "keyExpiry": expiry });
    if let Some(id) = sent {
        change["brokerNode"] = id.into();
    }
    entry(action, message, change)
}

pub(super) fn entry(action: AuditAction, message: String, change: Value) -> TrailEntry {
    TrailEntry {
        source: Source::Vpn,
        severity: Severity::Info,
        device: None,
        message,
        action,
        target: AuditTarget::Vpn,
        change,
    }
}
