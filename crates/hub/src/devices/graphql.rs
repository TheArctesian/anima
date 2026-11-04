//! The read side of the devices GraphQL part (K1; API-005, API-006, VPN-002).
//!
//! Each `#[Object]` block below is the single resolver block for its type. Later packages add
//! the fields that join other domains (`Device.status`, `Device.checks`, `Device.uptime`,
//! `SshSettings.hostKey`) to these same blocks (R-011).

#![expect(
    clippy::unused_async_trait_impl,
    reason = "the async-graphql Enum derive generates async trait impls"
)]

use async_graphql::{
    Context, Enum, ID, InputObject, Object, Result, SimpleObject, dataloader::DataLoader,
};
use chrono::{DateTime, Utc};
use sqlx::PgPool;

use super::{
    DevicesBySite, Error, FieldError, Missing, OperatingSystem, SitesById, SshAuth,
    graphql_error::{internal, invalid, schema_dependency, to_graphql},
    model::{Device, DeviceFilter, Site, SshSettings},
    page::{decode_cursor, encode_cursor, page_after, page_size},
    store,
};
use crate::{
    ids::SiteId,
    paging::PageEnd,
    scalars::{Cidr, IpAddr},
};

/// K1's default for `first`.
const DEFAULT_FIRST: i32 = 50;

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

/// The order of a device list. Only the name order exists until checks add status and uptime.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Enum)]
enum DeviceSort {
    #[default]
    Name,
}

/// Which devices a list returns (K1 `DeviceFilter`, without the status part).
#[derive(InputObject)]
#[graphql(name = "DeviceFilter")]
struct DeviceFilterInput {
    tags: Option<Vec<String>>,
    /// Case-insensitive substring of the name.
    name: Option<String>,
    site_id: Option<ID>,
}

impl DeviceFilterInput {
    fn into_filter(self) -> Result<DeviceFilter, FieldError> {
        let site = self
            .site_id
            .map(|id| {
                id.parse::<SiteId>()
                    .map_err(|_| FieldError::new("siteId", "That site id is not valid."))
            })
            .transpose()?;
        Ok(DeviceFilter {
            name: self.name,
            tags: self.tags.unwrap_or_default(),
            site,
        })
    }
}

#[derive(SimpleObject)]
struct DeviceEdge {
    cursor: String,
    node: Device,
}

#[derive(SimpleObject)]
struct DeviceConnection {
    edges: Vec<DeviceEdge>,
    page_info: PageEnd,
    total_count: i32,
}

impl DeviceConnection {
    fn new(devices: Vec<Device>, has_next_page: bool, total: usize) -> Self {
        let edges: Vec<_> = devices
            .into_iter()
            .map(|node| DeviceEdge {
                cursor: encode_cursor(&node.name),
                node,
            })
            .collect();
        let end_cursor = edges.last().map(|edge| edge.cursor.clone());
        Self {
            edges,
            page_info: PageEnd {
                has_next_page,
                end_cursor,
            },
            total_count: i32::try_from(total).unwrap_or(i32::MAX),
        }
    }
}

/// Reads `first` and `after`, reporting every bad one.
fn paging(first: i32, after: Option<&str>) -> Result<(usize, Option<String>), Vec<FieldError>> {
    match (page_size(first), after.map(decode_cursor).transpose()) {
        (Ok(size), Ok(after)) => Ok((size, after)),
        (size, after) => Err(size.err().into_iter().chain(after.err()).collect()),
    }
}

/// Reads the arguments of `devices`, reporting every bad one at once.
fn list_args(
    first: i32,
    after: Option<&str>,
    filter: Option<DeviceFilterInput>,
) -> Result<(usize, Option<String>, DeviceFilter), Vec<FieldError>> {
    let filter = filter.map_or_else(
        || Ok(DeviceFilter::default()),
        DeviceFilterInput::into_filter,
    );
    match (paging(first, after), filter) {
        (Ok((size, after)), Ok(filter)) => Ok((size, after, filter)),
        (paging, filter) => Err(paging
            .err()
            .into_iter()
            .flatten()
            .chain(filter.err())
            .collect()),
    }
}

fn pool<'a>(ctx: &Context<'a>) -> Result<&'a PgPool> {
    schema_dependency::<PgPool>(ctx)
}

#[derive(Default)]
pub struct Query;

#[Object]
impl Query {
    async fn sites(&self, ctx: &Context<'_>) -> Result<Vec<Site>> {
        store::list_sites(pool(ctx)?).await.map_err(to_graphql)
    }

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

    async fn tags(&self, ctx: &Context<'_>) -> Result<Vec<String>> {
        store::tags(pool(ctx)?).await.map_err(to_graphql)
    }
}

#[Object]
impl Site {
    async fn id(&self) -> ID {
        ID(self.id.to_string())
    }

    async fn name(&self) -> &str {
        &self.name
    }

    #[expect(
        clippy::unused_async,
        reason = "the Object macro requires resolvers to be async"
    )]
    async fn subnet(&self) -> Result<Cidr> {
        // The database column is `cidr`, which cannot hold host bits.
        Cidr::new(self.subnet).map_err(internal)
    }

    async fn devices(
        &self,
        ctx: &Context<'_>,
        #[graphql(default_with = "Some(DEFAULT_FIRST)")] first: Option<i32>,
        after: Option<String>,
    ) -> Result<DeviceConnection> {
        let (size, after) =
            paging(first.unwrap_or(DEFAULT_FIRST), after.as_deref()).map_err(|e| invalid(&e))?;
        let all = ctx
            .data::<DataLoader<DevicesBySite>>()?
            .load_one(self.id)
            .await
            .map_err(to_graphql)?
            .unwrap_or_default();
        let page = page_after(&all, |d| d.name.as_str(), after.as_deref(), size);
        let shown = page.rows.into_iter().cloned().collect();
        Ok(DeviceConnection::new(shown, page.has_next_page, all.len()))
    }
}

#[Object]
impl Device {
    async fn id(&self) -> ID {
        ID(self.id.to_string())
    }

    async fn site(&self, ctx: &Context<'_>) -> Result<Site> {
        schema_dependency::<DataLoader<SitesById>>(ctx)?
            .load_one(self.site)
            .await
            .map_err(to_graphql)?
            .ok_or_else(|| to_graphql(Error::NotFound(Missing::Site(self.site))))
    }

    async fn name(&self) -> &str {
        &self.name
    }

    async fn host(&self) -> &str {
        &self.host
    }

    async fn ip(&self) -> Option<IpAddr> {
        self.ip.map(IpAddr)
    }

    async fn kind(&self) -> &str {
        &self.kind
    }

    async fn os(&self) -> OperatingSystem {
        self.os
    }

    async fn tags(&self) -> &[String] {
        &self.tags
    }

    /// Manual maintenance flag (CHK-006).
    async fn maintenance(&self) -> bool {
        self.maintenance
    }

    async fn ssh(&self) -> &SshSettings {
        &self.ssh
    }

    async fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    async fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }
}

#[Object]
impl SshSettings {
    async fn enabled(&self) -> bool {
        self.enabled
    }

    async fn port(&self) -> i32 {
        i32::from(self.port)
    }

    async fn username(&self) -> &str {
        &self.username
    }

    async fn auth(&self) -> SshAuth {
        self.auth
    }
}
