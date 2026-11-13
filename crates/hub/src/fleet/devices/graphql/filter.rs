//! The arguments of `Query.devices`: which devices, in which order (VPN-002).

#![expect(
    clippy::unused_async_trait_impl,
    reason = "the async-graphql Enum derive generates async trait impls"
)]

use async_graphql::{Enum, ID, InputObject};

use super::standing::{Position, decode_cursor};
use crate::{
    fleet::devices::{FieldError, model::DeviceFilter, page::page_size, rules::validate_filter},
    ids::SiteId,
    status::Status,
};

/// The order of a device list. NAME is ordered in SQL. STATUS (worst first) and UPTIME (lowest
/// first, no figure last; the DAY window) are ordered in memory, with the name as tie-break.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Enum)]
pub(super) enum DeviceSort {
    #[default]
    Name,
    Status,
    Uptime,
}

/// Which devices a list returns (K1 `DeviceFilter`).
#[derive(InputObject)]
#[graphql(name = "DeviceFilter")]
pub(super) struct DeviceFilterInput {
    /// Any of these statuses; null or empty means every status.
    status: Option<Vec<Status>>,
    tags: Option<Vec<String>>,
    /// Case-insensitive substring of the name.
    name: Option<String>,
    site_id: Option<ID>,
}

impl DeviceFilterInput {
    fn into_filter(self) -> Result<(DeviceFilter, Vec<Status>), Vec<FieldError>> {
        let statuses = self.status.unwrap_or_default();
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
            (Ok(site), Ok(filter)) => Ok((DeviceFilter { site, ..filter }, statuses)),
            (site, sized) => Err(site
                .err()
                .into_iter()
                .chain(sized.err().into_iter().flatten())
                .collect()),
        }
    }
}

/// The validated arguments of `devices`.
pub(super) struct ListArgs {
    pub(super) size: usize,
    pub(super) after: Option<Position>,
    pub(super) filter: DeviceFilter,
    pub(super) statuses: Vec<Status>,
}

/// Reads the arguments of `devices`, reporting every bad one at once. The cursor must have
/// been made for `sort`.
pub(super) fn list_args(
    sort: DeviceSort,
    first: i32,
    after: Option<&str>,
    filter: Option<DeviceFilterInput>,
) -> Result<ListArgs, Vec<FieldError>> {
    let filter = filter.map_or_else(
        || Ok((DeviceFilter::default(), Vec::new())),
        DeviceFilterInput::into_filter,
    );
    let after = after.map(|text| decode_cursor(sort, text)).transpose();
    match (page_size(first), after, filter) {
        (Ok(size), Ok(after), Ok((filter, statuses))) => Ok(ListArgs {
            size,
            after,
            filter,
            statuses,
        }),
        (size, after, filter) => Err(size
            .err()
            .into_iter()
            .chain(after.err())
            .chain(filter.err().into_iter().flatten())
            .collect()),
    }
}
