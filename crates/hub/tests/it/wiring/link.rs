//! The hub in the link mode, as `main.rs` wires it (one-click plan, item S5): `anima-link`
//! reaches it through a Unix socket and names each device by its link address. The rules that
//! name a client then name a device: the permitted clients (NET-001, NET-002), the builder's
//! binding (BLD-024), the agent routes' refusal log (K8) and the audit log (LOG-005).

use std::{
    collections::BTreeMap,
    net::{Ipv6Addr, SocketAddr},
    os::unix::fs::PermissionsExt as _,
    path::PathBuf,
    sync::Arc,
    time::Duration,
};

use anima_hub::{
    Shutdown, all_routes,
    clock::{Clock, FixedClock},
    config,
    fleet::{
        access::{self, Edge, listen},
        events::Bus,
        sessions::Live,
    },
    schema::Dependencies,
    serve_until,
};
use serde_json::{Value, json};
use sqlx::PgPool;
use tokio::{
    io::{AsyncRead, AsyncReadExt as _, AsyncWrite, AsyncWriteExt as _},
    net::{TcpStream, UnixStream},
    task::JoinHandle,
};

use crate::hub::{PASSWORD, USERNAME, dependencies, set_operator, start};

/// The header `anima-link` writes, from the link's own test data; its source address is
/// replaced to name another device.
const HEADER: &[u8; 52] = include_bytes!("../../../../../link/hublink/testdata/device-header.bin");

const LAPTOP: &str = "fd41:4e49:4d41:aaaa::1";
const DESK: &str = "fd41:4e49:4d41:bbbb::2";

fn header_of(device: &str) -> Vec<u8> {
    let device: Ipv6Addr = device.parse().unwrap();
    let mut header = HEADER.to_vec();
    header[16..32].copy_from_slice(&device.octets());
    header
}

/// A hub in the link mode on a free loopback port and a socket in a private folder.
struct Linked {
    addr: SocketAddr,
    socket: PathBuf,
    _folder: tempfile::TempDir,
    serving: JoinHandle<()>,
}

impl Drop for Linked {
    fn drop(&mut self) {
        self.serving.abort();
    }
}

async fn serve_linked(pool: &PgPool) -> Linked {
    let folder = tempfile::tempdir().unwrap();
    std::fs::set_permissions(folder.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let socket = folder.path().join("link.sock");
    let env = BTreeMap::from([
        ("ANIMA_LISTEN".to_owned(), "127.0.0.1:0".to_owned()),
        (
            "ANIMA_LINK_SOCKET".to_owned(),
            socket.to_str().unwrap().to_owned(),
        ),
    ]);
    let config = Arc::new(config::from_env(&env).unwrap());
    access::permit_networks(pool, &config.permitted_networks)
        .await
        .unwrap();
    let clock: Arc<dyn Clock> = Arc::new(FixedClock(start()));
    let bus = Bus::new(16);
    let listening = listen::listen(config.listen, None, clock.clone())
        .await
        .unwrap();
    let addr = listening.local_addr().unwrap();
    let link = listen::link_socket(config.link_socket.as_deref().unwrap()).unwrap();
    let edge = Edge::new(pool.clone(), bus.clone(), clock.clone());
    let wired = Dependencies {
        config,
        ..dependencies(pool, clock.clone(), bus)
    };
    let shutdown = Shutdown {
        db: pool.clone(),
        live: Live::default(),
        clock,
        drain_limit: Duration::ZERO,
    };
    let entrance = listening.admitting(edge.clone()).with_link(Some(link));
    let app = all_routes(wired, edge);
    let serving = tokio::spawn(serve_until(entrance, app, shutdown, std::future::pending()));
    Linked {
        addr,
        socket,
        _folder: folder,
        serving,
    }
}

/// Where a request comes from.
#[derive(Clone, Copy)]
enum From<'a> {
    /// A device of the link, by its link address.
    Device(&'a str),
    /// A program on the hub's computer, through the loopback port.
    ThisComputer,
    /// The link's socket with no header.
    Unnamed,
}

/// A `POST` to `path` with a JSON `body`, signed with `token` if any.
#[derive(Clone, Copy)]
struct Post<'a> {
    path: &'a str,
    token: Option<&'a str>,
    body: &'a str,
}

