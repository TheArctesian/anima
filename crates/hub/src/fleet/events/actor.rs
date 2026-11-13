//! Who did a recorded action and how the request signed in (LOG-012, LOG-018).
#![expect(
    clippy::unused_async_trait_impl,
    reason = "the async-graphql Enum derive generates async trait impls"
)]

use async_graphql::Enum;

/// Who did a recorded action (LOG-012). K1 `ActorKind`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Enum)]
pub enum ActorKind {
    /// A person: the operator in the client, at the command line or through a script.
    User,
    /// An AI agent.
    Agent,
    /// The code of Anima, which no person and no AI agent asked.
    Hub,
}

impl ActorKind {
    /// The text stored in `actor_kind`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Agent => "agent",
            Self::Hub => "hub",
        }
    }

    /// The kind whose stored text is `text`.
    #[must_use]
    pub fn from_stored(text: &str) -> Option<Self> {
        [Self::User, Self::Agent, Self::Hub]
            .into_iter()
            .find(|kind| kind.as_str() == text)
    }
}

/// The kind of token a request signed in with, as the audit log records it (LOG-018). K1
/// `TokenKind`. A builder token has none: its requests are the hub's own (LOG-019).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Enum)]
pub enum TokenKind {
    Login,
    Browser,
    ApiKey,
}

impl TokenKind {
    /// The text stored in `audit_log.token_kind`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Login => "login",
            Self::Browser => "browser",
            Self::ApiKey => "api_key",
        }
    }

    /// The kind whose stored text is `text`.
    #[must_use]
    pub fn from_stored(text: &str) -> Option<Self> {
        [Self::Login, Self::Browser, Self::ApiKey]
            .into_iter()
            .find(|kind| kind.as_str() == text)
    }
}

/// Who did an action, and the token a request signed in with (LOG-012, LOG-018). It never
/// holds the token or a part of it (LOG-007): only its kind and an API key's name. A Hub actor
/// has neither.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Actor {
    kind: ActorKind,
    token: Option<TokenKind>,
    // Boxed, so an operator stays small in the session ends that carry one.
    api_key_name: Option<Box<str>>,
}

impl Actor {
    /// The hub itself: no token and no key name.
    #[must_use]
    pub const fn hub() -> Self {
        Self {
            kind: ActorKind::Hub,
            token: None,
            api_key_name: None,
        }
    }

    /// A request of `kind` signed in with `token`; `api_key_name` is kept only for an API key.
    /// A `Hub` kind gives [`Actor::hub`], which has no token.
    #[must_use]
    pub fn request(kind: ActorKind, token: TokenKind, api_key_name: Option<String>) -> Self {
        if kind == ActorKind::Hub {
            return Self::hub();
        }
        Self {
            kind,
            token: Some(token),
            api_key_name: api_key_name
                .filter(|_| token == TokenKind::ApiKey)
                .map(String::into_boxed_str),
        }
    }

    /// The operator at a login token (User, LOGIN), for tests. Only with the `test-support`
    /// feature, as `access::Identity::for_tests`.
    #[cfg(feature = "test-support")]
    #[must_use]
    pub const fn for_tests() -> Self {
        Self {
            kind: ActorKind::User,
            token: Some(TokenKind::Login),
            api_key_name: None,
        }
    }

    #[must_use]
    pub const fn kind(&self) -> ActorKind {
        self.kind
    }

    #[must_use]
    pub const fn token(&self) -> Option<TokenKind> {
        self.token
    }

    #[must_use]
    pub fn api_key_name(&self) -> Option<&str> {
        self.api_key_name.as_deref()
    }
}
