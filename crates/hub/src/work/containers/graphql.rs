//! `Query.containers` and `Query.containerLog` (K1; HOST-001, R-149), and each error as its
//! K7 code with a fixed sentence (API-010). `containerLog` is closed to every API key: a log
//! can hold a secret (D-23, MCP-004).

use std::sync::Arc;

use async_graphql::{Context, Error as GraphQlError, ID, Object, Result};
use chrono::{DateTime, Utc};
use sqlx::PgPool;

use super::{
    Container, ContainerId, ContainerLog, ContainerState, Error, Health, LineCount, LogLine, Reach,
    Tail, list, log,
};
use crate::{
    api_errors::{ErrorCode, FieldError, coded, internal, invalid, schema_dependency},
    clock::Clock,
    fleet::{
        access::{Authenticated, operator_only},
        credentials::Vault,
        ssh::LinkError,
        targets::AddressPolicy,
    },
    ids::DeviceId,
};

#[derive(Default)]
pub struct Query;

/// What a read needs from the schema: the stores of a link, the device and the time.
fn asked<'a>(ctx: &Context<'a>, device_id: &ID) -> Result<(Reach<'a>, DeviceId, DateTime<Utc>)> {
    let device = device_id
        .parse::<DeviceId>()
        .map_err(|_| invalid(&[FieldError::new("deviceId", "That device id is not valid.")]))?;
    let reach = Reach {
        db: schema_dependency::<PgPool>(ctx)?,
        vault: schema_dependency::<Vault>(ctx)?,
        policy: schema_dependency::<AddressPolicy>(ctx)?,
    };
    let now = schema_dependency::<Arc<dyn Clock>>(ctx)?.now();
    Ok((reach, device, now))
}

/// Merged into the root `Query`.
#[Object(name = "ContainersQuery")]
impl Query {
    /// The containers of a device, read now through the hub's SSH link (HOST-001).
    #[graphql(guard = "Authenticated")]
    async fn containers(&self, ctx: &Context<'_>, device_id: ID) -> Result<Vec<Container>> {
        let (reach, device, now) = asked(ctx, &device_id)?;
        list(reach, device, now).await.map_err(to_graphql)
    }

    /// The last lines of the log of one container (HOST-001). Never an API key.
    #[graphql(guard = "Authenticated")]
    #[expect(
        clippy::too_many_arguments,
        reason = "K1 fixes the argument list of Query.containerLog"
    )]
    async fn container_log(
        &self,
        ctx: &Context<'_>,
        device_id: ID,
        container_id: String,
        lines: Option<i32>,
    ) -> Result<ContainerLog> {
        operator_only(ctx, "containerLog")?;
        let tail = ContainerId::parse(&container_id)
            .and_then(|container| {
                Ok(Tail {
                    container,
                    lines: LineCount::new(lines)?,
                })
            })
            .map_err(to_graphql)?;
        let (reach, device, now) = asked(ctx, &device_id)?;
        log(reach, device, &tail, now).await.map_err(to_graphql)
    }
}

#[Object]
impl Container {
    async fn id(&self) -> &str {
        &self.id
    }

    async fn name(&self) -> &str {
        &self.name
    }

    async fn image(&self) -> &str {
        &self.image
    }

    async fn state(&self) -> ContainerState {
        self.state
    }

    async fn status(&self) -> &str {
        &self.status
    }

    async fn health(&self) -> Health {
        self.health
    }

    async fn ports(&self) -> &[String] {
        &self.ports
    }

    async fn created_at(&self) -> Option<DateTime<Utc>> {
        self.created_at
    }

    async fn compose_project(&self) -> Option<&str> {
        self.compose_project.as_deref()
    }

    async fn compose_service(&self) -> Option<&str> {
        self.compose_service.as_deref()
    }
}

#[Object]
impl ContainerLog {
    async fn lines(&self) -> &[LogLine] {
        &self.lines
    }

    async fn cut(&self) -> bool {
        self.cut
    }
}

