//! The page of devices that `Query.devices` and `Site.devices` return (API-006).

use async_graphql::SimpleObject;

use crate::{
    fleet::devices::{
        FieldError,
        model::Device,
        page::{decode_cursor, encode_cursor, page_size},
    },
    paging::PageEnd,
};

/// K1's default for `first`.
pub(super) const DEFAULT_FIRST: i32 = 50;

#[derive(SimpleObject)]
pub(super) struct DeviceEdge {
    cursor: String,
    node: Device,
}

#[derive(SimpleObject)]
pub(super) struct DeviceConnection {
    edges: Vec<DeviceEdge>,
    page_info: PageEnd,
    total_count: i32,
}

impl DeviceConnection {
    pub(super) fn new(devices: Vec<Device>, has_next_page: bool, total: usize) -> Self {
        let cursored = devices
            .into_iter()
            .map(|node| (encode_cursor(&node.name), node))
            .collect();
        Self::with_cursors(cursored, has_next_page, total)
    }

    /// A page whose edges already carry their cursors.
    pub(super) fn with_cursors(
        devices: Vec<(String, Device)>,
        has_next_page: bool,
        total: usize,
    ) -> Self {
        let edges: Vec<_> = devices
            .into_iter()
            .map(|(cursor, node)| DeviceEdge { cursor, node })
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
pub(super) fn paging(
    first: i32,
    after: Option<&str>,
) -> Result<(usize, Option<String>), Vec<FieldError>> {
    match (page_size(first), after.map(decode_cursor).transpose()) {
        (Ok(size), Ok(after)) => Ok((size, after)),
        (size, after) => Err(size.err().into_iter().chain(after.err()).collect()),
    }
}