/// Sends `request` from `from`; returns the whole response as text, empty when the hub closed
/// the connection with no answer.
async fn post(hub: &Linked, from: From<'_>, request: Post<'_>) -> String {
    let Post { path, token, body } = request;
    let mut request = format!(
        "POST {path} HTTP/1.1\r\nhost: hub\r\nconnection: close\r\n\
         content-type: application/json\r\ncontent-length: {}\r\n",
        body.len()
    );
    if let Some(token) = token {
        request.extend(["authorization: Bearer ", token, "\r\n"]);
    }
    request.extend(["\r\n", body]);
    let first = match from {
        From::Device(device) => header_of(device),
        From::ThisComputer | From::Unnamed => Vec::new(),
    };
    let sent = [first.as_slice(), request.as_bytes()].concat();
    let exchange = async {
        if matches!(from, From::ThisComputer) {
            answered(TcpStream::connect(hub.addr).await.unwrap(), &sent).await
        } else {
            answered(UnixStream::connect(&hub.socket).await.unwrap(), &sent).await
        }
    };
    tokio::time::timeout(Duration::from_secs(20), exchange)
        .await
        .expect("the hub must answer or close")
}

async fn answered(mut stream: impl AsyncRead + AsyncWrite + Unpin, sent: &[u8]) -> String {
    // The hub may close before everything is written.
    let _ = stream.write_all(sent).await;
    let mut response = Vec::new();
    let _ = stream.read_to_end(&mut response).await;
    String::from_utf8_lossy(&response).into_owned()
}

/// A GraphQL request; returns the response body as JSON.
async fn graphql(hub: &Linked, from: From<'_>, token: Option<&str>, request: Value) -> Value {
    let body = request.to_string();
    let request = Post {
        path: "/graphql",
        token,
        body: &body,
    };
    let response = post(hub, from, request).await;
    let (head, body) = response
        .split_once("\r\n\r\n")
        .unwrap_or_else(|| panic!("no answer: {response:?}"));
    assert!(head.starts_with("HTTP/1.1 200"), "{head}");
    serde_json::from_str(body).unwrap()
}

async fn login(hub: &Linked, from: From<'_>) -> String {
    let query = "mutation($input: LoginInput!) { login(input: $input) { token } }";
    let input = json!({ "input": { "username": USERNAME, "password": PASSWORD } });
    let response = graphql(
        hub,
        from,
        None,
        json!({ "query": query, "variables": input }),
    )
    .await;
    response["data"]["login"]["token"]
        .as_str()
        .unwrap_or_else(|| panic!("login failed: {response}"))
        .to_owned()
}

