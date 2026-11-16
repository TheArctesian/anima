//! Which pull requests or issues a list asks GitHub for. Pure.
#![expect(
    clippy::unused_async_trait_impl,
    reason = "the async-graphql Enum derive generates async trait impls"
)]

use async_graphql::Enum;
use provider_github::live::StateFilter;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
pub enum OpenOrClosed {
    Open,
    Closed,
    All,
}

impl OpenOrClosed {
    /// The provider's filter for this choice.
    #[must_use]
    pub const fn filter(self) -> StateFilter {
        match self {
            Self::Open => StateFilter::Open,
            Self::Closed => StateFilter::Closed,
            Self::All => StateFilter::All,
        }
    }
}
