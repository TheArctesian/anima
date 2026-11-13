//! K1 `OpenIn`: mirrors the domain enum, which stays free of GraphQL.
#![expect(
    clippy::unused_async_trait_impl,
    reason = "the async-graphql Enum derive generates async trait impls"
)]

use async_graphql::Enum;

use crate::fleet::web_interfaces::OpenIn;

#[derive(Clone, Copy, Enum, PartialEq, Eq)]
#[graphql(remote = "OpenIn", name = "OpenIn")]
pub(super) enum OpenInValue {
    AnimaTab,
    SystemBrowser,
}
