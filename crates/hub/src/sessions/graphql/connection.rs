//! The page of sessions that `Query.sessions` returns (API-006) and its arguments.

use async_graphql::{ID, SimpleObject};

use crate::{
    ids::DeviceId,
    paging::PageEnd,
    sessions::{
        model::{FieldError, Session, SessionFilter},
        page::{Cursor, Page, decode_cursor, encode_cursor, page_size},
    },
};

/// K1's default for `first`.
pub(super) const DEFAULT_FIRST: i32 = 50;

#[derive(SimpleObject)]
pub(super) struct SessionEdge {
    cursor: String,
    node: Session,
}

#[derive(SimpleObject)]
pub(super) struct SessionConnection {
    edges: Vec<SessionEdge>,
    page_info: PageEnd,
}

impl From<Page<Session>> for SessionConnection {
    fn from(page: Page<Session>) -> Self {
        let edges: Vec<_> = page
            .rows
            .into_iter()
            .map(|node| SessionEdge {
                cursor: encode_cursor(Cursor::of(&node)),
                node,
            })
            .collect();
        let end_cursor = edges.last().map(|edge| edge.cursor.clone());
        Self {
            edges,
            page_info: PageEnd {
                has_next_page: page.has_next_page,
                end_cursor,
            },
        }
    }
}

/// The checked arguments of `Query.sessions`.
pub(super) struct SessionsArgs {
    pub(super) filter: SessionFilter,
    pub(super) size: usize,
    pub(super) after: Option<Cursor>,
}

/// Reads the arguments, reporting every bad one at once.
pub(super) fn sessions_args(
    device_id: Option<&ID>,
    live: Option<bool>,
    first: i32,
    after: Option<&str>,
) -> Result<SessionsArgs, Vec<FieldError>> {
    let device = device_id
        .map(|id| {
            id.parse::<DeviceId>()
                .map_err(|_| FieldError::new("deviceId", "That device id is not valid."))
        })
        .transpose();
    match (
        device,
        page_size(first),
        after.map(decode_cursor).transpose(),
    ) {
        (Ok(device), Ok(size), Ok(after)) => Ok(SessionsArgs {
            filter: SessionFilter { device, live },
            size,
            after,
        }),
        (device, size, after) => Err(device
            .err()
            .into_iter()
            .chain(size.err())
            .chain(after.err())
            .collect()),
    }
}
