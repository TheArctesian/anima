//! The way in of the hub's own daemon in the hosted mode (HMD-071, task HM-T35 of
//! hosted-mode.md; K8 Section 1): a private socket whose connections reach only the routes of
//! K8, each as from this computer, which is the hub's own device (HMD-045).

use std::net::{Ipv4Addr, SocketAddr};

use axum::{
    Router,
    extract::Request,
    http::StatusCode,
    middleware::{self, Next},
    response::{IntoResponse as _, Response},
};

use super::{FromLink, Holding, Serving};

/// The peer of every request of the hub's own daemon.
const THIS_COMPUTER: SocketAddr = SocketAddr::new(std::net::IpAddr::V4(Ipv4Addr::LOCALHOST), 0);

/// The start of every route of K8.
const DAEMON_ROUTES: &str = "/daemon/v1/";

/// `router` with each request off the routes of K8 answered 404 before it reaches a route.
pub(super) fn daemon_routes_only(router: Router) -> Router {
    router.layer(middleware::from_fn(only_daemon_routes))
}

async fn only_daemon_routes(request: Request, next: Next) -> Response {
    if request.uri().path().starts_with(DAEMON_ROUTES) {
        next.run(request).await
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}

impl Serving {
    /// The work of serving `incoming` from the socket of the hub's own daemon. A connection
    /// that another user's process made is closed with no answer; so is one for which the
    /// places of this computer are all taken.
    pub(in super::super) fn admit_own_daemon(
        self: &std::sync::Arc<Self>,
        incoming: FromLink,
    ) -> impl Future<Output = ()> + Send + 'static {
        let serving = std::sync::Arc::clone(self);
        async move {
            let Some(stream) = incoming.own() else {
                return;
            };
            let Some(slot) = serving.link_connections.take(THIS_COMPUTER.ip()) else {
                return;
            };
            let stall = serving.limits.write_stall;
            let router = serving.own_daemon_router.clone();
            let io = Holding::new(stream, slot, stall);
            serving.http_with(router, io, THIS_COMPUTER).await;
        }
    }
}
