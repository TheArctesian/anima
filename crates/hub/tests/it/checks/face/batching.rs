//! API-005: `statuses`, `uptimes` and `resolved_ips` cost the same few statements for 13
//! devices as for 163.
//!
//! A TCP proxy between the pool and PostgreSQL counts the client's Bind and simple Query
//! messages, as in the devices batching test.

use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use anima_hub::{
    checks::{self, UptimeWindow},
    ids::DeviceId,
};
use chrono::{DateTime, Utc};
use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions, PgSslMode},
};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    net::{TcpListener, TcpStream, UnixStream},
};

/// Splits the client's byte stream into PostgreSQL frontend messages and counts statements.
#[derive(Default)]
struct Frames {
    pending: Vec<u8>,
    started: bool,
}

impl Frames {
    /// Feeds bytes; returns how many statements (Bind or Query messages) they completed.
    fn feed(&mut self, bytes: &[u8]) -> usize {
        self.pending.extend_from_slice(bytes);
        let mut statements = 0;
        // The startup message has no type byte; every later message starts with one.
        loop {
            let skip = usize::from(self.started);
            let Some(length) = self.pending.get(skip..skip + 4) else {
                return statements;
            };
            let total = skip + u32::from_be_bytes(length.try_into().unwrap()) as usize;
            if self.pending.len() < total {
                return statements;
            }
            if self.started && matches!(self.pending[0], b'B' | b'Q') {
                statements += 1;
            }
            self.started = true;
            self.pending.drain(..total);
        }
    }
}

/// Where the real database listens: a TCP port or a Unix socket.
#[derive(Clone)]
enum Upstream {
    Tcp(String, u16),
    Unix(PathBuf),
}

trait Wire: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send> Wire for T {}

impl Upstream {
    async fn connect(&self) -> std::io::Result<Box<dyn Wire>> {
        Ok(match self {
            Self::Tcp(host, port) => Box::new(TcpStream::connect((host.as_str(), *port)).await?),
            Self::Unix(path) => Box::new(UnixStream::connect(path).await?),
        })
    }
}

async fn forward_and_count(client: TcpStream, upstream: Upstream, count: Arc<AtomicUsize>) {
    let Ok(server) = upstream.connect().await else {
        return;
    };
    let (mut from_client, mut to_client) = tokio::io::split(client);
    let (mut from_server, mut to_server) = tokio::io::split(server);
    let up = async {
        let (mut frames, mut buffer) = (Frames::default(), [0_u8; 8192]);
        while let Ok(read) = from_client.read(&mut buffer).await {
            if read == 0 {
                break;
            }
            count.fetch_add(frames.feed(&buffer[..read]), Ordering::SeqCst);
            if to_server.write_all(&buffer[..read]).await.is_err() {
                break;
            }
        }
    };
    let down = tokio::io::copy(&mut from_server, &mut to_client);
    tokio::select! { () = up => {}, _ = down => {} }
}

/// A one-connection pool on the same database whose statements are counted.
pub(in crate::checks) async fn counting_pool(hub: &PgPool) -> (PgPool, Arc<AtomicUsize>) {
    let real = hub.connect_options();
    let upstream = real.get_socket().map_or_else(
        || Upstream::Tcp(real.get_host().to_owned(), real.get_port()),
        |path| Upstream::Unix(path.join(format!(".s.PGSQL.{}", real.get_port()))),
    );
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let count = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&count);
    tokio::spawn(async move {
        while let Ok((client, _)) = listener.accept().await {
            tokio::spawn(forward_and_count(
                client,
                upstream.clone(),
                Arc::clone(&counter),
            ));
        }
    });
    let options = PgConnectOptions::new()
        .host("127.0.0.1")
        .port(port)
        .username(real.get_username())
        .database(real.get_database().unwrap_or("postgres"))
        .ssl_mode(PgSslMode::Disable);
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .unwrap();
    (pool, count)
}

/// 150 more devices on the home site, each with SSH settings and two checks.
pub(in crate::checks) async fn add_bulk(pool: &PgPool) {
    for sql in [
        "INSERT INTO devices.devices (id, site_id, name, host, kind, os, created_at, updated_at) \
         SELECT gen_random_uuid(), '0190a000-0000-7000-8001-000000000001', 'bulk-' || n, \
                'bulk-' || n || '.lan', 'Server', 'linux', now(), now() \
         FROM generate_series(1, 150) AS n",
        "INSERT INTO devices.ssh_settings (device_id, enabled, port, username, auth) \
         SELECT id, false, 22, 'operator', 'password' FROM devices.devices WHERE name LIKE 'bulk-%'",
        "INSERT INTO checks.checks (id, device_id, type, name, status) \
         SELECT gen_random_uuid(), d.id, t.type, t.type, 'down' FROM devices.devices d, \
                unnest(ARRAY['icmp', 'tcp']) AS t(type) WHERE d.name LIKE 'bulk-%'",
    ] {
        sqlx::query(sql).execute(pool).await.unwrap();
    }
}

async fn all_devices(pool: &PgPool) -> Vec<DeviceId> {
    sqlx::query_scalar("SELECT id FROM devices.devices")
        .fetch_all(pool)
        .await
        .unwrap()
}

/// The statements that `statuses`, `uptimes` and `resolved_ips` cost for the devices.
async fn costs(pool: &PgPool, count: &AtomicUsize, devices: &[DeviceId]) -> [usize; 3] {
    let now: DateTime<Utc> = "2026-11-04T09:00:00Z".parse().unwrap();
    count.store(0, Ordering::SeqCst);
    let shown = checks::statuses(pool, devices, now).await.unwrap();
    assert_eq!(shown.len(), devices.len());
    let statuses = count.swap(0, Ordering::SeqCst);
    checks::uptimes(pool, devices, UptimeWindow::Week, now)
        .await
        .unwrap();
    let uptimes = count.swap(0, Ordering::SeqCst);
    checks::resolved_ips(pool, devices).await.unwrap();
    [statuses, uptimes, count.swap(0, Ordering::SeqCst)]
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(
        path = "../../../fixtures",
        scripts("devices", "checks", "maintenance")
    )
)]
async fn api_005_status_uptime_and_addresses_cost_constant_statements_for_13_or_163_devices(
    pool: PgPool,
) {
    let (counted, count) = counting_pool(&pool).await;
    let few = all_devices(&pool).await;
    costs(&counted, &count, &few).await;
    let few_cost = costs(&counted, &count, &few).await;

    add_bulk(&pool).await;
    let many = all_devices(&pool).await;
    let many_cost = costs(&counted, &count, &many).await;

    assert_eq!((few.len(), many.len()), (13, 163));
    assert_eq!(few_cost, many_cost);
    // One statement for the checks, one for the devices and one for the windows; one each
    // for the uptime sums and the addresses.
    assert_eq!(many_cost, [3, 1, 1]);
}
