//! WP-10 handoffs 5–7: the hub ends stale sessions before it serves, serves over TCP with the
//! socket peer as the client address, and on the stop signal ends its sessions with
//! `HUB_SHUTDOWN` and drains for a bounded time before serving returns (R-040, D-13, NET-002).

use std::{
    net::SocketAddr,
    time::{Duration, Instant},
};

use anima_hub::{
    Shutdown,
    access::{self, Edge, Password, listen},
    router,
    schema::build_schema,
    serve_until,
    sessions::{self, Live},
};
use serde_json::{Value, json};
use sqlx::PgPool;
use tokio::{
    io::{AsyncReadExt as _, AsyncWriteExt as _},
    net::TcpStream,
    sync::oneshot,
    task::JoinHandle,
};

use super::{
    OPERATOR_ID, PVE_01, accept_key, add_operator, audit_clients, clock, now, session_end, wired,
};
use crate::hub::{PASSWORD, USERNAME};

const STALE: &str = "0190a000-0000-7000-8010-000000000001";

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn r_040_opening_sessions_ends_a_row_a_previous_hub_left_live(pool: PgPool) {
    add_operator(&pool).await;
    sqlx::query(
        "insert into sessions.sessions (id, device_id, device_name, protocol, operator_id, \
                operator_name, client_addr, started_at) \
         values ($1::text::uuid, $2::text::uuid, 'pve-01', 'ssh', $3::text::uuid, 'operator', \
                 '10.10.0.77', $4 - interval '1 hour')",
    )
    .bind(STALE)
    .bind(PVE_01)
    .bind(OPERATOR_ID)
    .bind(now())
    .execute(&pool)
    .await
    .unwrap();

    // The schema needs the `Live` this returns, so nothing is served before it.
    let (live, ended) = sessions::open(&pool, now()).await.unwrap();
    drop(build_schema(wired(&pool, &live)));

    assert_eq!(ended, 1);
    assert_eq!(
        session_end(&pool, STALE).await,
        (true, Some("hub_shutdown".to_owned()))
    );
    assert_eq!(audit_clients(&pool, STALE, "end").await.len(), 1);
}

/// One HTTP/1.1 `POST /graphql` over a new connection to `addr`; returns the body as JSON.
async fn post(addr: SocketAddr, headers: &[(&str, &str)], body: &Value) -> Value {
    let body = body.to_string();
    let mut request = format!(
        "POST /graphql HTTP/1.1\r\nhost: hub\r\nconnection: close\r\n\
         content-type: application/json\r\ncontent-length: {}\r\n",
        body.len()
    );
    for (name, value) in headers {
        request.extend([name, ": ", value, "\r\n"]);
    }
    request.push_str("\r\n");
    request.push_str(&body);
    let mut stream = TcpStream::connect(addr).await.unwrap();
    stream.write_all(request.as_bytes()).await.unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).await.unwrap();
    let (head, body) = response.split_once("\r\n\r\n").unwrap();
    assert!(head.starts_with("HTTP/1.1 200"), "{head}");
    serde_json::from_str(body).unwrap()
}

async fn login(addr: SocketAddr) -> String {
    let query = "mutation($input: LoginInput!) { login(input: $input) { token } }";
    let input = json!({ "input": { "username": USERNAME, "password": PASSWORD } });
    let response = post(addr, &[], &json!({ "query": query, "variables": input })).await;
    response["data"]["login"]["token"]
        .as_str()
        .unwrap_or_else(|| panic!("login failed: {response}"))
        .to_owned()
}

/// Starts a session on PVE-01 as `token`, claiming another address in `X-Forwarded-For`.
async fn start_spoofed(addr: SocketAddr, token: &str) -> String {
    let query = "mutation($input: StartSessionInput!) { startSession(input: $input) { \
                 sessionId } }";
    let input = json!({ "input": { "deviceId": PVE_01, "protocol": "SSH",
                                   "display": { "widthPx": 1280, "heightPx": 800 } } });
    let bearer = format!("Bearer {token}");
    let headers = [
        ("authorization", bearer.as_str()),
        ("x-forwarded-for", "203.0.113.9"),
        ("x-real-ip", "203.0.113.9"),
        ("forwarded", "for=203.0.113.9"),
    ];
    let response = post(
        addr,
        &headers,
        &json!({ "query": query, "variables": input }),
    )
    .await;
    response["data"]["startSession"]["sessionId"]
        .as_str()
        .unwrap_or_else(|| panic!("start failed: {response}"))
        .to_owned()
}

