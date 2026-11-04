//! A hub served over TLS from one `tls.pem` as a runtime login: it seeds
//! `ANIMA_PERMITTED_NETWORKS`, refuses other addresses, and `anima-hub health` finds it
//! (SEC-001, NET-001, NET-002, WP-04 H-2 and the healthcheck).

use std::{
    io::{BufRead as _, BufReader, Read as _, Write as _},
    net::SocketAddr,
    path::Path,
    sync::{Arc, mpsc},
    thread,
    time::Instant,
};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use sqlx::PgPool;
use tokio::{
    io::{AsyncReadExt as _, AsyncWriteExt as _},
    net::TcpStream,
};
use tokio_rustls::{
    TlsConnector,
    rustls::{
        ClientConfig, RootCertStore,
        crypto::ring,
        pki_types::{CertificateDer, ServerName},
        version::TLS13,
    },
};

use super::{DEADLINE, Running, finished, spawn};

/// A LOGIN role in `anima_runtime`, as the deployment's hub login is (R-016).
async fn runtime_login(pool: &PgPool) -> String {
    let role = format!("anima_test_{}_edge", uuid::Uuid::now_v7().simple());
    sqlx::query(&format!("create role {role} login in role anima_runtime"))
        .execute(pool)
        .await
        .unwrap();
    role
}

/// `DATABASE_URL` with its user replaced by `role` and its database by `database`.
fn url_as(role: &str, database: &str) -> String {
    let url = std::env::var("DATABASE_URL").expect("tests run with DATABASE_URL (R-009)");
    let (scheme, rest) = url.split_once("://").unwrap();
    let host = rest.split_once('@').map_or(rest, |(_, host)| host);
    let (base, query) = host
        .split_once('?')
        .map_or((host, None), |(b, q)| (b, Some(q)));
    let server = base.split_once('/').map_or(base, |(server, _)| server);
    let query = query.map(|q| format!("?{q}")).unwrap_or_default();
    format!("{scheme}://{role}@{server}/{database}{query}")
}

/// The files a served hub reads, in one temporary folder.
struct Secrets {
    dir: tempfile::TempDir,
    der: Vec<u8>,
}

fn secrets(url: &str) -> Secrets {
    let dir = tempfile::tempdir().unwrap();
    let made = rcgen::generate_simple_self_signed(vec!["localhost".to_owned()]).unwrap();
    let tls_pem = format!("{}{}", made.signing_key.serialize_pem(), made.cert.pem());
    std::fs::write(dir.path().join("tls.pem"), tls_pem).unwrap();
    std::fs::write(dir.path().join("database_url"), url).unwrap();
    let mut key = std::fs::File::create(dir.path().join("anima_key")).unwrap();
    write!(key, "{}", STANDARD.encode([5_u8; 32])).unwrap();
    Secrets {
        dir,
        der: made.cert.der().to_vec(),
    }
}

/// Both TLS variables name the one `tls.pem`, as compose sets them (WP-04).
fn tls_env(secrets: &Secrets) -> [(&'static str, String); 2] {
    let tls_pem = path_text(secrets.dir.path(), "tls.pem");
    [
        ("ANIMA_TLS_CERT_FILE", tls_pem.clone()),
        ("ANIMA_TLS_KEY_FILE", tls_pem),
    ]
}

/// Whether `anima-hub health` finds the hub on `addr` healthy, and its standard error.
fn health(secrets: &Secrets, addr: SocketAddr) -> (bool, String) {
    let listen = addr.to_string();
    let tls = tls_env(secrets);
    let mut envs: Vec<(&str, &str)> = tls.iter().map(|(k, v)| (*k, v.as_str())).collect();
    envs.push(("ANIMA_LISTEN", &listen));
    finished(spawn(&["health"], &envs))
}

fn path_text(dir: &Path, name: &str) -> String {
    dir.join(name).to_str().unwrap().to_owned()
}

/// Starts a hub over TLS permitting `networks`; returns it and the address it listens on,
/// read from its `hub started` log line.
fn serve(secrets: &Secrets, networks: &str) -> (Running, SocketAddr) {
    let dir = secrets.dir.path();
    let files = [
        ("ANIMA_DATABASE_URL_FILE", path_text(dir, "database_url")),
        ("ANIMA_KEY_FILE", path_text(dir, "anima_key")),
    ];
    let mut envs: Vec<(&str, &str)> = files.iter().map(|(k, v)| (*k, v.as_str())).collect();
    let tls = tls_env(secrets);
    envs.extend(tls.iter().map(|(k, v)| (*k, v.as_str())));
    envs.extend([
        ("ANIMA_LISTEN", "127.0.0.1:0"),
        ("ANIMA_PERMITTED_NETWORKS", networks),
    ]);
    let mut hub = spawn(&[], &envs);
    let stdout = hub.0.stdout.take().unwrap();
    let (lines, received) = mpsc::channel();
    thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            let _ = lines.send(line);
        }
    });
    let started = Instant::now();
    loop {
        let left = DEADLINE.saturating_sub(started.elapsed());
        let line = received.recv_timeout(left).unwrap_or_else(|_| {
            let mut stderr = String::new();
            let _ = hub.0.stderr.take().unwrap().read_to_string(&mut stderr);
            panic!("the hub did not start: {stderr}");
        });
        if line.contains("hub started") {
            let line: serde_json::Value = serde_json::from_str(&line).unwrap();
            let listen = line["fields"]["listen"].as_str().unwrap().parse().unwrap();
            return (hub, listen);
        }
    }
}

