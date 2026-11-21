//! The types of the routes domain.

use std::net::SocketAddr;

use chrono::{DateTime, Utc};

use super::{names::RouteName, proof::Proof};
use crate::ids::{ApplicationId, DeviceId, RouteId};

/// One public name of one application, with its fixed target (SF-NEW-routetarget).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Route {
    pub id: RouteId,
    pub name: RouteName,
    pub application: ApplicationId,
    pub device: DeviceId,
    pub port: u16,
    /// The address fixed when the name was added; a link address for a computer of the link.
    pub address: Option<SocketAddr>,
    pub token: String,
    pub proof: Proof,
    pub failed_reads: u32,
    /// The second consent at the machine (OD-HOST-43). Nothing sets it yet.
    pub machine_consent: bool,
    /// The hub's record of the application names this device and this port, and no socket of
    /// the engine (SF-NEW-revroutetarget). The hub keeps no such record before task H6, so
    /// nothing sets it yet.
    pub application_recorded: bool,
    pub checked_at: Option<DateTime<Utc>>,
    pub certificate: Option<CertificateState>,
    pub traffic: Traffic,
    pub created_at: DateTime<Utc>,
}

/// What the hub knows of the certificate of a route. Never the key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CertificateState {
    pub not_after: DateTime<Utc>,
    pub stored_at: DateTime<Utc>,
}

/// What the proxy counted for a route.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Traffic {
    pub requests: u64,
    pub bytes_in: u64,
    pub bytes_out: u64,
    /// Connections that a limit of the route refused.
    pub refused: u64,
}

/// Why a route does not serve now, in the order the hub checks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Waits {
    /// The name is not proven, or was lost.
    Proof,
    /// The second consent at the machine is not given (OD-HOST-43).
    MachineConsent,
    /// The hub has no record of the application with this device and port and without the
    /// socket of the engine (task H6; SF-NEW-revroutetarget).
    ApplicationRecord,
    /// The route has no certificate that is valid now.
    Certificate,
}

impl Route {
    /// What the route waits for at `now`; `None` when it serves.
    #[must_use]
    pub fn waits(&self, now: DateTime<Utc>) -> Option<Waits> {
        if self.proof != Proof::Proven {
            return Some(Waits::Proof);
        }
        if !self.machine_consent {
            return Some(Waits::MachineConsent);
        }
        if !self.application_recorded {
            return Some(Waits::ApplicationRecord);
        }
        match self.certificate {
            Some(certificate) if certificate.not_after > now => None,
            _ => Some(Waits::Certificate),
        }
    }
}

/// What a person gives to add a name. The device and the port are what the person says; the
/// route serves only when the hub's record of the application confirms them and says that it
/// holds no socket of the engine ([`Route::application_recorded`], SF-NEW-revroutetarget).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewRoute {
    pub name: String,
    pub application: ApplicationId,
    pub device: DeviceId,
    pub port: i64,
}

/// A route that was added, with what the person must do and what the hub warns about.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Added {
    pub route: Route,
    /// The name shares its domain with the hub (SF-NEW-routeabuse).
    pub same_site_as_hub: bool,
}
