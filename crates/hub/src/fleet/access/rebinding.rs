//! The link mode's guard against DNS rebinding (FRN-007, SEC-002): a web page whose name an
//! attacker points at 127.0.0.1 can reach the hub's loopback port, and its failed sign-ins
//! would fill the throttle of this computer's address (D-8). A browser always sends the name
//! it used in `Host`, so the hub answers only a loopback literal there, and an `Origin` only
//! from the desktop app's web view. No `Origin` is allowed: the command line and the daemon
//! send none, and a browser sends one with every cross-origin request.

use axum::{
    extract::Request,
    http::{
        HeaderMap, StatusCode,
        header::{HOST, ORIGIN},
    },
    middleware::{self, Next},
    response::{IntoResponse as _, Response},
    routing::Route,
};
use std::convert::Infallible;

use super::PublicOrigin;
use tower::{Layer, Service};

/// axum middleware for the link mode: refuses with a bare 403 a request whose host is not
/// `127.0.0.1`, `[::1]` or `localhost` (any port), or whose `Origin` is not one of `origins`.
/// The host is the `Host` header, or the URI's authority when there is none (HTTP/2); a
/// request with neither is refused.
///
/// With `gateway`, the public origin of a hosted hub (HMD-061), a request may also name the
/// host of that origin and come from that origin: the gateway of the pool passes a browser's
/// request with the name and the origin that the browser used. Only a hosted hub gives one: its
/// loopback port is reached only by the pool, never by a browser on the same computer, so the
/// rebinding that this guard stops cannot occur there.
#[must_use]
pub fn loopback_names_layer(
    origins: &'static [&'static str],
    gateway: Option<PublicOrigin>,
) -> impl Layer<
    Route,
    Service: Service<Request, Response = Response, Error = Infallible, Future: Send + 'static>
                 + Clone
                 + Send
                 + Sync
                 + 'static,
> + Clone
+ Send
+ Sync
+ 'static {
    middleware::from_fn(move |request: Request, next: Next| {
        let gateway = gateway.clone();
        async move {
            let host = request.headers().get(HOST).map_or_else(
                || request.uri().authority().map(|a| a.as_str().to_owned()),
                |value| value.to_str().ok().map(str::to_owned),
            );
            let gateway = gateway.as_ref().map(PublicOrigin::as_str);
            let named = host
                .as_deref()
                .is_some_and(|host| is_loopback_host(host) || is_host_of(host, gateway));
            if named && origin_permitted(request.headers(), origins, gateway) {
                next.run(request).await
            } else {
                StatusCode::FORBIDDEN.into_response()
            }
        }
    })
}

/// Whether every `Origin` header of the request is one of `origins` or `gateway`; true when
/// there is none.
pub(super) fn origin_permitted(
    headers: &HeaderMap,
    origins: &[&str],
    gateway: Option<&str>,
) -> bool {
    headers.get_all(ORIGIN).iter().all(|value| {
        value
            .to_str()
            .is_ok_and(|origin| origins.contains(&origin) || gateway == Some(origin))
    })
}

/// Whether `authority` is the host of the `https` origin `gateway`, with no port or with 443,
/// in any case of letters. False for no gateway and for an origin that is not `https`.
pub(super) fn is_host_of(authority: &str, gateway: Option<&str>) -> bool {
    let Some(host) = gateway.and_then(|origin| origin.strip_prefix("https://")) else {
        return false;
    };
    let named = authority.strip_suffix(":443").unwrap_or(authority);
    !host.is_empty() && named.eq_ignore_ascii_case(host)
}

/// Whether `authority` is `127.0.0.1`, `[::1]` or `localhost` (any case), with or without a
/// port of 1 to 5 digits. Any other name, address or form is refused.
pub(super) fn is_loopback_host(authority: &str) -> bool {
    let (host, port) = match authority.strip_prefix('[') {
        Some(bracketed) => match bracketed.split_once(']') {
            Some(("::1", rest)) => ("[::1]", rest),
            _ => return false,
        },
        // The port keeps its `:`, so `localhost:` (an empty port) is refused below.
        None => authority
            .find(':')
            .map_or((authority, ""), |at| authority.split_at(at)),
    };
    let port_ok = port.is_empty()
        || port.strip_prefix(':').is_some_and(|digits| {
            (1..=5).contains(&digits.len()) && digits.bytes().all(|b| b.is_ascii_digit())
        });
    port_ok && (host == "127.0.0.1" || host == "[::1]" || host.eq_ignore_ascii_case("localhost"))
}
