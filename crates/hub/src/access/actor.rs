//! Who drives a request: the actor kind the hub records for it (LOG-014, LOG-017), from the
//! token kind, what an API key is made for and the agent declaration (LOG-015, LOG-016).
//!
//! Pure: the request's signals are parameters. The declaration can only raise a request to
//! Agent; no input turns an agent key into User, and no request is Hub except one of Anima's
//! own programs (LOG-019).
#![expect(
    clippy::unused_async_trait_impl,
    reason = "the async-graphql Enum derive generates async trait impls"
)]

use async_graphql::Enum;
use axum::http::HeaderMap;

use super::tokens::TokenKind;
use crate::events::{self, ActorKind};

/// The header of the agent declaration on `POST /graphql` (K7, LOG-016).
pub const ACTOR_HEADER: &str = "anima-actor";

/// The field of the agent declaration in a `connection_init` payload (K7, LOG-016).
const ACTOR_FIELD: &str = "actor";

/// The one value that declares an AI agent, in any case (LOG-016).
const AGENT: &str = "agent";

/// What an API key is made for (LOG-015). K1 `MadeFor`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
pub enum MadeFor {
    /// A script that the operator runs.
    User,
    /// An AI agent.
    Agent,
}

impl MadeFor {
    /// The text stored in `api_tokens.made_for`.
    pub(super) const fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Agent => "agent",
        }
    }

    /// The value stored as `text`; `None` for anything else, and for a row with none.
    pub(super) fn from_stored(text: Option<&str>) -> Option<Self> {
        match text? {
            "user" => Some(Self::User),
            "agent" => Some(Self::Agent),
            _ => None,
        }
    }
}

/// Whether a request declared itself an AI agent (LOG-016). Any value but `agent` is no
/// declaration: it is neither an error nor logged, so it tells a caller nothing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Declaration(bool);

impl Declaration {
    /// The declaration of an HTTP request: exactly one `Anima-Actor` header whose value is
    /// `agent` in any case. Two headers, or a value that is not text, declare nothing.
    #[must_use]
    pub fn of_headers(headers: &HeaderMap) -> Self {
        let mut values = headers.get_all(ACTOR_HEADER).iter();
        let declared = match (values.next(), values.next()) {
            (Some(value), None) => value.to_str().is_ok_and(is_agent),
            _ => false,
        };
        Self(declared)
    }

    /// The declaration of a WebSocket connection: the `connection_init` payload's field
    /// `"actor": "agent"`, in any case. Any other value or type declares nothing.
    #[must_use]
    pub fn of_payload(payload: &serde_json::Value) -> Self {
        let declared = payload
            .get(ACTOR_FIELD)
            .and_then(serde_json::Value::as_str)
            .is_some_and(is_agent);
        Self(declared)
    }

    /// A request that declared itself an AI agent, for tests.
    #[cfg(test)]
    pub(super) const AGENT: Self = Self(true);
}

fn is_agent(value: &str) -> bool {
    value.trim().eq_ignore_ascii_case(AGENT)
}

/// What a request says about who drives it: the kind of its token, what its API key is made
/// for (`None` for every other kind) and its declaration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Signals {
    pub token: TokenKind,
    pub made_for: Option<MadeFor>,
    pub declared: Declaration,
}

/// The actor kind of an action (LOG-014), `None` when no request caused it:
///
/// 1. Hub, when no request caused it, or a builder token's request (LOG-019);
/// 2. Agent, when the key is made for an agent or the request declared an agent (LOG-017);
/// 3. User, for every other request.
#[must_use]
pub(super) fn actor_kind(request: Option<Signals>) -> ActorKind {
    let Some(request) = request else {
        return ActorKind::Hub;
    };
    if request.token == TokenKind::Builder {
        return ActorKind::Hub;
    }
    if request.made_for == Some(MadeFor::Agent) || request.declared.0 {
        return ActorKind::Agent;
    }
    ActorKind::User
}

/// The token kind the audit log records for `token` (LOG-018); `None` for a builder token,
/// whose requests are the hub's own (LOG-019).
pub(super) const fn recorded_token(token: TokenKind) -> Option<events::TokenKind> {
    match token {
        TokenKind::Login => Some(events::TokenKind::Login),
        TokenKind::Browser => Some(events::TokenKind::Browser),
        TokenKind::ApiKey => Some(events::TokenKind::ApiKey),
        TokenKind::Builder => None,
    }
}

/// The actor a request with `signals` is recorded as; `api_key_name` is kept for an API key.
pub(super) fn actor(signals: Signals, api_key_name: Option<String>) -> events::Actor {
    let kind = actor_kind(Some(signals));
    match recorded_token(signals.token) {
        Some(token) => events::Actor::request(kind, token, api_key_name),
        None => events::Actor::hub(),
    }
}

#[cfg(test)]
mod tests;
