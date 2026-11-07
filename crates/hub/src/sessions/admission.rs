//! What a tunnel request must pass before its session task starts, shared by `/tunnel` and
//! `/terminal` (K4 Sections 1, 2 and 8.1, API-009): the lazy sweep of expired tickets
//! (D-2), the token from the query, the client address and the path, the single-use
//! redemption, and the subprotocol the WebSocket upgrade must offer.
//!
//! Neither the URI, the query nor the token reaches a log, a span or an error (LOG-007).

use axum::{
    extract::{WebSocketUpgrade, ws::rejection::WebSocketUpgradeRejection},
    http::{Extensions, StatusCode},
};
use chrono::{DateTime, Utc};
use sqlx::PgPool;

use super::{Live, Presented, Protocol, Redeemed, end::sweep};
use crate::access;

/// Largest WebSocket message and frame from the client, on either tunnel (WP-08 H-3, K4
/// Section 8.5).
const MAX_MESSAGE_BYTES: usize = 64 * 1024;

/// Sweeps expired tickets, so the 403 below never leaves an expired row live (D-2).
///
/// # Errors
/// 503, before any token is read, when the sweep cannot run.
pub(super) async fn swept(db: &PgPool, live: &Live, now: DateTime<Utc>) -> Result<(), StatusCode> {
    sweep(db, live, now).await.map_err(|error| {
        tracing::error!(%error, "the tunnel could not sweep expired tickets");
        StatusCode::SERVICE_UNAVAILABLE
    })
}

/// Redeems the query's token for the client address on `path` at `now`; `tunnel_path`
/// says where each protocol's tickets go. A presented token is spent whatever happens next,
/// valid or not (K4 Section 1), so callers redeem before they check the subprotocol: a
/// request without it still spends the token, and cannot be used to test a token without
/// spending it.
///
/// # Errors
/// 403, with no detail, for every refusal: no token, no client address, or `Live::redeem`
/// refused it (a ticket on another tunnel's path included).
pub(super) fn redeem(
    live: &Live,
    request: Request<'_>,
    path: &str,
    tunnel_path: fn(Protocol) -> &'static str,
) -> Result<Redeemed, StatusCode> {
    let Request {
        query,
        extensions,
        now,
    } = request;
    let (Some(token), Some(client)) = (token_in(query), access::client_addr(extensions)) else {
        return Err(StatusCode::FORBIDDEN);
    };
    let presented = Presented {
        token,
        client,
        path,
        tunnel_path,
    };
    live.redeem(presented, now)
        .map_err(|_| StatusCode::FORBIDDEN)
}

/// What a tunnel request brings to its redemption: the raw query, the request's extensions
/// (the client address) and when it came.
#[derive(Clone, Copy)]
pub(super) struct Request<'a> {
    pub(super) query: &'a str,
    pub(super) extensions: &'a Extensions,
    pub(super) now: DateTime<Utc>,
}

/// The value of the first `token=` pair. No decoding: a token is base64url, so a
/// percent-encoded one is not canonical and is refused like any unknown token.
pub(super) fn token_in(query: &str) -> Option<&str> {
    query
        .split('&')
        .find_map(|pair| pair.strip_prefix("token="))
}

/// The upgrade, when it is one and offers `subprotocol`, with the size caps of K4 Section 8.5.
pub(super) fn offering(
    upgrade: Result<WebSocketUpgrade, WebSocketUpgradeRejection>,
    subprotocol: &'static str,
) -> Option<WebSocketUpgrade> {
    let upgrade = upgrade.ok()?.protocols([subprotocol]);
    upgrade.selected_protocol()?;
    Some(
        upgrade
            .max_message_size(MAX_MESSAGE_BYTES)
            .max_frame_size(MAX_MESSAGE_BYTES),
    )
}
