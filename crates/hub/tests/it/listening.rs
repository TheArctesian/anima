//! The hub binary's edge as `main.rs` wires it: the bind check stops start-up (NET-003,
//! SEC-008), and a hub served over TLS as a runtime login seeds `ANIMA_PERMITTED_NETWORKS`
//! and refuses other addresses (SEC-001, NET-001, NET-002). Ports are chosen by the system;
//! every spawn has a deadline and is killed when the test ends.

use std::{
    io::{BufRead as _, BufReader, Read as _, Write as _},
    net::SocketAddr,
    path::Path,
    process::{Child, Command, Stdio},
    sync::{Arc, mpsc},
    thread,
    time::{Duration, Instant},
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

const DEADLINE: Duration = Duration::from_secs(20);

/// A hub process, killed when dropped.
struct Running(Child);

impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn spawn(envs: &[(&str, &str)]) -> Running {
    let child = Command::new(env!("CARGO_BIN_EXE_anima-hub"))
        .env_clear()
        .env("ANIMA_LOG", "info")
        .envs(envs.iter().copied())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    Running(child)
}

/// The standard error of a hub that must stop by itself, and whether it succeeded.
fn refused_start(envs: &[(&str, &str)]) -> (bool, String) {
    let mut hub = spawn(envs);
    let started = Instant::now();
    let status = loop {
        if let Some(status) = hub.0.try_wait().unwrap() {
            break status;
        }
        assert!(started.elapsed() < DEADLINE, "the hub did not stop");
        thread::sleep(Duration::from_millis(50));
    };
    let mut stderr = String::new();
    hub.0
        .stderr
        .take()
        .unwrap()
        .read_to_string(&mut stderr)
        .unwrap();
    (status.success(), stderr)
}

#[test]
fn net_003_start_up_refuses_an_unspecified_or_public_listen_address() {
    let cases = [
        (
            "0.0.0.0:0",
            "ANIMA_LISTEN=0.0.0.0 is the unspecified address",
        ),
        ("[::]:0", "ANIMA_LISTEN=:: is the unspecified address"),
        (
            "8.8.8.8:0",
            "ANIMA_LISTEN=8.8.8.8 is not a loopback, private",
        ),
        (
            "[2001:db8::1]:0",
            "ANIMA_LISTEN=2001:db8::1 is not a loopback, private",
        ),
        (
            "10.0.0.1:0",
            "ANIMA_TLS_CERT_FILE and ANIMA_TLS_KEY_FILE must be set",
        ),
    ];
    for (listen, expected) in cases {
        let (ok, stderr) = refused_start(&[("ANIMA_LISTEN", listen)]);
        assert!(!ok, "{listen}");
        assert!(
            stderr.starts_with(&format!("anima-hub: {expected}")),
            "{listen}: {stderr}"
        );
    }
}

#[test]
fn sec_001_a_bad_key_file_stops_start_up_without_its_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let made = rcgen::generate_simple_self_signed(vec!["localhost".to_owned()]).unwrap();
    let cert = dir.path().join("cert.pem");
    let key = dir.path().join("key.pem");
    std::fs::write(&cert, made.cert.pem()).unwrap();
    let pem = made.signing_key.serialize_pem();
    let broken = format!("{}!{}", &pem[..60], &pem[60..]);
    std::fs::write(&key, &broken).unwrap();
    let (ok, stderr) = refused_start(&[
        ("ANIMA_LISTEN", "127.0.0.1:0"),
        ("ANIMA_TLS_CERT_FILE", cert.to_str().unwrap()),
        ("ANIMA_TLS_KEY_FILE", key.to_str().unwrap()),
    ]);
    assert!(!ok);
    assert_eq!(
        stderr.trim(),
        "anima-hub: the TLS certificate cannot be loaded: the key file holds no valid PEM \
         private key"
    );
    assert!(!stderr.contains(&pem[30..60]), "{stderr}");
}

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
    std::fs::write(dir.path().join("cert.pem"), made.cert.pem()).unwrap();
    std::fs::write(dir.path().join("key.pem"), made.signing_key.serialize_pem()).unwrap();
    std::fs::write(dir.path().join("database_url"), url).unwrap();
    let mut key = std::fs::File::create(dir.path().join("anima_key")).unwrap();
    write!(key, "{}", STANDARD.encode([5_u8; 32])).unwrap();
    Secrets {
        dir,
        der: made.cert.der().to_vec(),
    }
}

fn path_text(dir: &Path, name: &str) -> String {
    dir.join(name).to_str().unwrap().to_owned()
}

/// Starts a hub over TLS permitting `networks`; returns it and the address it listens on,
/// read from its `hub started` log line.
fn serve(secrets: &Secrets, networks: &str) -> (Running, SocketAddr) {
    let dir = secrets.dir.path();
    let files = [
        ("ANIMA_TLS_CERT_FILE", path_text(dir, "cert.pem")),
        ("ANIMA_TLS_KEY_FILE", path_text(dir, "key.pem")),
        ("ANIMA_DATABASE_URL_FILE", path_text(dir, "database_url")),
        ("ANIMA_KEY_FILE", path_text(dir, "anima_key")),
    ];
    let mut envs: Vec<(&str, &str)> = files.iter().map(|(k, v)| (*k, v.as_str())).collect();
    envs.extend([
        ("ANIMA_LISTEN", "127.0.0.1:0"),
        ("ANIMA_PERMITTED_NETWORKS", networks),
    ]);
    let mut hub = spawn(&envs);
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

    let (hub, addr) = serve(&secrets, "10.0.0.0/8");
    let refused = tokio::time::timeout(DEADLINE, post_over_tls(addr, &secrets.der))
        .await
        .expect("an answer in time");
    assert!(refused.starts_with("HTTP/1.1 403"), "{refused}");
    assert!(refused.contains(r#""code":"FORBIDDEN""#), "{refused}");
    drop(hub);
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