/// The status line and body of `POST /graphql { __typename }` over TLS, trusting `der`.
async fn post_over_tls(addr: SocketAddr, der: &[u8]) -> String {
    let mut roots = RootCertStore::empty();
    roots.add(CertificateDer::from(der.to_vec())).unwrap();
    let config = ClientConfig::builder_with_provider(Arc::new(ring::default_provider()))
        .with_protocol_versions(&[&TLS13])
        .unwrap()
        .with_root_certificates(roots)
        .with_no_client_auth();
    let tcp = TcpStream::connect(addr).await.unwrap();
    let name = ServerName::try_from("localhost").unwrap();
    let mut tls = TlsConnector::from(Arc::new(config))
        .connect(name, tcp)
        .await
        .unwrap();
    let body = r#"{"query":"{ __typename }"}"#;
    let request = format!(
        "POST /graphql HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\n\
         Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    tls.write_all(request.as_bytes()).await.unwrap();
    let mut response = String::new();
    tls.read_to_string(&mut response).await.unwrap();
    response
}

async fn configured_networks(pool: &PgPool) -> Vec<String> {
    sqlx::query_scalar(
        "select cidr::text from access.permitted_networks \
         where label = 'ANIMA_PERMITTED_NETWORKS' order by cidr",
    )
    .fetch_all(pool)
    .await
    .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn net_002_sec_001_the_served_hub_refuses_outside_networks_over_tls(pool: PgPool) {
    let database = pool.connect_options().get_database().unwrap().to_owned();
    let role = runtime_login(&pool).await;
    let secrets = secrets(&url_as(&role, &database));

    // The peer is the hub's own listen address, as for the compose healthcheck run inside the
    // container (WP-04): it passes the accept-time check (which closes 172.16/12 and outside
    // peers, `access::listen::tests::edge`), reaches `GET /health` and gets 403 on `/graphql`.
    let (hub, addr) = serve(&secrets, "10.0.0.0/8");
    let refused = tokio::time::timeout(DEADLINE, post_over_tls(addr, &secrets.der))
        .await
        .expect("an answer in time");
    assert!(refused.starts_with("HTTP/1.1 403"), "{refused}");
    assert!(refused.contains(r#""code":"FORBIDDEN""#), "{refused}");
    assert_eq!(
        health(&secrets, addr),
        (true, String::new()),
        "health needs no network"
    );
    drop(hub);
    let (healthy, stderr) = health(&secrets, addr);
    assert!(!healthy);
    assert!(
        stderr.starts_with("anima-hub: the hub is not healthy: "),
        "{stderr}"
    );
    assert!(!stderr.contains("PRIVATE"), "{stderr}");
    assert_eq!(configured_networks(&pool).await, ["10.0.0.0/8"]);
    let refusals: Vec<String> = sqlx::query_scalar(
        "select message from events.event_log where source = 'access' order by at",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        refusals,
        ["Refused a request from 127.0.0.1: the address is not in a permitted network."]
    );

    let (hub, addr) = serve(&secrets, "127.0.0.0/8");
    let passed = tokio::time::timeout(DEADLINE, post_over_tls(addr, &secrets.der))
        .await
        .expect("an answer in time");
    assert!(passed.starts_with("HTTP/1.1 200"), "{passed}");
    assert!(
        passed.ends_with(r#"{"data":{"__typename":"Query"}}"#),
        "{passed}"
    );
    drop(hub);
    assert_eq!(configured_networks(&pool).await, ["127.0.0.0/8"]);
    sqlx::query(&format!("drop role {role}"))
        .execute(&pool)
        .await
        .unwrap();
}
