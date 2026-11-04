//! The NET-002 edge: every request from outside the permitted networks is refused before any
//! route runs, and every request that passes carries its [`ClientAddr`] (D-10, D-11).

use std::{
    convert::Infallible,
    net::{IpAddr, SocketAddr},
    sync::{Arc, Mutex, MutexGuard, PoisonError},
};

use async_graphql_axum::GraphQLResponse;
use axum::{
    extract::{ConnectInfo, Request, State},
    http::StatusCode,
    middleware::{self, Next},
    response::{IntoResponse as _, Response},
    routing::Route,
};
use chrono::{DateTime, Utc};
use ipnet::IpNet;
use sqlx::PgPool;
use tower::{Layer, Service};

use super::{
    ClientAddr, Error,
    networks::{self, Loaded, Refusals},
    store,
};
use crate::{
    clock::Clock,
    events::{self, Bus, NewEvent, Severity, Source},
};

/// What the layer keeps between requests.
#[derive(Clone)]
struct Gate {
    db: PgPool,
    bus: Bus,
    clock: Arc<dyn Clock>,
    loaded: Arc<Mutex<Option<Loaded>>>,
    refusals: Arc<Mutex<Refusals>>,
}

/// axum middleware: refuses a client address outside the permitted networks with 403 and
/// writes the refusal to the event log (source `ACCESS`, severity `WARNING`), at most once a
/// minute per address (NET-002, D-11). The address is the TCP peer from
/// `ConnectInfo<SocketAddr>`, never a header; an IPv4-mapped IPv6 peer counts as IPv4. A
/// request that passes gets that [`ClientAddr`] in its extensions.
///
/// The list is `access.permitted_networks`, read again after 30 seconds (D-10). Without a
/// peer address, or when the list cannot be read, the request is refused (fail closed).
pub fn permitted_networks_layer(
    db: PgPool,
    bus: Bus,
    clock: Arc<dyn Clock>,
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
    let gate = Gate {
        db,
        bus,
        clock,
        loaded: Arc::default(),
        refusals: Arc::default(),
    };
    middleware::from_fn_with_state(gate, admit)
}

async fn admit(State(gate): State<Gate>, mut request: Request, next: Next) -> Response {
    let Some(ConnectInfo(peer)) = request.extensions().get::<ConnectInfo<SocketAddr>>() else {
        let error = Error::Unavailable;
        error.log();
        return refusal(StatusCode::INTERNAL_SERVER_ERROR, &error);
    };
    let client = ClientAddr(networks::canonical(peer.ip()));
    let now = gate.clock.now();
    let permitted = match gate.networks(now).await {
        Ok(permitted) => permitted,
        Err(error) => {
            error.log();
            return refusal(StatusCode::SERVICE_UNAVAILABLE, &error);
        }
    };
    if !networks::permits(&permitted, client.0) {
        gate.refused(client.0, now).await;
        return refusal(StatusCode::FORBIDDEN, &Error::NotPermitted);
    }
    request.extensions_mut().insert(client);
    next.run(request).await
}

impl Gate {
    /// The permitted networks at `now`: the kept list while it is fresh, else read again.
    async fn networks(&self, now: DateTime<Utc>) -> Result<Arc<[IpNet]>, Error> {
        let kept = lock(&self.loaded)
            .as_ref()
            .filter(|loaded| loaded.fresh_at(now))
            .map(|loaded| Arc::clone(&loaded.networks));
        if let Some(networks) = kept {
            return Ok(networks);
        }
        let read = Loaded::new(store::permitted_networks(&self.db).await?, now);
        let networks = Arc::clone(&read.networks);
        *lock(&self.loaded) = Some(read);
        Ok(networks)
    }

    /// Writes the refusal of `client` to the event log unless the limit says it was written
    /// recently. A write that fails is logged by kind; the request is refused either way.
    async fn refused(&self, client: IpAddr, now: DateTime<Utc>) {
        if !lock(&self.refusals).admit(client, now) {
            return;
        }
        let event = NewEvent {
            severity: Severity::Warning,
            source: Source::Access,
            device: None,
            message: format!(
                "Refused a request from {client}: the address is not in a permitted network."
            ),
        };
        if let Err(error) = self.record(now, event).await {
            error.log();
        }
    }

    async fn record(&self, now: DateTime<Utc>, event: NewEvent) -> Result<(), Error> {
        let mut conn = self.db.acquire().await?;
        events::record(&mut conn, &self.bus, now, event).await?;
        Ok(())
    }
}

/// The plain message and K7 code of `error`, as a GraphQL error body with `status`.
fn refusal(status: StatusCode, error: &Error) -> Response {
    let body = async_graphql::Response::from_errors(vec![error.to_server_error()]);
    let mut response = GraphQLResponse::from(body).into_response();
    *response.status_mut() = status;
    response
}

/// The guarded value; a panic elsewhere while holding the lock leaves plain data, still valid.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
