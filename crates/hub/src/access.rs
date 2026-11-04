//! Who may use the hub: the signed-in operator and the GraphQL guard (API-003).

mod guard;
#[cfg(test)]
mod tests;

use async_graphql::ErrorExtensions as _;

pub use crate::events::{ClientAddr, Operator};
pub use guard::Authenticated;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Sign in to continue.")]
    Unauthenticated,
    /// The message stays generic: the source error may hold SQL text (API-010).
    #[error("Something went wrong on the hub.")]
    Database(#[from] sqlx::Error),
}

impl Error {
    /// The GraphQL `extensions.code` for this error (K7).
    const fn code(&self) -> &'static str {
        match self {
            Self::Unauthenticated => "UNAUTHENTICATED",
            Self::Database(_) => "INTERNAL",
        }
    }

    /// The GraphQL error the operator sees: the plain message and the K7 code, nothing else.
    fn to_graphql(&self) -> async_graphql::Error {
        let code = self.code();
        async_graphql::Error::new(self.to_string()).extend_with(|_, extensions| {
            extensions.set("code", code);
        })
    }
}
