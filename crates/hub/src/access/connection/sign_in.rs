//! A connection's `connection_init`: who it signs in as, from the payload's bearer token or the
//! upgrade's browser cookie (K7 "Authentication transport", D-3, WP-43 A-3).

use async_graphql::Data;
use axum::http::HeaderMap;
use tokio::sync::oneshot;
use zeroize::Zeroizing;

use super::{Lease, SocketAccess, Verdict};
use crate::access::{
    Declaration, Error, Identity, PublicOrigin, SameOrigin, browser,
    identity::{identify_connection, resolve},
    tokens::Transport,
};

/// The `__Host-anima` value of an upgrade from the public origin, kept until `connection_init`.
/// Wiped on drop; no `Debug`, so it cannot reach a log (LOG-007).
#[derive(Clone)]
pub(super) struct CookieToken(Zeroizing<String>);

impl CookieToken {
    /// The cookie of an upgrade with `headers`, when their one `Origin` is `origin` and they have
    /// no `Authorization` header (A-3, A-4).
    pub(super) fn of(headers: &HeaderMap, origin: &PublicOrigin) -> Option<Self> {
        let from = SameOrigin::of(headers, origin)?;
        let token = browser::cookie_token(headers, &from)?;
        Some(Self(Zeroizing::new(token.to_owned())))
    }
}

/// The `connection_init` handler: resolves the payload's bearer token, or the upgrade's cookie
/// when the payload has no `authorization`, never logging either, and reports the verdict to
/// the connection.
pub(super) async fn sign_in(
    access: SocketAccess,
    payload: serde_json::Value,
    verdict: oneshot::Sender<Verdict>,
) -> async_graphql::Result<Data> {
    let identified = identify_socket(&access, &payload).await;
    drop(payload);
    let (decision, data) = match identified {
        Ok(Some(identity)) => {
            let lease = Lease {
                token: identity.token(),
                expires_at: identity.expires_at(),
            };
            let mut data = identity.into_connection_data();
            if let Some(client) = access.client {
                data.insert(client);
            }
            (Verdict::Leased(lease), Ok(data))
        }
        Ok(None) => (Verdict::Refused, Err("Forbidden".into())),
        Err(error) => {
            error.log();
            (Verdict::Unavailable, Err("Unavailable".into()))
        }
    };
    // Fails only when the connection is gone, and then nobody waits for the verdict.
    let _ = verdict.send(decision);
    data
}

/// The identity of a connection: from the payload's `authorization` when it has one, with the
/// bearer rules; else from the kept cookie, which must hold a browser token (A-3).
async fn identify_socket(
    access: &SocketAccess,
    payload: &serde_json::Value,
) -> Result<Option<Identity>, Error> {
    let now = access.clock.now();
    match &access.cookie {
        Some(cookie) if payload.get("authorization").is_none() => {
            let identity = resolve(&access.db, &cookie.0, Transport::Cookie, now).await?;
            let declaration = Declaration::of_payload(payload);
            Ok(identity.map(|identity| identity.declared(declaration)))
        }
        _ => identify_connection(&access.db, payload, now).await,
    }
}
