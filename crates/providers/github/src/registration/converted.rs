//! Reading the conversion reply. Only the App's id, slug, owner and key are read: the client
//! secret and the webhook secret in the reply are never put into a value (PRJ-039).

use serde::Deserialize;
use source_exchange::{Converted, Owner, Problem};
use zeroize::Zeroizing;

use super::paths::{is_login, is_slug};
use crate::problem::problem;

#[derive(Deserialize)]
struct Reply {
    id: u64,
    slug: String,
    owner: Account,
    #[serde(deserialize_with = "wiping")]
    pem: Zeroizing<String>,
}

/// Reads the key straight into a value that wipes itself, also when the reply is refused.
fn wiping<'de, D: serde::Deserializer<'de>>(reader: D) -> Result<Zeroizing<String>, D::Error> {
    String::deserialize(reader).map(Zeroizing::new)
}

#[derive(Deserialize)]
struct Account {
    id: u64,
    login: String,
    #[serde(rename = "type")]
    kind: String,
}

/// Reads the `201` reply. The problem never quotes it: it holds a key (LOG-007).
pub(crate) fn parse_converted(body: &[u8]) -> Result<Converted, Problem> {
    serde_json::from_slice::<Reply>(body)
        .ok()
        .filter(|reply| {
            is_slug(&reply.slug) && is_login(&reply.owner.login) && !reply.pem.is_empty()
        })
        .map(|reply| Converted {
            app_id: reply.id,
            slug: reply.slug,
            owner: Owner {
                id: reply.owner.id,
                login: reply.owner.login,
                organisation: reply.owner.kind == "Organization",
            },
            private_key_pem: reply.pem,
        })
        .ok_or_else(|| {
            problem(
                "conversion_unreadable",
                "GitHub App",
                "GitHub's reply to the App conversion could not be read",
            )
        })
}
