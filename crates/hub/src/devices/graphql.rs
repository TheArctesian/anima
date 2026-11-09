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
mod remote;
mod site;
mod ssh;
mod standing;
mod trail;

use std::sync::Arc;

use async_graphql::{Context, ID, Object, Result, dataloader::DataLoader};
use sqlx::PgPool;
use uuid::Uuid;

pub use self::{
    credential_input::{PasswordCredential, SshCredential},
    input::SshSettingsInput,
    remote::{
        RdpCredentialStored, RdpSettingsInput, TransferPolicyInput, VncCredentialStored,
        VncSettingsInput,
    },
    ssh::CredentialStored,
    standing::{
        ChecksOf, GpusOf, ReadingsOf, ResolvedIpOf, StaleOf, StatusOf, SuggestionsOf, UptimeOf,
        WebInterfacesOf, gpus_loader, readings_loader, stale_loader,
        web_interface_suggestions_loader, web_interfaces_loader,
    },
};

use self::{
    connection::{DEFAULT_FIRST, DeviceConnection},
    error::to_graphql,
    filter::{DeviceFilterInput, DeviceSort, list_args},
    input::{CreateDeviceInput, UpdateDeviceInput, checked, device_id},
    remote::{DeviceRdp, DeviceVnc, clear_password, set_password, set_wake_relay},
    ssh::DeviceSsh,
};
use super::{
    DevicesById, DevicesBySite, Error, FieldError, SitesById,
    model::{Device, NewSiteInput, Site},
    rules::{validate_change, validate_new_device, validate_new_site},
    store,
};
use crate::{
    access::Authenticated,
    api_errors::{begin, commit, internal, invalid, pool},
    checks,
    clock::Clock,
    credentials::{PasswordKind, clear_ssh},
    events::Trail,
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

/// The loader behind `RdpSettings.credentialStored`: one query per batch of device ids, no
/// cache.
#[must_use]
pub fn rdp_credential_loader(pool: PgPool) -> DataLoader<RdpCredentialStored> {
    DataLoader::new(RdpCredentialStored(pool), tokio::spawn)
}

/// The loader behind `VncSettings.credentialStored`: one query per batch of device ids, no
/// cache.
#[must_use]
pub fn vnc_credential_loader(pool: PgPool) -> DataLoader<VncCredentialStored> {
    DataLoader::new(VncCredentialStored(pool), tokio::spawn)
}

/// The loader behind `Site.wakeRelay`: one query per batch of device ids, no cache.
#[must_use]
pub fn device_loader(pool: PgPool) -> DataLoader<DevicesById> {
    DataLoader::new(DevicesById(pool), tokio::spawn)
}

/// The loader behind `Device.status`: one `checks::statuses` call per batch, judged at the
/// schema clock's time read once per batch (CHK-004). No cache.
#[must_use]
pub fn status_loader(pool: PgPool, clock: Arc<dyn Clock>) -> DataLoader<StatusOf> {
    DataLoader::new(StatusOf { db: pool, clock }, tokio::spawn)
}

/// The loader behind `Device.uptime`: one `checks::uptimes` call per window per batch. No cache.
#[must_use]
pub fn uptime_loader(pool: PgPool, clock: Arc<dyn Clock>) -> DataLoader<UptimeOf> {
    DataLoader::new(UptimeOf { db: pool, clock }, tokio::spawn)
}

/// The loader behind `Device.checks`: one query per batch of device ids, no cache.
#[must_use]
pub fn checks_loader(pool: PgPool) -> DataLoader<ChecksOf> {
    DataLoader::new(ChecksOf(pool), tokio::spawn)
}

/// The loader behind `Device.ip` for a device whose host is a name: one query per batch, no
/// cache.
#[must_use]
pub fn resolved_ip_loader(pool: PgPool) -> DataLoader<ResolvedIpOf> {
    DataLoader::new(ResolvedIpOf(pool), tokio::spawn)
}

#[derive(Default)]
pub struct Query;

/// Merged into the root `Query` (`schema.rs`); the name only keeps it apart from the others.
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
        let sort = sort.unwrap_or_default();
        let args = list_args(
            sort,
            first.unwrap_or(DEFAULT_FIRST),
            after.as_deref(),
            filter,
        )
        .map_err(|e| invalid(&e))?;
        standing::list(ctx, sort, args).await
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
        trail
            .write(&mut tx, trail::site_added(&site))
            .await
            .map_err(internal)?;
        commit(tx).await?;
        Ok(site)
    }

    #[graphql(guard = "Authenticated")]
    async fn create_device(&self, ctx: &Context<'_>, input: CreateDeviceInput) -> Result<Device> {
        let add_default_check = input.add_default_check();
        let (form, bad_site) = input.into_form();
        let new = checked(validate_new_device(&form), bad_site)?;
        let trail = Trail::from_context(ctx)?;
        let id = DeviceId::from(Uuid::now_v7());
        let mut tx = begin(ctx).await?;
        let device = store::create_device(&mut tx, id, trail.now, &new)
            .await
            .map_err(to_graphql)?;
        if add_default_check {
            checks::add_default_check(&mut tx, id)
                .await
                .map_err(internal)?;
        }
        trail
            .write(
                &mut tx,
                trail::device_added(&device, &new, add_default_check),
            )
            .await
            .map_err(internal)?;
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
            .await
            .map_err(internal)?;
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
        trail
            .write(&mut tx, trail::device_deleted(&device))
            .await
            .map_err(internal)?;
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

    /// Sets or, with a null `deviceId`, clears the device that wakes the site's devices (D-7).
    #[graphql(guard = "Authenticated")]
    async fn set_site_wake_relay(
        &self,
        ctx: &Context<'_>,
        site_id: ID,
        device_id: Option<ID>,
    ) -> Result<Site> {
        set_wake_relay(ctx, &site_id, device_id.as_ref()).await
    }

    /// Seals and stores the device's RDP password, replacing any earlier one (DEV-011, SEC-003).
    #[graphql(guard = "Authenticated")]
    async fn set_rdp_credential(
        &self,
        ctx: &Context<'_>,
        device_id: ID,
        credential: PasswordCredential,
    ) -> Result<DeviceRdp> {
        let device = set_password(ctx, &device_id, credential, PasswordKind::Rdp).await?;
        Ok(DeviceRdp::new(&device, Some(true)))
    }

    /// Deletes the device's RDP password.
    #[graphql(guard = "Authenticated")]
    async fn clear_rdp_credential(&self, ctx: &Context<'_>, device_id: ID) -> Result<DeviceRdp> {
        let device = clear_password(ctx, &device_id, PasswordKind::Rdp).await?;
        Ok(DeviceRdp::new(&device, Some(false)))
    }

    /// Seals and stores the device's VNC password, replacing any earlier one (DEV-011, SEC-003).
    #[graphql(guard = "Authenticated")]
    async fn set_vnc_credential(
        &self,
        ctx: &Context<'_>,
        device_id: ID,
        credential: PasswordCredential,
    ) -> Result<DeviceVnc> {
        let device = set_password(ctx, &device_id, credential, PasswordKind::Vnc).await?;
        Ok(DeviceVnc::new(&device, Some(true)))
    }

    /// Deletes the device's VNC password.
    #[graphql(guard = "Authenticated")]
    async fn clear_vnc_credential(&self, ctx: &Context<'_>, device_id: ID) -> Result<DeviceVnc> {
        let device = clear_password(ctx, &device_id, PasswordKind::Vnc).await?;
        Ok(DeviceVnc::new(&device, Some(false)))
    }
}
