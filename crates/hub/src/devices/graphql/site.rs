//! `Site` as the GraphQL type K1 describes (VPN-002).

use async_graphql::{Context, ID, Object, Result, dataloader::DataLoader};

use super::{
    connection::{DEFAULT_FIRST, DeviceConnection, paging},
    error::{internal, invalid, to_graphql},
};
use crate::{
    devices::{DevicesBySite, model::Site, page::page_after},
    scalars::Cidr,
};

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
        clippy::unused_async_trait_impl,
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
