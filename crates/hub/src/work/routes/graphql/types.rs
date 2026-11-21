//! The K1 types of the routes: `ApplicationRoute`, `ApplicationRouteAdded` and
//! `ApplicationRouteInput`. No field holds a certificate or a key.
#![expect(
    clippy::unused_async_trait_impl,
    reason = "the async-graphql Enum derive generates async trait impls"
)]

use async_graphql::{Enum, ID, InputObject, Object};
use chrono::{DateTime, Utc};

use crate::work::routes::{
    Added, Route, Waits,
    proof::{Proof, ProofRule, record_name, record_value},
};

#[derive(Clone, Copy, PartialEq, Eq, Enum)]
#[graphql(name = "ApplicationRouteProof")]
pub(super) enum ProofValue {
    Waiting,
    Proven,
    Lost,
}

#[derive(Clone, Copy, PartialEq, Eq, Enum)]
#[graphql(name = "ApplicationRouteWait")]
pub(super) enum WaitValue {
    Proof,
    MachineConsent,
    ApplicationRecord,
    Certificate,
}

/// A route with what its fields need: the rule of the proof and the time of the read.
pub(super) struct RouteView {
    pub route: Route,
    pub rule: ProofRule,
    pub now: DateTime<Utc>,
}

/// Counts as K1 `Float`: exact up to 2^53.
#[expect(
    clippy::cast_precision_loss,
    reason = "K1 gives counts as Float; a count above 2^53 loses only its last digits"
)]
const fn float(count: u64) -> f64 {
    count as f64
}

#[Object(name = "ApplicationRoute")]
impl RouteView {
    async fn id(&self) -> ID {
        ID(self.route.id.to_string())
    }

    async fn name(&self) -> &str {
        self.route.name.as_str()
    }

    async fn application_id(&self) -> ID {
        ID(self.route.application.to_string())
    }

    async fn device_id(&self) -> ID {
        ID(self.route.device.to_string())
    }

    async fn port(&self) -> i32 {
        i32::from(self.route.port)
    }

    async fn proof(&self) -> ProofValue {
        match self.route.proof {
            Proof::Waiting => ProofValue::Waiting,
            Proof::Proven => ProofValue::Proven,
            Proof::Lost => ProofValue::Lost,
        }
    }

    async fn failed_reads(&self) -> i32 {
        i32::try_from(self.route.failed_reads).unwrap_or(i32::MAX)
    }

    async fn proof_record_name(&self) -> Option<String> {
        matches!(self.rule, ProofRule::Token { .. }).then(|| record_name(&self.route.name))
    }

    async fn proof_record_value(&self) -> Option<String> {
        match &self.rule {
            ProofRule::Token { hub } => Some(record_value(hub, &self.route.token)),
            ProofRule::Address(_) => None,
        }
    }

    async fn machine_consent(&self) -> bool {
        self.route.machine_consent
    }

    async fn certificate_not_after(&self) -> Option<DateTime<Utc>> {
        self.route
            .certificate
            .map(|certificate| certificate.not_after)
    }

    async fn waits_for(&self) -> Option<WaitValue> {
        self.route.waits(self.now).map(|waits| match waits {
            Waits::Proof => WaitValue::Proof,
            Waits::MachineConsent => WaitValue::MachineConsent,
            Waits::ApplicationRecord => WaitValue::ApplicationRecord,
            Waits::Certificate => WaitValue::Certificate,
        })
    }

    async fn checked_at(&self) -> Option<DateTime<Utc>> {
        self.route.checked_at
    }

    async fn requests(&self) -> f64 {
        float(self.route.traffic.requests)
    }

    async fn bytes_in(&self) -> f64 {
        float(self.route.traffic.bytes_in)
    }

    async fn bytes_out(&self) -> f64 {
        float(self.route.traffic.bytes_out)
    }

    async fn refused(&self) -> f64 {
        float(self.route.traffic.refused)
    }

    async fn created_at(&self) -> DateTime<Utc> {
        self.route.created_at
    }
}

/// K1 `ApplicationRouteAdded`.
pub(super) struct AddedView {
    pub added: Added,
    pub rule: ProofRule,
    pub now: DateTime<Utc>,
}

#[Object(name = "ApplicationRouteAdded")]
impl AddedView {
    async fn route(&self) -> RouteView {
        RouteView {
            route: self.added.route.clone(),
            rule: self.rule.clone(),
            now: self.now,
        }
    }

    async fn same_site_as_hub(&self) -> bool {
        self.added.same_site_as_hub
    }
}

/// K1 `ApplicationRouteInput`.
#[derive(InputObject)]
#[graphql(name = "ApplicationRouteInput")]
pub(super) struct RouteFields {
    pub name: String,
    pub application_id: ID,
    pub device_id: ID,
    pub port: i32,
}
