//! The cookie on the `GET /graphql` WebSocket upgrade: a socket from the public origin may sign
//! in with an empty `connection_init`, any other origin is refused with 4403, and the payload's
//! bearer still signs the desktop app in (D-3, K7, A-3).

use std::{
    net::SocketAddr,
    sync::Arc,
    time::{Duration, Instant},
};

use anima_hub::{
    access::{self, PublicOrigin},
    clock::{Clock, FixedClock},
    events::Bus,
    schema::build_schema,
};
use async_graphql::futures_util::{SinkExt as _, StreamExt as _};
use axum::{Router, extract::WebSocketUpgrade, http::HeaderMap, routing::get};
use chrono::{DateTime, TimeDelta, Utc};
use serde_json::{Value, json};
use sqlx::PgPool;
use tokio::{net::TcpStream, task::JoinHandle};
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, connect_async,
    tungstenite::{Message, client::IntoClientRequest as _, http::HeaderValue},
};

use super::{PUBLIC, browser_hub, browser_token, cookie, hash};
use crate::hub::{Hub, set_operator, start};

/// The longest any socket step may take.
const DEADLINE: Duration = Duration::from_secs(10);

/// A server on a loopback port; dropping it stops the server.
pub(crate) struct Served {
    pub(crate) addr: SocketAddr,
    task: JoinHandle<()>,
}

