//! `wakeDevice` and `WakeResult` (K1; RA-031, D-8): a `Mutation` part behind the same guard as
//! the sessions part, and the errors as the sessions part maps its own (K7, API-010).

use std::sync::Arc;

use async_graphql::{Context, Error as GraphQlError, ID, Object, Result};
use chrono::{DateTime, Utc};
use sqlx::PgPool;

use super::{Error, WakeSent, audit, refusal::sentence, wake};
use crate::{
    access::Authenticated,
    api_errors::{ErrorCode, coded, internal, invalid, schema_dependency},
    clock::Clock,
    credentials::Vault,
    events::{Bus, ClientAddr, Operator},
    ids::DeviceId,
    sessions::{Actor, FieldError, LinkError, Missing},
    targets::AddressPolicy,
};

#[derive(Default)]
pub struct Mutation;

/// Merged into the root `Mutation`.
#[Object(name = "WakeMutation")]
impl Mutation {
    /// Sends a Wake-on-LAN packet for the device from its site's wake relay, and writes the
    /// audit entry `device.wake` when the relay sent it (RA-031, LOG-005).
    #[graphql(guard = "Authenticated")]
    async fn wake_device(&self, ctx: &Context<'_>, device_id: ID) -> Result<WakeSent> {
        let device = device_id
            .parse::<DeviceId>()
            .map_err(|_| invalid(&[FieldError::new("deviceId", "That device id is not valid.")]))?;
        let by = Actor {
            operator: schema_dependency::<Operator>(ctx)?.clone(),
            client: *schema_dependency::<ClientAddr>(ctx)?,
            at: schema_dependency::<Arc<dyn Clock>>(ctx)?.now(),
        };
        let db = schema_dependency::<PgPool>(ctx)?;
        let sent = wake(
            db,
            schema_dependency::<Vault>(ctx)?,
            schema_dependency::<AddressPolicy>(ctx)?,
            schema_dependency::<Bus>(ctx)?,
            device,
            by.at,
        )
        .await
        .map_err(to_graphql)?;
        // A sent packet whose audit entry failed is logged here and stays `INTERNAL`: the
        // mutation never reports success for an action it could not audit (LOG-005).
        if let Err(error) = audit(db, &by, device, &sent).await {
            tracing::error!(%device, %error, "the audit entry of a sent Wake-on-LAN could not be written");
            return Err(to_graphql(error));
        }
        Ok(sent)
    }
}

#[Object(name = "WakeResult")]
impl WakeSent {
    async fn sent_at(&self) -> DateTime<Utc> {
        self.sent_at
    }

    async fn relay_name(&self) -> &str {
        &self.relay_name
    }
}

/// The GraphQL error with its `extensions.code` (K7) and the fixed sentence; a store failure
/// is `INTERNAL` with no detail.
fn to_graphql(error: Error) -> GraphQlError {
    let Some(code) = code(&error) else {
        return internal(error);
    };
    let message = match &error {
        Error::NotFound(device) => format!("No such {}.", Missing::Device(*device)),
        other => sentence(other).to_owned(),
    };
    coded(message, code)
}

/// What the operator can fix is `CONFLICT`; a relay that does not answer or a command that
/// fails is `UNAVAILABLE`; `None` for a store failure.
const fn code(error: &Error) -> Option<ErrorCode> {
    match error {
        Error::NotFound(_) => Some(ErrorCode::NotFound),
        Error::NoMacAddress | Error::NoRelay | Error::RelayIsDevice => Some(ErrorCode::Conflict),
        Error::NotSent => Some(ErrorCode::Unavailable),
        Error::Link(link) => link_code(link),
        Error::Devices(_) | Error::Events(_) | Error::Database(_) => None,
    }
}

/// [`code`] for a link to the relay that failed.
const fn link_code(error: &LinkError) -> Option<ErrorCode> {
    match error {
        LinkError::SshOff
        | LinkError::NoHostKey
        | LinkError::HostKeyChanged
        | LinkError::RefusedTarget
        | LinkError::NoCredential
        | LinkError::CredentialRefused
        | LinkError::UnreadableKey => Some(ErrorCode::Conflict),
        LinkError::Unresolved
        | LinkError::Unreachable
        | LinkError::Timeout
        | LinkError::NoKeyExchange
        | LinkError::Lost
        | LinkError::CommandRefused
        | LinkError::TimeLimit => Some(ErrorCode::Unavailable),
        LinkError::Sessions(_) | LinkError::Credentials(_) => None,
    }
}
