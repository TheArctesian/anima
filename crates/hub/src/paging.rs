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

#[cfg(test)]
mod tests {
    use async_graphql::{EmptyMutation, EmptySubscription, Object, Schema};

    use super::PageEnd;

    struct Query;

    #[Object]
    impl Query {
        async fn page(&self) -> PageEnd {
            PageEnd {
                has_next_page: true,
                end_cursor: Some("c1".to_owned()),
            }
        }
    }

    #[test]
    fn api_006_page_info_has_exactly_the_k1_fields() {
        let sdl = Schema::new(Query, EmptyMutation, EmptySubscription).sdl();
        let block = sdl
            .split("type PageInfo {")
            .nth(1)
            .and_then(|rest| rest.split('}').next())
            .expect("PageInfo type");
        let fields: Vec<&str> = block
            .lines()
            .map(str::trim)
            // Field lines are `name: Type`; description lines have no bare `name:` prefix.
            .filter(|line| {
                line.split_once(':')
                    .is_some_and(|(name, _)| !name.is_empty() && !name.contains(' '))
            })
            .collect();
        assert_eq!(fields, ["hasNextPage: Boolean!", "endCursor: String"]);
    }
}