impl Drop for Served {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn serve(app: Router) -> Served {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let service = app.into_make_service_with_connect_info::<SocketAddr>();
    let task = tokio::spawn(async move { axum::serve(listener, service).await.unwrap() });
    Served { addr, task }
}

pub(crate) type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// Opens a `graphql-transport-ws` socket to `/graphql` with the extra upgrade `headers`.
pub(crate) async fn open(addr: SocketAddr, headers: &[(&'static str, &str)]) -> Socket {
    let mut request = format!("ws://{addr}/graphql")
        .into_client_request()
        .unwrap();
    let sent = request.headers_mut();
    sent.insert(
        "sec-websocket-protocol",
        HeaderValue::from_static("graphql-transport-ws"),
    );
    for (name, value) in headers {
        sent.append(*name, HeaderValue::from_str(value).unwrap());
    }
    let connected = tokio::time::timeout(DEADLINE, connect_async(request))
        .await
        .unwrap();
    connected.unwrap().0
}

/// What the hub sent next: a JSON message, or the close code (`None` without a frame).
pub(crate) async fn receive(socket: &mut Socket) -> Result<Value, Option<u16>> {
    loop {
        match tokio::time::timeout(DEADLINE, socket.next()).await.unwrap() {
            Some(Ok(Message::Text(text))) => return Ok(serde_json::from_str(&text).unwrap()),
            Some(Ok(Message::Close(frame))) => return Err(frame.map(|f| u16::from(f.code))),
            Some(Ok(_)) => {}
            Some(Err(_)) | None => return Err(None),
        }
    }
}

/// Sends `connection_init` with `payload` (none when null); returns the answer.
pub(crate) async fn init(socket: &mut Socket, payload: Value) -> Result<Value, Option<u16>> {
    let message = if payload.is_null() {
        json!({ "type": "connection_init" })
    } else {
        json!({ "type": "connection_init", "payload": payload })
    };
    socket
        .send(Message::text(message.to_string()))
        .await
        .unwrap();
    receive(socket).await
}

pub(crate) fn ack() -> Value {
    json!({ "type": "connection_ack" })
}

pub(crate) const FORBIDDEN: Result<Value, Option<u16>> = Err(Some(4403));

#[sqlx::test(migrations = "./migrations")]
async fn d_3_a_socket_from_the_public_origin_signs_in_by_cookie(pool: PgPool) {
    set_operator(&pool).await;
    let hub = browser_hub(&pool, start());
    let cookie = cookie(&browser_token(&hub).await);
    let served = serve(hub).await;

    for payload in [Value::Null, json!({})] {
        let mut socket = open(served.addr, &[("origin", PUBLIC), ("cookie", &cookie)]).await;
        assert_eq!(
            init(&mut socket, payload.clone()).await,
            Ok(ack()),
            "{payload}"
        );
        let me = json!({ "id": "1", "type": "subscribe", "payload": { "query": "{ me { username } }" } });
        socket.send(Message::text(me.to_string())).await.unwrap();
        let next = receive(&mut socket).await.unwrap();
        assert_eq!(
            next["payload"]["data"]["me"]["username"], "operator",
            "{next}"
        );
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn d_3_a_socket_from_another_origin_ignores_the_cookie(pool: PgPool) {
    set_operator(&pool).await;
    let hub = browser_hub(&pool, start());
    let cookie = cookie(&browser_token(&hub).await);
    let served = serve(hub).await;
    let refusals: [&[(&'static str, &str)]; 5] = [
        &[("cookie", &cookie)],
        &[("cookie", &cookie), ("origin", "https://evil.example")],
        &[("cookie", &cookie), ("origin", "null")],
        &[("cookie", &cookie), ("origin", PUBLIC), ("origin", PUBLIC)],
        &[
            ("cookie", &cookie),
            ("origin", PUBLIC),
            ("authorization", "Bearer junk"),
        ],
    ];
    for headers in refusals {
        let mut socket = open(served.addr, headers).await;
        assert_eq!(
            init(&mut socket, Value::Null).await,
            FORBIDDEN,
            "{headers:?}"
        );
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn a_3_on_a_socket_the_payload_bearer_wins_and_must_not_be_a_browser_token(pool: PgPool) {
    set_operator(&pool).await;
    let login = Hub::new(&pool, start()).token().await;
    let hub = browser_hub(&pool, start());
    let browser = browser_token(&hub).await;
    let served = serve(hub).await;
    let bearer = |token: &str| json!({ "authorization": format!("Bearer {token}") });

    let mut desktop = open(served.addr, &[]).await;
    assert_eq!(
        init(&mut desktop, bearer(&login)).await,
        Ok(ack()),
        "desktop path"
    );
    let mut tauri = open(served.addr, &[("origin", "tauri://localhost")]).await;
    assert_eq!(
        init(&mut tauri, bearer(&login)).await,
        Ok(ack()),
        "web view"
    );

    let mut stolen = open(served.addr, &[]).await;
    assert_eq!(
        init(&mut stolen, bearer(&browser)).await,
        FORBIDDEN,
        "browser token as bearer"
    );
    let cookie = cookie(&browser);
    let mut junk = open(served.addr, &[("origin", PUBLIC), ("cookie", &cookie)]).await;
    let payload = json!({ "authorization": "Bearer junk" });
    assert_eq!(
        init(&mut junk, payload).await,
        FORBIDDEN,
        "the payload decides"
    );
    let login_cookie = super::cookie(&login);
    let mut wrong_kind = open(
        served.addr,
        &[("origin", PUBLIC), ("cookie", &login_cookie)],
    )
    .await;
    assert_eq!(
        init(&mut wrong_kind, Value::Null).await,
        FORBIDDEN,
        "login token as cookie"
    );
}

/// A clock that runs from `base` with real time.
struct Running {
    base: DateTime<Utc>,
    started: Instant,
}

impl Clock for Running {
    fn now(&self) -> DateTime<Utc> {
        self.base + TimeDelta::from_std(self.started.elapsed()).unwrap()
    }
}

/// Only the WebSocket route at the public origin, re-checking every 100 ms.
pub(crate) async fn serve_watched(pool: &PgPool, clock: Arc<dyn Clock>) -> Served {
    let schema = build_schema(crate::hub::dependencies(pool, clock.clone(), Bus::new(16)));
    let access = access::SocketAccess::new(pool.clone(), clock, None)
        .with_timings(Duration::from_millis(100), DEADLINE);
    let origin = PublicOrigin::new(PUBLIC.to_owned());
    let route = get(move |headers: HeaderMap, upgrade: WebSocketUpgrade| {
        let access = access.clone().with_upgrade(&headers, &origin);
        let schema = schema.clone();
        async move { access::accept_socket(upgrade, schema, access) }
    });
    serve(Router::new().route("/graphql", route)).await
}

#[sqlx::test(migrations = "./migrations")]
async fn d_3_a_cookie_socket_closes_when_its_token_is_deleted(pool: PgPool) {
    set_operator(&pool).await;
    let token = browser_token(&browser_hub(&pool, start())).await;
    let clock: Arc<dyn Clock> = Arc::new(FixedClock(start()));
    let served = serve_watched(&pool, clock).await;
    let mut socket = open(
        served.addr,
        &[("origin", PUBLIC), ("cookie", &cookie(&token))],
    )
    .await;
    assert_eq!(init(&mut socket, Value::Null).await, Ok(ack()));

    sqlx::query("delete from access.api_tokens where token_hash = $1")
        .bind(hash(&token))
        .execute(&pool)
        .await
        .unwrap();

    assert_eq!(receive(&mut socket).await, FORBIDDEN);
}

#[sqlx::test(migrations = "./migrations")]
async fn d_3_a_cookie_socket_closes_at_the_token_expiry(pool: PgPool) {
    set_operator(&pool).await;
    let token = browser_token(&browser_hub(&pool, start())).await;
    let base = start() + TimeDelta::days(7) - TimeDelta::seconds(2);
    let clock: Arc<dyn Clock> = Arc::new(Running {
        base,
        started: Instant::now(),
    });
    let served = serve_watched(&pool, clock).await;
    let mut socket = open(
        served.addr,
        &[("origin", PUBLIC), ("cookie", &cookie(&token))],
    )
    .await;
    assert_eq!(init(&mut socket, Value::Null).await, Ok(ack()));

    assert_eq!(receive(&mut socket).await, FORBIDDEN);
}