/// The messages of the event log, oldest first, once `expected` of them are written: a
/// refusal is written after its connection is closed.
async fn events(pool: &PgPool, expected: usize) -> Vec<String> {
    for _ in 0..100 {
        let messages: Vec<String> =
            sqlx::query_scalar("select message from events.event_log order by at, id")
                .fetch_all(pool)
                .await
                .unwrap();
        if messages.len() >= expected {
            return messages;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("the event log did not get {expected} events");
}

#[sqlx::test(migrations = "./migrations")]
async fn net_002_the_links_devices_and_this_computer_sign_in_and_the_audit_names_each(
    pool: PgPool,
) {
    set_operator(&pool).await;
    let hub = serve_linked(&pool).await;

    login(&hub, From::Device(LAPTOP)).await;
    login(&hub, From::Device(DESK)).await;
    login(&hub, From::ThisComputer).await;

    let clients: Vec<String> = sqlx::query_scalar(
        "select host(client_addr) from events.audit_log where action = 'login' order by at, id",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(clients, [LAPTOP, DESK, "127.0.0.1"]);
}

#[sqlx::test(migrations = "./migrations")]
async fn net_002_a_socket_connection_that_names_no_device_gets_no_answer_and_is_logged(
    pool: PgPool,
) {
    set_operator(&pool).await;
    let hub = serve_linked(&pool).await;
    let body = json!({ "query": "{ __typename }" }).to_string();
    let request = Post {
        path: "/graphql",
        token: None,
        body: &body,
    };

    let unnamed = post(&hub, From::Unnamed, request).await;
    assert_eq!(unnamed, "");
    // An address that is not a link address cannot be named through the socket: here the
    // hub's own loopback address, which the permitted networks hold.
    let as_loopback = post(&hub, From::Device("::ffff:127.0.0.1"), request).await;
    assert_eq!(as_loopback, "");

    assert_eq!(
        events(&pool, 1).await,
        [
            "Refused a connection on the link's socket: it did not start with the header that \
          names a device of the link."
        ]
    );
    let named = post(&hub, From::Device(LAPTOP), request).await;
    assert!(named.starts_with("HTTP/1.1 200"), "{named}");
}

const LINUX: &str = "{ os: LINUX, arch: X86_64 }";

async fn claim(hub: &Linked, from: From<'_>, builder: &str) -> Value {
    let query = format!("mutation {{ claimBuildJob(platform: {LINUX}) {{ state }} }}");
    graphql(hub, from, Some(builder), json!({ "query": query })).await
}

#[sqlx::test(migrations = "./migrations")]
async fn bld_024_a_builder_token_works_only_from_the_device_that_enrolled_it(pool: PgPool) {
    set_operator(&pool).await;
    let hub = serve_linked(&pool).await;
    let laptop = From::Device(LAPTOP);
    let operator = login(&hub, laptop).await;
    let create = "mutation { createRelease(version: \"0.1.12\", \
                  commit: \"0123456789abcdef0123456789abcdef01234567\", \
                  targets: [LINUX_X86_64]) { id } }";
    let created = graphql(&hub, laptop, Some(&operator), json!({ "query": create })).await;
    assert!(created["errors"].is_null(), "{created}");
    let enrol = format!(
        "mutation {{ enrolBuilder(name: \"laptop\", platform: {LINUX}) \
         {{ token builder {{ clientAddr }} }} }}"
    );
    let enrolled = graphql(&hub, laptop, Some(&operator), json!({ "query": enrol })).await;
    let enrolled = &enrolled["data"]["enrolBuilder"];
    assert_eq!(enrolled["builder"]["clientAddr"], LAPTOP, "{enrolled}");
    let builder = enrolled["token"].as_str().unwrap();

    for elsewhere in [From::Device(DESK), From::ThisComputer] {
        let refused = claim(&hub, elsewhere, builder).await;
        let code = &refused["errors"][0]["extensions"]["code"];
        assert_eq!(code, "FORBIDDEN", "{refused}");
    }
    let claimed = claim(&hub, laptop, builder).await;
    assert_eq!(
        claimed["data"]["claimBuildJob"]["state"], "CLAIMED",
        "{claimed}"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn k8_an_agent_refusal_is_logged_for_each_device_of_the_link(pool: PgPool) {
    set_operator(&pool).await;
    let hub = serve_linked(&pool).await;
    // The spelling of an agent token that no agent holds.
    let unknown = format!("anima_agent_{}", "A".repeat(43));

    for device in [LAPTOP, LAPTOP, DESK] {
        let push = Post {
            path: "/agent/v1/push",
            token: Some(&unknown),
            body: "{}",
        };
        let refused = post(&hub, From::Device(device), push).await;
        assert!(refused.starts_with("HTTP/1.1 4"), "{device}: {refused}");
    }

    // Once a minute for each client: one event for the laptop's two refusals, one for the desk.
    let written = events(&pool, 2).await;
    assert_eq!(written.len(), 2, "{written:?}");
    assert!(
        written[0].contains(&format!("from {LAPTOP}:")),
        "{written:?}"
    );
    assert!(written[1].contains(&format!("from {DESK}:")), "{written:?}");
}
