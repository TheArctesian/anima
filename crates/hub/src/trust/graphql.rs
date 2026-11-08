//! The trust GraphQL part: the root `Mutation` with `acceptHostKey` and
//! `acceptServerCertificate` (K1; D-6, SEC-006, SEC-007), and the loaders behind
//! `SshSettings.hostKey` and `RdpSettings.serverCertificate` (API-005).
//!
//! The types live in the sub-modules; each `#[Object]` block there is the single resolver block
//! for its type.

mod error;
mod host_key;
mod server_certificate;

use std::sync::Arc;

use async_graphql::{Context, ID, Object, Result, dataloader::DataLoader};
use sqlx::PgPool;

pub use self::{host_key::HostKeysByDevice, server_certificate::ServerCertificatesByDevice};

use self::error::to_graphql;
use super::{
    CertificateFingerprint, Fingerprint, HostKey, ServerCertificate, Sightings,
    accept::{Acceptance, accept_host_key},
    accept_certificate::{CertificateAcceptance, accept_server_certificate},
    model::Actor,
};
use crate::{
    access::Authenticated,
    api_errors::{FieldError, invalid, schema_dependency},
    clock::Clock,
    events::{ClientAddr, Operator},
    ids::DeviceId,
};

/// The loader behind `SshSettings.hostKey`: one query per batch of device ids, no cache, so
/// a key accepted shows in the next request (API-005, R-014).
#[must_use]
pub fn host_key_loader(pool: PgPool) -> DataLoader<HostKeysByDevice> {
    DataLoader::new(HostKeysByDevice(pool), tokio::spawn)
}

/// The loader behind `RdpSettings.serverCertificate`: one query per batch of device ids, no
/// cache, so a certificate accepted shows in the next request (API-005, SEC-007).
#[must_use]
pub fn server_certificate_loader(pool: PgPool) -> DataLoader<ServerCertificatesByDevice> {
    DataLoader::new(ServerCertificatesByDevice(pool), tokio::spawn)
}

/// Who acts, from where and when, as the schema data of a signed-in request gives it.
fn actor(ctx: &Context<'_>) -> Result<Actor> {
    Ok(Actor {
        operator: schema_dependency::<Operator>(ctx)?.clone(),
        client: *schema_dependency::<ClientAddr>(ctx)?,
        at: schema_dependency::<Arc<dyn Clock>>(ctx)?.now(),
    })
}

/// `acceptHostKey`'s arguments, reporting both bad ones at once.
fn acceptance(device_id: &ID, fingerprint: &str) -> Result<Acceptance, Vec<FieldError>> {
    let device = device_id
        .parse::<DeviceId>()
        .map_err(|_| FieldError::new("deviceId", "That device id is not valid."));
    let fingerprint = fingerprint.parse::<Fingerprint>().map_err(|_| {
        FieldError::new(
            "fingerprintSha256",
            "Enter the fingerprint as the connection test shows it: SHA256: and 43 characters.",
        )
    });
    match (device, fingerprint) {
        (Ok(device), Ok(fingerprint)) => Ok(Acceptance {
            device,
            fingerprint,
        }),
        (device, fingerprint) => Err(device.err().into_iter().chain(fingerprint.err()).collect()),
    }
}

/// `acceptServerCertificate`'s arguments, reporting both bad ones at once.
fn certificate_acceptance(
    device_id: &ID,
    fingerprint: &str,
) -> Result<CertificateAcceptance, Vec<FieldError>> {
    let device = device_id
        .parse::<DeviceId>()
        .map_err(|_| FieldError::new("deviceId", "That device id is not valid."));
    let fingerprint = fingerprint.parse::<CertificateFingerprint>().map_err(|_| {
        FieldError::new(
            "fingerprintSha256",
            "Enter the fingerprint as the connection test shows it: 32 pairs of hex digits \
             joined by colons.",
        )
    });
    match (device, fingerprint) {
        (Ok(device), Ok(fingerprint)) => Ok(CertificateAcceptance {
            device,
            fingerprint,
        }),
        (device, fingerprint) => Err(device.err().into_iter().chain(fingerprint.err()).collect()),
    }
}

#[derive(Default)]
pub struct Mutation;

/// Merged into the root `Mutation`; the name only keeps it apart from the other parts.
#[Object(name = "TrustMutation")]
impl Mutation {
    /// Accepts the host key the last connection test saw for the device, when the
    /// fingerprint matches; else `CONFLICT` (D-6, SEC-006).
    #[graphql(guard = "Authenticated")]
    async fn accept_host_key(
        &self,
        ctx: &Context<'_>,
        device_id: ID,
        fingerprint_sha256: String,
    ) -> Result<HostKey> {
        let acceptance =
            acceptance(&device_id, &fingerprint_sha256).map_err(|fields| invalid(&fields))?;
        accept_host_key(
            schema_dependency::<PgPool>(ctx)?,
            schema_dependency::<Sightings>(ctx)?,
            &actor(ctx)?,
            acceptance,
        )
        .await
        .map_err(to_graphql)
    }

    /// Accepts the RDP server certificate the last connection test saw for the device, when
    /// the fingerprint matches; else `CONFLICT` (SEC-007).
    #[graphql(guard = "Authenticated")]
    async fn accept_server_certificate(
        &self,
        ctx: &Context<'_>,
        device_id: ID,
        fingerprint_sha256: String,
    ) -> Result<ServerCertificate> {
        let acceptance = certificate_acceptance(&device_id, &fingerprint_sha256)
            .map_err(|fields| invalid(&fields))?;
        accept_server_certificate(
            schema_dependency::<PgPool>(ctx)?,
            schema_dependency::<Sightings>(ctx)?,
            &actor(ctx)?,
            acceptance,
        )
        .await
        .map_err(to_graphql)
    }
}
