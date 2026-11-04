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
    http::{Method, StatusCode},
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
    networks::{self, Loaded, REFUSALS_PER_SPAN, Refusals},
    store,
};
use crate::{
    clock::Clock,
    events::{self, Bus, NewEvent, Severity, Source},
};

/// The permitted networks and the refusals written, shared by the accept-time check of the
/// listener and the layer, so both give one answer and write each refusal once (NET-002).
#[derive(Clone)]
pub struct Edge {
    db: PgPool,
    bus: Bus,
    clock: Arc<dyn Clock>,
    loaded: Arc<Mutex<Option<Loaded>>>,
    refusals: Arc<Mutex<Refusals>>,
}

/// The events a refusal writes, decided under the D-11 limit when the client was refused:
/// written with [`Edge::write`] after the client is answered or closed.
#[must_use]
pub(super) struct Refusal {
    now: DateTime<Utc>,
    messages: Vec<String>,
}

/// Whether a client address may use the hub.
pub(super) enum Verdict {
    Permitted,
    /// Outside the permitted networks, or a container or proxy address; the events to write.
    Refused(Refusal),
    /// The list cannot be read.
    Unavailable(Error),
}

impl Edge {
    #[must_use]
    pub fn new(db: PgPool, bus: Bus, clock: Arc<dyn Clock>) -> Self {
        Self {
            db,
            bus,
            clock,
            loaded: Arc::default(),
            refusals: Arc::default(),
        }
    }
}

/// axum middleware: refuses a client address outside the permitted networks of `edge` with 403
/// and writes the refusal to the event log (source `ACCESS`, severity `WARNING`), at most once
/// a minute per address (NET-002, D-11). The address is the TCP peer from
/// `ConnectInfo<SocketAddr>`, never a header; an IPv4-mapped IPv6 peer counts as IPv4. A
/// request that passes gets that [`ClientAddr`] in its extensions.
///
/// The list is `access.permitted_networks`, read again after 30 seconds (D-10). Without a
/// peer address, or when the list cannot be read, the request is refused (fail closed). A
/// peer in `172.16.0.0/12` is refused whatever the list says: in the deployment that is a
/// container or Docker's proxy, never the real client. Only `GET /health` passes unchecked.
pub fn permitted_networks_layer(
    edge: Edge,
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
    middleware::from_fn_with_state(edge, admit)
}

async fn admit(State(edge): State<Edge>, mut request: Request, next: Next) -> Response {
    let Some(ConnectInfo(peer)) = request.extensions().get::<ConnectInfo<SocketAddr>>() else {
        let error = Error::Unavailable;
        error.log();
        return refusal(StatusCode::INTERNAL_SERVER_ERROR, &error);
    };
    let client = ClientAddr(networks::canonical(peer.ip()));
    if is_liveness(&request) {
        return next.run(request).await;
    }
    match edge.verdict(client.0, REQUEST).await {
        Verdict::Permitted => {}
        Verdict::Refused(written) => {
            edge.write(written).await;
            return refusal(StatusCode::FORBIDDEN, &Error::NotPermitted);
        }
        Verdict::Unavailable(error) => {
            error.log();
            return refusal(StatusCode::SERVICE_UNAVAILABLE, &error);
        }
    }
    request.extensions_mut().insert(client);
    next.run(request).await
}

/// What was refused, in "Refused a {what} from {address}: {why}".
pub(super) const REQUEST: &str = "request";
pub(super) const CONNECTION: &str = "connection";

/// Why a refusal event was written.
const OUTSIDE: &str = "the address is not in a permitted network.";
const UNTRUSTED: &str =
    "a container or proxy address (172.16.0.0/12) is never taken as a client address.";

/// `GET /health`, the token-free liveness answer the compose healthcheck reads from inside the
/// container, whose own address is a container address. It passes without a client address
/// and so reaches no route that needs one; it holds no data.
fn is_liveness(request: &Request) -> bool {
    request.method() == Method::GET && request.uri().path() == "/health"
}

impl Edge {
    /// Whether `client` may use the hub now. A refusal is noted as the refusal of a `what`
    /// ([`REQUEST`] or [`CONNECTION`]) under the D-11 limit.
    pub(super) async fn verdict(&self, client: IpAddr, what: &str) -> Verdict {
        let now = self.clock.now();
        if networks::untrusted(client) {
            let message = format!("Refused a {what} from {client}: {UNTRUSTED}");
            return Verdict::Refused(self.refused(client, now, message));
        }
        match self.networks(now).await {
            Ok(permitted) if networks::permits(&permitted, client) => Verdict::Permitted,
            Ok(_) => {
                let message = format!("Refused a {what} from {client}: {OUTSIDE}");
                Verdict::Refused(self.refused(client, now, message))
            }
            Err(error) => Verdict::Unavailable(error),
        }
    }

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

    /// The events for refusing `client` at `now`: `message`, unless the limit says it was
    /// written recently, and once a minute how many refusals the limit left out.
    fn refused(&self, client: IpAddr, now: DateTime<Utc>, message: String) -> Refusal {
        let mut refusals = lock(&self.refusals);
        let mut messages = Vec::new();
        if refusals.admit(client, now) {
            messages.push(message);
        }
        if let Some(count) = refusals.unrecorded(now) {
            messages.push(format!(
                "{count} more refusals were not recorded: at most {REFUSALS_PER_SPAN} are \
                 recorded a minute."
            ));
        }
        Refusal { now, messages }
    }

    /// Writes the events of `refusal`. A write that fails is logged by kind; the client is
    /// refused either way.
    pub(super) async fn write(&self, refusal: Refusal) {
        for message in refusal.messages {
            let event = NewEvent {
                severity: Severity::Warning,
                source: Source::Access,
                device: None,
                message,
            };
            if let Err(error) = self.record(refusal.now, event).await {
                error.log();
            }
        }
    }

    async fn record(&self, now: DateTime<Utc>, event: NewEvent) -> Result<(), Error> {
        let mut conn = self.db.acquire().await?;
        events::record(&mut conn, &self.bus, now, event).await?;
        Ok(())
    }
}

#[cfg(test)]
impl Edge {
    /// An edge whose list is `networks` for as long as `clock` stands still, with no database:
    /// refusal events fail to write at once and are logged by kind.
    pub(super) fn permitting(networks: &[IpNet], clock: Arc<dyn Clock>) -> Self {
        let db = sqlx::postgres::PgPoolOptions::new()
            .acquire_timeout(std::time::Duration::from_secs(1))
            .connect_lazy("postgres://127.0.0.1:1/none")
            .expect("a lazy pool");
        let now = clock.now();
        let edge = Self::new(db, Bus::new(1), clock);
        *lock(&edge.loaded) = Some(Loaded::new(networks.iter().copied(), now));
        edge
    }
}

#[cfg(test)]
impl Refusal {
    /// The events this refusal writes.
    pub(super) fn messages(&self) -> &[String] {
        &self.messages
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
