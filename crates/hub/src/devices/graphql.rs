//! The devices GraphQL part: root `Query` and `Mutation` and the loader constructors (K1;
//! API-003, API-005, API-006, VPN-002, DEV-010, DEV-015).
//!
//! The types live in the sub-modules; each `#[Object]` block there is the single resolver block
//! for its type. Later packages add the fields that join other domains (`Device.status`,
//! `Device.checks`, `Device.uptime`, `SshSettings.hostKey`) to these same types (R-011).

mod connection;
mod credential;
mod credential_input;
mod device;
pub(super) mod error;
mod filter;
mod input;
mod site;
mod ssh;
mod trail;

use async_graphql::{Context, ID, Object, Result, dataloader::DataLoader};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

pub use self::{credential_input::SshCredential, input::SshSettingsInput, ssh::CredentialStored};

use self::{
    connection::{DEFAULT_FIRST, DeviceConnection},
    error::{invalid, schema_dependency, to_graphql},
    filter::{DeviceFilterInput, DeviceSort, list_args},
    input::{CreateDeviceInput, UpdateDeviceInput, checked, device_id},
    ssh::DeviceSsh,
    trail::Trail,
};
use super::{
    DevicesBySite, Error, FieldError, SitesById,
    model::{Device, NewSiteInput, Site},
    rules::{validate_change, validate_new_device, validate_new_site},
    store,
};
use crate::{
    access::Authenticated,
    credentials::clear_ssh,
    ids::{DeviceId, SiteId},
    scalars::Cidr,
};

/// The loader behind `Device.site`: one query per batch of site ids. It has no cache, so one
/// instance serves every request.
#[must_use]
pub fn site_loader(pool: PgPool) -> DataLoader<SitesById> {
    DataLoader::new(SitesById(pool), tokio::spawn)
}

/// The loader behind `Site.devices`: one query per batch of site ids, no cache.
#[must_use]
pub fn devices_by_site_loader(pool: PgPool) -> DataLoader<DevicesBySite> {
    DataLoader::new(DevicesBySite(pool), tokio::spawn)
}

/// The loader behind `SshSettings.credentialStored`: one query per batch of device ids, no
/// cache, so a secret stored or cleared shows in the next request.
#[must_use]
pub fn credential_loader(pool: PgPool) -> DataLoader<CredentialStored> {
    DataLoader::new(CredentialStored(pool), tokio::spawn)
}

fn pool<'a>(ctx: &Context<'a>) -> Result<&'a PgPool> {
    schema_dependency::<PgPool>(ctx)
}

/// The transaction a mutation writes its change, its event and its audit entry in.
async fn begin(ctx: &Context<'_>) -> Result<Transaction<'static, Postgres>> {
    pool(ctx)?
        .begin()
        .await
        .map_err(|e| to_graphql(Error::Database(e)))
}

async fn commit(tx: Transaction<'static, Postgres>) -> Result<()> {
    tx.commit()
        .await
        .map_err(|e| to_graphql(Error::Database(e)))
}

#[derive(Default)]
pub struct Query;

/// Merged into the root `Query`; the name only keeps it apart from the other parts.
#[Object(name = "DevicesQuery")]
impl Query {
    #[graphql(guard = "Authenticated")]
    async fn sites(&self, ctx: &Context<'_>) -> Result<Vec<Site>> {
        store::list_sites(pool(ctx)?).await.map_err(to_graphql)
    }

    #[graphql(guard = "Authenticated")]
    async fn device(&self, ctx: &Context<'_>, id: ID) -> Result<Option<Device>> {
        let id = id
            .parse()
            .map_err(|_| invalid(&[FieldError::new("id", "That device id is not valid.")]))?;
        match store::get(pool(ctx)?, id).await {
            Ok(device) => Ok(Some(device)),
            Err(Error::NotFound(_)) => Ok(None),
            Err(error) => Err(to_graphql(error)),
        }
    }

    #[graphql(guard = "Authenticated")]
    #[expect(
        clippy::too_many_arguments,
        reason = "K1 fixes the argument list of Query.devices"
    )]
    async fn devices(
        &self,
        ctx: &Context<'_>,
        filter: Option<DeviceFilterInput>,
        #[graphql(default_with = "Some(DeviceSort::Name)")] sort: Option<DeviceSort>,
        #[graphql(default_with = "Some(DEFAULT_FIRST)")] first: Option<i32>,
        after: Option<String>,
    ) -> Result<DeviceConnection> {
        let DeviceSort::Name = sort.unwrap_or_default();
        let (size, after, filter) =
            list_args(first.unwrap_or(DEFAULT_FIRST), after.as_deref(), filter)
                .map_err(|e| invalid(&e))?;
        let page = store::list(pool(ctx)?, &filter, size, after.as_deref())
            .await
            .map_err(to_graphql)?;
        let total = usize::try_from(page.total).unwrap_or(0);
        Ok(DeviceConnection::new(
            page.devices,
            page.has_next_page,
            total,
        ))
    }

    #[graphql(guard = "Authenticated")]
    async fn tags(&self, ctx: &Context<'_>) -> Result<Vec<String>> {
        store::tags(pool(ctx)?).await.map_err(to_graphql)
    }
}