async fn stored_client(pool: &PgPool, id: &str) -> String {
    sqlx::query_scalar("select host(client_addr) from sessions.sessions where id = $1::text::uuid")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap()
}

/// A hub serving on a free local port until the returned sender fires, then draining for at
/// most `drain_limit`.
struct Served {
    addr: SocketAddr,
    stop: oneshot::Sender<()>,
    serving: JoinHandle<()>,
}

async fn serve(pool: &PgPool, live: Live, drain_limit: Duration) -> Served {
    let schema = build_schema(wired(pool, &live));
    let any_port: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let listening = listen::listen(any_port, None, clock()).await.unwrap();
    let addr = listening.local_addr().unwrap();
    let edge = Edge::new(pool.clone(), anima_hub::events::Bus::new(16), clock());
    let (stop, signal) = oneshot::channel::<()>();
    let app = router(schema, pool.clone(), clock());
    let shutdown = Shutdown {
        db: pool.clone(),
        live,
        clock: clock(),
        drain_limit,
    };
    let serving = tokio::spawn(serve_until(
        listening.admitting(edge),
        app,
        shutdown,
        async {
            signal.await.ok();
        },
    ));
    Served {
        addr,
        stop,
        serving,
    }
}

impl Served {
    /// Fires the stop signal; returns how long serving took to return.
    async fn stop(self) -> Duration {
        let began = Instant::now();
        self.stop.send(()).unwrap();
        tokio::time::timeout(Duration::from_secs(10), self.serving)
            .await
            .expect("serving returns after the stop signal")
            .unwrap();
        began.elapsed()
    }
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn r_040_the_stop_signal_ends_a_pending_ticket_and_serving_returns(pool: PgPool) {
    access::set_operator(&pool, USERNAME, Password::new(PASSWORD.to_owned()), now())
        .await
        .unwrap();
    accept_key(&pool, PVE_01, USERNAME).await;
    let (live, _) = sessions::open(&pool, now()).await.unwrap();
    let hub = serve(&pool, live, Duration::from_secs(5)).await;

    let token = login(hub.addr).await;
    let session = start_spoofed(hub.addr, &token).await;
    // NET-002, LOG-005: the address is the socket peer, never a header.
    assert_eq!(stored_client(&pool, &session).await, "127.0.0.1");
    assert_eq!(
        audit_clients(&pool, &session, "start").await,
        vec!["127.0.0.1"]
    );
    assert_eq!(session_end(&pool, &session).await, (false, None));

    hub.stop().await;

    assert_eq!(
        session_end(&pool, &session).await,
        (true, Some("hub_shutdown".to_owned()))
    );
    assert_eq!(
        audit_clients(&pool, &session, "end").await,
        vec!["127.0.0.1"]
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn r_040_a_request_held_open_cannot_hold_the_drain_past_its_limit(pool: PgPool) {
    let limit = Duration::from_millis(300);
    let hub = serve(&pool, Live::default(), limit).await;
    let mut client = TcpStream::connect(hub.addr).await.unwrap();
    // A body announced at 1000 bytes, one sent, the connection kept open.
    let partial = "POST /graphql HTTP/1.1\r\nhost: hub\r\ncontent-type: application/json\r\n\
                   content-length: 1000\r\n\r\n{";
    client.write_all(partial.as_bytes()).await.unwrap();
    // Give the server time to accept the connection and read the request head.
    tokio::time::sleep(Duration::from_millis(200)).await;

    let took = hub.stop().await;

    // At least the limit: the drain waited for the open request, then gave up.
    assert!(
        took >= limit,
        "returned after {took:?}, before the drain limit"
    );
    drop(client);
}
