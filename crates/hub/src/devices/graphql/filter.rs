//! The arguments of `Query.devices`: which devices, in which order (VPN-002).

#![expect(
    clippy::unused_async_trait_impl,
    reason = "the async-graphql Enum derive generates async trait impls"
)]

use async_graphql::{Enum, ID, InputObject};

use super::connection::paging;
use crate::{
    devices::{FieldError, model::DeviceFilter, rules::validate_filter},
    ids::SiteId,
};

/// The order of a device list. Only the name order exists until checks add status and uptime.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Enum)]
pub(super) enum DeviceSort {
    #[default]
    Name,
}

/// Which devices a list returns (K1 `DeviceFilter`, without the status part).
#[derive(InputObject)]
#[graphql(name = "DeviceFilter")]
pub(super) struct DeviceFilterInput {
    tags: Option<Vec<String>>,
    /// Case-insensitive substring of the name.
    name: Option<String>,
    site_id: Option<ID>,
}

impl DeviceFilterInput {
    fn into_filter(self) -> Result<DeviceFilter, Vec<FieldError>> {
        let site = self
            .site_id
            .map(|id| {
                id.parse::<SiteId>()
                    .map_err(|_| FieldError::new("siteId", "That site id is not valid."))
            })
            .transpose();
        let sized = validate_filter(DeviceFilter {
            name: self.name,
            tags: self.tags.unwrap_or_default(),
            site: None,
        });
        match (site, sized) {
            (Ok(site), Ok(filter)) => Ok(DeviceFilter { site, ..filter }),
            (site, sized) => Err(site
                .err()
                .into_iter()
                .chain(sized.err().into_iter().flatten())
                .collect()),
        }
    }
}

/// Reads the arguments of `devices`, reporting every bad one at once.
pub(super) fn list_args(
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
            .chain(filter.err().into_iter().flatten())
            .collect()),
    }
}