/// Each mutation validates, then writes its change, one event and one audit entry in one
/// transaction: if any step fails, nothing is stored.
#[derive(Default)]
pub struct Mutation;

/// Merged into the root `Mutation`.
#[Object(name = "DevicesMutation")]
impl Mutation {
    #[graphql(guard = "Authenticated")]
    async fn create_site(&self, ctx: &Context<'_>, name: String, subnet: Cidr) -> Result<Site> {
        let new = checked(validate_new_site(&NewSiteInput { name, subnet }), None)?;
        let trail = Trail::from_context(ctx)?;
        let id = SiteId::from(Uuid::now_v7());
        let mut tx = begin(ctx).await?;
        let site = store::create_site(&mut tx, id, &new)
            .await
            .map_err(to_graphql)?;
        trail.write(&mut tx, trail::site_added(&site)).await?;
        commit(tx).await?;
        Ok(site)
    }

    #[graphql(guard = "Authenticated")]
    async fn create_device(&self, ctx: &Context<'_>, input: CreateDeviceInput) -> Result<Device> {
        let (form, bad_site) = input.into_form();
        let new = checked(validate_new_device(&form), bad_site)?;
        let trail = Trail::from_context(ctx)?;
        let id = DeviceId::from(Uuid::now_v7());
        let mut tx = begin(ctx).await?;
        let device = store::create_device(&mut tx, id, trail.now, &new)
            .await
            .map_err(to_graphql)?;
        trail
            .write(&mut tx, trail::device_added(&device, &new))
            .await?;
        commit(tx).await?;
        Ok(device)
    }

    #[graphql(guard = "Authenticated")]
    async fn update_device(
        &self,
        ctx: &Context<'_>,
        id: ID,
        input: UpdateDeviceInput,
    ) -> Result<Device> {
        let (form, bad_site) = input.into_form();
        let id = device_id(&id);
        let bad_ids: Vec<_> = id
            .as_ref()
            .err()
            .cloned()
            .into_iter()
            .chain(bad_site)
            .collect();
        let change = checked(validate_change(&form), bad_ids)?;
        let id = id.map_err(|error| invalid(&[error]))?;
        if change.is_empty() {
            // Nothing to write: no new `updated_at`, no event, no audit entry.
            return store::get(pool(ctx)?, id).await.map_err(to_graphql);
        }
        let trail = Trail::from_context(ctx)?;
        let mut tx = begin(ctx).await?;
        let device = store::update_device(&mut tx, id, trail.now, &change)
            .await
            .map_err(to_graphql)?;
        trail
            .write(&mut tx, trail::device_updated(&device, &change))
            .await?;
        commit(tx).await?;
        Ok(device)
    }

    #[graphql(guard = "Authenticated")]
    async fn delete_device(&self, ctx: &Context<'_>, id: ID) -> Result<ID> {
        let id = device_id(&id).map_err(|e| invalid(&[e]))?;
        let trail = Trail::from_context(ctx)?;
        let mut tx = begin(ctx).await?;
        // The secret goes first, under the credentials lock, before the device row is locked:
        // the order a concurrent `setSshCredential` takes, so the two cannot deadlock.
        clear_ssh(&mut tx, id)
            .await
            .map_err(|error| credential::credential_error(error, id))?;
        let device = store::delete_device(&mut tx, id)
            .await
            .map_err(to_graphql)?;
        trail.write(&mut tx, trail::device_deleted(&device)).await?;
        commit(tx).await?;
        Ok(ID(device.id.to_string()))
    }

    /// Seals and stores the device's SSH secret, replacing any earlier one (DEV-012, SEC-003).
    #[graphql(guard = "Authenticated")]
    async fn set_ssh_credential(
        &self,
        ctx: &Context<'_>,
        device_id: ID,
        credential: SshCredential,
    ) -> Result<DeviceSsh> {
        credential::set(ctx, &device_id, credential).await
    }

    /// Deletes the device's SSH secret.
    #[graphql(guard = "Authenticated")]
    async fn clear_ssh_credential(&self, ctx: &Context<'_>, device_id: ID) -> Result<DeviceSsh> {
        credential::clear(ctx, &device_id).await
    }
}
