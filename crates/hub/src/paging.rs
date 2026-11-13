//! Cursor pagination shared by every list in the API (API-006, K1, R-014).
//!
//! K1's `PageInfo` has only `hasNextPage` and `endCursor`. async-graphql's built-in connection
//! type adds backward paging fields that K1 does not have, so domains use this type instead.
//! Cursors stay opaque and each domain encodes its own.

use async_graphql::SimpleObject;

/// Where a page ends and whether another page follows. Exposed as K1's `PageInfo`; the Rust
/// name avoids the banned word "info" (CQ-030).
#[derive(Clone, Debug, Default, PartialEq, Eq, SimpleObject)]
#[graphql(name = "PageInfo")]
pub struct PageEnd {
    pub has_next_page: bool,
    /// The cursor of the last edge on this page; `None` when the page is empty.
    pub end_cursor: Option<String>,
}
