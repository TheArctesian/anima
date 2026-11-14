//! Minting an installation token: the path to post to and how to read the reply (SRC-002).

use chrono::{DateTime, Utc};
use serde::Deserialize;
use source_exchange::{Minted, Problem};

pub(crate) fn mint_path(installation: &str) -> String {
    format!("/app/installations/{installation}/access_tokens")
}

#[derive(Deserialize)]
struct Reply {
    token: String,
    expires_at: DateTime<Utc>,
}

/// Reads the `201` reply. The problem never quotes the reply: it holds a token (LOG-007).
pub(crate) fn parse_minted(body: &[u8]) -> Result<Minted, Problem> {
    serde_json::from_slice::<Reply>(body)
        .ok()
        .filter(|reply| !reply.token.is_empty())
        .map(|reply| Minted {
            token: reply.token,
            expires_at: reply.expires_at,
        })
        .ok_or_else(|| Problem {
            code: "token_unreadable",
            subject: "installation token".to_owned(),
            message: "GitHub's reply to the token request could not be read".to_owned(),
        })
}
