//! The root queries of the containers of a device (K1; HOST-001, R-149; HOST-017 to
//! HOST-021, R-150). `containerLog` and `containerDetail` are closed to every API key: a log,
//! a command or a label can hold a secret (D-23, MCP-004).

mod disk;
mod objects;
mod sentences;

use std::sync::Arc;

use async_graphql::{Context, ID, Object, Result};
use chrono::{DateTime, Utc};
use sqlx::PgPool;

pub(super) use self::sentences::to_graphql;
use super::{
    Container, ContainerDetail, ContainerId, ContainerLog, ImageUpdate, LineCount, Network, Reach,
    Storage, Tail, detail, image_updates, list, log, networks, storage,
};
use crate::{
    api_errors::{FieldError, invalid, schema_dependency},
    clock::Clock,
    fleet::{
        access::{Authenticated, operator_only},
        credentials::Vault,
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

    /// The details of one container (HOST-017): the names of its environment variables and
    /// no value. Never an API key: a command or a label can hold a secret.
    #[graphql(guard = "Authenticated")]
    async fn container_detail(
        &self,
        ctx: &Context<'_>,
        device_id: ID,
        container_id: String,
    ) -> Result<ContainerDetail> {
        operator_only(ctx, "containerDetail")?;
        let container = ContainerId::parse(&container_id).map_err(to_graphql)?;
        let (reach, device, now) = asked(ctx, &device_id)?;
        detail(reach, device, &container, now)
            .await
            .map_err(to_graphql)
    }

    /// The images and the volumes of a device, with the space a clean-up can free (HOST-020).
    #[graphql(guard = "Authenticated")]
    async fn container_storage(&self, ctx: &Context<'_>, device_id: ID) -> Result<Storage> {
        let (reach, device, now) = asked(ctx, &device_id)?;
        storage(reach, device, now).await.map_err(to_graphql)
    }

    /// The networks of Docker on a device (HOST-020).
    #[graphql(guard = "Authenticated")]
    async fn container_networks(&self, ctx: &Context<'_>, device_id: ID) -> Result<Vec<Network>> {
        let (reach, device, now) = asked(ctx, &device_id)?;
        networks(reach, device, now).await.map_err(to_graphql)
    }

    /// Whether a newer image exists for each container of a device (HOST-021).
    #[graphql(guard = "Authenticated")]
    async fn container_image_updates(
        &self,
        ctx: &Context<'_>,
        device_id: ID,
    ) -> Result<Vec<ImageUpdate>> {
        let (reach, device, now) = asked(ctx, &device_id)?;
        image_updates(reach, device, now).await.map_err(to_graphql)
    }
}
