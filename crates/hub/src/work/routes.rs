//! The public names of applications, served on the hub (HOST-046, R-NEW-hubrouting,
//! R-NEW-buildnow; task H21; Section 12 of `docs/proposals/hosted-applications-stage-2.md`).
//!
//! A route is one name of one application with a target that is fixed when the name is added:
//! the device, the port that the applications program recorded and the address resolved then
//! (SF-NEW-routetarget). A route serves only when its name is proven (Section 12.4), the second
//! consent at the machine is given (OD-HOST-43; nothing can give it yet), the hub's record of the
//! application confirms the device and the port and names no socket of the engine (task H6; the
//! hub has no such record yet, SF-NEW-revroutetarget) and it has a valid certificate
//! (Section 12.5). The proxy is a process apart from the hub; it reaches an
//! application only through [`door`], which takes the identifier of an application and never a
//! device or a port (SF-NEW-routeproxyexit). Messages in [`Error`] hold no secret (LOG-007).

pub mod caa;
pub mod certificates;
mod dns;
pub mod door;
pub mod graphql;
mod model;
pub mod names;
pub mod proof;
mod store;
mod target;
#[cfg(test)]
mod tests;

use std::net::SocketAddr;

use chrono::{DateTime, Utc};
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

pub use self::dns::SystemDns;
pub use self::model::{Added, CertificateState, NewRoute, Route, Traffic, Waits};
use self::{
    certificates::Issuer,
    names::{NameRefusal, NameRules, RouteName},
    proof::{ProofRule, Resolver},
    target::TargetRefusal,
};
use crate::{
    config::Config,
    fleet::{
        credentials::{self, Secret, Vault},
        devices,
        targets::{self, AddressPolicy},
    },
    ids::{ApplicationId, DeviceId, RouteId},
};

