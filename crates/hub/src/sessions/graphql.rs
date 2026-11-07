//! The sessions GraphQL part: root `Query` and `Mutation` (K1; RA-008, API-003, API-006,
//! API-009).
//!
//! The types live in the sub-modules; each `#[Object]` block there is the single resolver block
//! for its type.

mod connection;
pub(super) mod connection_test;
pub(super) mod error;
mod host_key;
mod server_certificate;
mod session;
mod ticket;

use std::sync::Arc;

use async_graphql::{Context, ID, Object, Result, dataloader::DataLoader};
use sqlx::PgPool;

/// For the files part's resolver (WP-27 D-8): the same errors and schema data.
pub(super) use self::error::{invalid, schema_dependency, to_graphql};
pub use self::{host_key::HostKeysByDevice, server_certificate::ServerCertificatesByDevice};

use self::{
    connection::{DEFAULT_FIRST, SessionConnection, sessions_args},
    connection_test::ConnectionTestInput,
    ticket::StartSessionInput,
};
use super::{
    Actor, FieldError, Fingerprint, HostKey, Live, ServerCertificate, Session, Ticket,
    end::end_session,
    start, store,
    trust::{
        Acceptance, CertificateAcceptance, CertificateFingerprint, ConnectionTest, TestRequest,
        accept_host_key, accept_server_certificate, test_connection,
    },
};
use crate::{
    access::Authenticated,
    clock::Clock,
    credentials::Vault,
    events::{ClientAddr, Operator},
    ids::{DeviceId, SessionId},
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
pub struct Query;

/// Merged into the root `Query`; the name only keeps it apart from the other parts.
#[Object(name = "SessionsQuery")]
impl Query {
    /// Sessions newest first, live and ended (RA-008).
    #[graphql(guard = "Authenticated")]
    #[expect(
        clippy::too_many_arguments,
        reason = "K1 fixes the argument list of Query.sessions"
    )]
    async fn sessions(
        &self,
        ctx: &Context<'_>,
        device_id: Option<ID>,
        live: Option<bool>,
        #[graphql(default_with = "Some(DEFAULT_FIRST)")] first: Option<i32>,
        after: Option<String>,
    ) -> Result<SessionConnection> {
        let args = sessions_args(
            device_id.as_ref(),
            live,
            first.unwrap_or(DEFAULT_FIRST),
            after.as_deref(),
        )
        .map_err(|fields| invalid(&fields))?;
        let page = store::page(
            schema_dependency::<PgPool>(ctx)?,
            args.filter,
            args.size,
            args.after,
        )
        .await
        .map_err(to_graphql)?;
        Ok(SessionConnection::from(page))
    }
}

#[derive(Default)]
pub struct Mutation;

/// Merged into the root `Mutation`.
#[Object(name = "SessionsMutation")]
impl Mutation {
    /// Starts a session and returns its single-use tunnel ticket (K4 §1, API-009).
    #[graphql(guard = "Authenticated")]
    async fn start_session(&self, ctx: &Context<'_>, input: StartSessionInput) -> Result<Ticket> {
        let mut request = input.into_request().map_err(|fields| invalid(&fields))?;
        request.wake = Some(ticket::waking(ctx, request.device).await?);
        start(
            schema_dependency::<PgPool>(ctx)?,
            schema_dependency::<Arc<dyn Clock>>(ctx)?,
            schema_dependency::<Live>(ctx)?,
            schema_dependency::<Operator>(ctx)?,
            *schema_dependency::<ClientAddr>(ctx)?,
            request,
        )
        .await
        .map_err(to_graphql)
    }

    /// Ends a session with `OPERATOR`, live or pending; an ended one is returned unchanged
    /// (D-10).
    #[graphql(guard = "Authenticated")]
    async fn end_session(&self, ctx: &Context<'_>, id: ID) -> Result<Session> {
        let session = id
            .parse::<SessionId>()
            .map_err(|_| invalid(&[FieldError::new("id", "That session id is not valid.")]))?;
        let by = actor(ctx)?;
        end_session(
            schema_dependency::<PgPool>(ctx)?,
            schema_dependency::<Live>(ctx)?,
            session,
            &by,
        )
        .await
        .map_err(to_graphql)
    }

    /// Tests SSH, RDP or VNC to a saved device or to the form's values (DEV-013). For SSH a
    /// credential is sent only to the device's accepted host key (SEC-006); an RDP or VNC test
    /// sends none, and RDP reads the server certificate (SEC-007).
    #[graphql(guard = "Authenticated")]
    async fn test_connection(
        &self,
        ctx: &Context<'_>,
        input: ConnectionTestInput,
    ) -> Result<ConnectionTest> {
        let subject = input.into_subject().map_err(|fields| invalid(&fields))?;
        let request = TestRequest {
            subject,
            at: schema_dependency::<Arc<dyn Clock>>(ctx)?.now(),
        };
        test_connection(
            schema_dependency::<PgPool>(ctx)?,
            schema_dependency::<Vault>(ctx)?,
            schema_dependency::<Live>(ctx)?,
            request,
        )
        .await
        .map_err(to_graphql)
    }

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
            schema_dependency::<Live>(ctx)?,
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
            schema_dependency::<Live>(ctx)?,
            &actor(ctx)?,
            acceptance,
        )
        .await
        .map_err(to_graphql)
    }
}