#[Object(name = "ContainerLogLine")]
impl LogLine {
    async fn at(&self) -> Option<DateTime<Utc>> {
        self.at
    }

    async fn text(&self) -> &str {
        &self.text
    }
}

/// The GraphQL error with its `extensions.code` (K7) and the fixed sentence; a store failure
/// is `INTERNAL` with no detail.
pub(super) fn to_graphql(error: Error) -> GraphQlError {
    match &error {
        Error::InvalidContainerId => invalid(&[FieldError::new(
            "containerId",
            "A container identifier is 12 to 64 lower-case hexadecimal characters.",
        )]),
        Error::InvalidLineCount => invalid(&[FieldError::new(
            "lines",
            "The number of lines is 1 to 1000.",
        )]),
        other => match stated(other) {
            Some((code, sentence)) => coded(sentence, code),
            None => internal(error),
        },
    }
}

/// The code and the sentence the operator reads; `None` for a store failure.
const fn stated(error: &Error) -> Option<(ErrorCode, &'static str)> {
    match error {
        Error::NotFound(_) => Some((ErrorCode::NotFound, "No device has this ID.")),
        Error::NoContainer => Some((
            ErrorCode::NotFound,
            "The device has no container with this identifier.",
        )),
        Error::NoDocker => Some((
            ErrorCode::Conflict,
            "Docker was not read: the device's SSH user finds no docker command. Anima \
             reads Docker only.",
        )),
        Error::DockerFailed => Some((
            ErrorCode::Unavailable,
            "Docker was not read: docker did not succeed on the device. Check that \
             Docker runs and that the device's SSH user may use it.",
        )),
        Error::Unreadable => Some((
            ErrorCode::Unavailable,
            "Docker was not read: the hub could not read the answer of docker.",
        )),
        Error::Link(link) => link_stated(link),
        Error::InvalidContainerId | Error::InvalidLineCount | Error::Devices(_) => None,
    }
}

/// [`stated`] for a link to the device that failed. What the operator can fix is `CONFLICT`;
/// a device that does not answer is `UNAVAILABLE`.
const fn link_stated(error: &LinkError) -> Option<(ErrorCode, &'static str)> {
    match error {
        LinkError::NoCredential => Some((
            ErrorCode::Conflict,
            "Docker was not read: the device has no SSH credential in Anima. Anima \
             reads the containers of a device through SSH.",
        )),
        LinkError::SshOff => Some((
            ErrorCode::Conflict,
            "Docker was not read: SSH is turned off for the device. Turn it on in the \
             device's SSH settings.",
        )),
        LinkError::NoHostKey => Some((
            ErrorCode::Conflict,
            "Docker was not read: no host key is accepted for the device. Test its \
             connection and accept its key.",
        )),
        LinkError::HostKeyChanged => Some((
            ErrorCode::Conflict,
            "Docker was not read: the device presented another host key than the \
             accepted one. No credential was sent.",
        )),
        LinkError::RefusedTarget => Some((
            ErrorCode::Conflict,
            "Docker was not read: the device's host resolves to an address the hub may \
             not connect to.",
        )),
        LinkError::CredentialForOtherHost
        | LinkError::CredentialRefused
        | LinkError::UnreadableKey => Some((
            ErrorCode::Conflict,
            "Docker was not read: the device did not accept the stored SSH \
                 credential. Enter it again in the device's SSH settings.",
        )),
        LinkError::Unresolved
        | LinkError::Unreachable
        | LinkError::Timeout
        | LinkError::NoKeyExchange
        | LinkError::Lost => Some((
            ErrorCode::Unavailable,
            "Docker was not read: the device cannot be reached over SSH.",
        )),
        LinkError::CommandRefused | LinkError::TimeLimit => Some((
            ErrorCode::Unavailable,
            "Docker was not read: the device did not finish the docker command in time.",
        )),
        LinkError::Trust(_) | LinkError::Credentials(_) => None,
    }
}