/// Why a route operation failed.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("no such route")]
    NotFound,
    #[error("no such device")]
    NoDevice,
    #[error(transparent)]
    Name(#[from] NameRefusal),
    #[error(transparent)]
    Target(#[from] TargetRefusal),
    #[error("a route already has this name")]
    NameTaken,
    #[error("the application already has routes to a different device or port")]
    OtherTarget,
    #[error("the host of the device does not resolve to an address the hub may reach")]
    Unresolved,
    #[error("the route does not serve now")]
    NotServing(Waits),
    #[error(transparent)]
    Caa(#[from] caa::CaaRefusal),
    #[error(transparent)]
    Issue(#[from] certificates::IssueError),
    #[error("no random bytes for a token")]
    Random(#[source] getrandom::Error),
    #[error("the key of a certificate cannot be sealed or opened")]
    Seal(#[source] credentials::Error),
    #[error("a stored route cannot be read")]
    Unreadable,
    #[error("device store error")]
    Devices(#[source] devices::Error),
    #[error("database error")]
    Database(#[from] sqlx::Error),
}

/// The settings of the routes on this hub.
#[derive(Clone, Debug)]
pub struct Settings {
    pub names: NameRules,
    pub proof: ProofRule,
    /// The address rule of each connection of the hub.
    pub policy: AddressPolicy,
    /// The listeners of the hub itself, which a route never targets.
    pub hub_listeners: Vec<SocketAddr>,
}

impl Settings {
    /// The settings of this hub: the name of the hub from its public origin; the proof by
    /// address on a hub with `ANIMA_ROUTES_PUBLIC_ADDRESS`, by token otherwise.
    #[must_use]
    pub fn new(config: &Config, policy: AddressPolicy) -> Self {
        let hub = hub_name(&config.public_origin);
        let proof = config
            .routes
            .public_address
            .map_or_else(|| ProofRule::Token { hub: hub.clone() }, ProofRule::Address);
        Self {
            names: NameRules {
                hub,
                cookie_domains: config.routes.cookie_domains.clone(),
            },
            proof,
            policy,
            hub_listeners: vec![config.listen],
        }
    }
}

/// The host of an origin such as `https://hub.anima.ing:8443`, in lower case.
fn hub_name(origin: &str) -> String {
    let rest = origin.split_once("://").map_or(origin, |(_, rest)| rest);
    let host = rest.split(['/', ':']).next().unwrap_or_default();
    host.to_ascii_lowercase()
}

/// The routes of one application, or of each, by name.
///
/// # Errors
/// `Database` or `Unreadable`.
pub async fn list(db: &PgPool, application: Option<ApplicationId>) -> Result<Vec<Route>, Error> {
    store::list(db, application).await
}

/// One route.
///
/// # Errors
/// `NotFound`, `Database` or `Unreadable`.
pub async fn get(db: &PgPool, id: RouteId) -> Result<Route, Error> {
    store::get(db, id).await?.ok_or(Error::NotFound)
}

/// Adds a name to an application, with a new token and its target fixed now. The caller has
/// checked who may do this (SF-NEW-routepublish).
///
/// # Errors
/// The [`Error`] of the first rule that refuses.
pub async fn add(
    (db, settings): (&PgPool, &Settings),
    conn: &mut PgConnection,
    new: &NewRoute,
    now: DateTime<Utc>,
) -> Result<Added, Error> {
    let name = RouteName::parse(&new.name, &settings.names)?;
    let port = target::check_port(new.port)?;
    let device = devices::get(db, new.device)
        .await
        .map_err(|error| match error {
            devices::Error::NotFound(_) => Error::NoDevice,
            other => Error::Devices(other),
        })?;
    let address = fixed_address(&device.host, port, settings).await?;
    let others = store::list(&mut *conn, Some(new.application)).await?;
    if others
        .iter()
        .any(|other| (other.device, other.port) != (new.device, port))
    {
        return Err(Error::OtherTarget);
    }
    let route = Route {
        id: RouteId::from(Uuid::now_v7()),
        name,
        application: new.application,
        device: new.device,
        port,
        address: Some(address),
        token: proof::new_token().map_err(Error::Random)?,
        proof: proof::Proof::Waiting,
        failed_reads: 0,
        machine_consent: false,
        application_recorded: false,
        checked_at: None,
        certificate: None,
        traffic: Traffic::default(),
        created_at: now,
    };
    store::insert(conn, &route).await?;
    let same_site_as_hub = route.name.same_site_as(&settings.names.hub);
    Ok(Added {
        route,
        same_site_as_hub,
    })
}

/// The one address of the target, resolved once now and checked (SF-NEW-routetarget).
async fn fixed_address(host: &str, port: u16, settings: &Settings) -> Result<SocketAddr, Error> {
    let found = targets::resolve_checked(host, port, &settings.policy)
        .await
        .map_err(|_| Error::Unresolved)?;
    let first = *found.first().ok_or(Error::Unresolved)?;
    Ok(target::check_address(
        first,
        &settings.policy,
        &settings.hub_listeners,
    )?)
}

/// Reads the proof of a route once and keeps its new state. One failed read changes nothing;
/// a proven name is lost only after several in a row (SF-NEW-routetoken).
///
/// # Errors
/// `NotFound`, `Database` or `Unreadable`.
pub async fn check(
    conn: &mut PgConnection,
    (resolver, settings): (&dyn Resolver, &Settings),
    id: RouteId,
    now: DateTime<Utc>,
) -> Result<Route, Error> {
    let mut route = store::get(&mut *conn, id).await?.ok_or(Error::NotFound)?;
    let read = proof::read(resolver, &settings.proof, &route.name, &route.token).await;
    let (state, failed) = proof::after(route.proof, route.failed_reads, read);
    store::set_proof(conn, id, (state, failed), now).await?;
    route.proof = state;
    route.failed_reads = failed;
    route.checked_at = Some(now);
    Ok(route)
}

/// Removes a route with its certificate and its key; the device and the name it had.
///
/// # Errors
/// `NotFound` or `Database`.
pub async fn remove(conn: &mut PgConnection, id: RouteId) -> Result<(DeviceId, String), Error> {
    store::delete(conn, id).await?.ok_or(Error::NotFound)
}

/// What a certificate order needs besides the route.
#[derive(Clone, Copy)]
pub struct Ordering<'a> {
    pub db: &'a PgPool,
    pub vault: &'a Vault,
    pub resolver: &'a dyn Resolver,
    pub issuer: &'a dyn Issuer,
}

/// Asks for a certificate of a proven route when it has none or one that ends soon, after the
/// CAA check (Section 12.5). Returns whether a certificate was stored. A failed order makes the
/// next one wait longer.
///
/// # Errors
/// `NotServing(Proof)` before the proof; the CAA refusal; the issuer's error; `Seal`.
pub async fn order_certificate(
    ordering: Ordering<'_>,
    id: RouteId,
    now: DateTime<Utc>,
) -> Result<bool, Error> {
    let route = get(ordering.db, id).await?;
    if route.proof != proof::Proof::Proven {
        return Err(Error::NotServing(Waits::Proof));
    }
    let (failures, next) = store::issue_state(ordering.db, id).await?;
    let not_after = route.certificate.map(|certificate| certificate.not_after);
    if !certificates::needs_order(not_after, next, now) {
        return Ok(false);
    }
    caa::check(ordering.resolver, &route.name, certificates::AUTHORITY).await?;
    let issued = match ordering.issuer.issue(&route.name).await {
        Ok(issued) => issued,
        Err(error) => {
            let failures = failures.saturating_add(1);
            let next = certificates::next_attempt(failures, now);
            store::issue_failed(ordering.db, id, failures, next).await?;
            return Err(error.into());
        }
    };
    let context = certificates::key_context(id, &route.name);
    let sealed = ordering
        .vault
        .seal(&issued.key_pem, context.as_bytes())
        .map_err(Error::Seal)?;
    let stored = store::StoredCertificate {
        chain_pem: issued.chain_pem,
        sealed_key: sealed.to_bytes(),
        not_after: issued.not_after,
    };
    let mut conn = ordering.db.acquire().await?;
    store::store_certificate(&mut conn, id, &stored, now).await?;
    Ok(true)
}

/// The certificate of a serving route with its key opened, for the proxy only (through
/// [`door`]). No `Debug`: it holds the key.
pub struct Served {
    pub name: RouteName,
    pub application: ApplicationId,
    pub chain_pem: String,
    pub key_pem: Secret,
}

/// Each route that serves at `now`, with its certificate and key: the one view of the routes
/// that the proxy gets (Section 12.3).
///
/// # Errors
/// `Database`, `Unreadable` or `Seal`.
pub async fn served(db: &PgPool, vault: &Vault, now: DateTime<Utc>) -> Result<Vec<Served>, Error> {
    let mut found = Vec::new();
    for route in store::list(db, None).await? {
        if route.waits(now).is_some() {
            continue;
        }
        let Some(stored) = store::certificate(db, route.id).await? else {
            continue;
        };
        let sealed = credentials::Sealed::from_bytes(&stored.sealed_key).map_err(Error::Seal)?;
        let context = certificates::key_context(route.id, &route.name);
        let key_pem = vault
            .open(&sealed, context.as_bytes())
            .map_err(Error::Seal)?;
        found.push(Served {
            name: route.name,
            application: route.application,
            chain_pem: stored.chain_pem,
            key_pem,
        });
    }
    Ok(found)
}

/// The fixed target of the application, from the route that serves for it at `now`: the one
/// way from an application identifier to a device and a port (SF-NEW-routeproxyexit).
///
/// # Errors
/// `NotFound` when no route of the application serves; `Database` or `Unreadable`.
pub async fn target_of(
    db: &PgPool,
    application: ApplicationId,
    now: DateTime<Utc>,
) -> Result<(DeviceId, SocketAddr), Error> {
    store::list(db, Some(application))
        .await?
        .into_iter()
        .filter(|route| route.waits(now).is_none())
        .find_map(|route| route.address.map(|address| (route.device, address)))
        .ok_or(Error::NotFound)
}

/// Adds what the proxy counted for the route of `name`. An unknown name is no error: the route
/// can be removed while the proxy counts.
///
/// # Errors
/// `Database`.
pub async fn count(
    db: &PgPool,
    name: &str,
    traffic: Traffic,
    now: DateTime<Utc>,
) -> Result<(), Error> {
    if let Some(route) = store::list(db, None)
        .await?
        .into_iter()
        .find(|route| route.name.as_str() == name)
    {
        store::add_traffic(db, route.id, traffic, now).await?;
    }
    Ok(())
}
