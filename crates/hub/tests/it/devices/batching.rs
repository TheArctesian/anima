//! API-005: a nested selection costs the same number of SQL statements however many devices it returns.
//!
//! The test counts statements at the wire: a TCP proxy between the hub's pool and PostgreSQL
//! counts the client's Bind and simple Query messages. That is exact, needs no database
//! privileges and does not wait for statistics to flush.

use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions, PgSslMode},
};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    net::{TcpListener, TcpStream, UnixStream},
};

use super::graphql_read::{data, schema_for};

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

/// A pool on the same database whose statements are counted.
async fn counting_pool(hub: &PgPool) -> (PgPool, Arc<AtomicUsize>) {
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
    let pool = PgPoolOptions::new().connect_with(options).await.unwrap();
    (pool, count)
}

const BATCHED_AT_MOST: usize = 6;
const NESTED: &str =
    "{ sites { name devices(first: 200) { totalCount edges { node { name site { name } } } } } }";

fn rendered_devices(result: &serde_json::Value) -> usize {
    result["sites"]
        .as_array()
        .unwrap()
        .iter()
        .map(|site| site["devices"]["edges"].as_array().unwrap().len())
        .sum()
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn api_005_nested_site_device_site_costs_a_constant_few_statements_for_13_or_113_devices(
    pool: PgPool,
) {
    let (counted, statements) = counting_pool(&pool).await;
    let schema = schema_for(&counted);

    let few = data(&schema, NESTED).await;
    let few_statements = statements.swap(0, Ordering::SeqCst);

    sqlx::query(
        "INSERT INTO devices.devices (id, site_id, name, host, kind, os, created_at, updated_at) \
         SELECT gen_random_uuid(), (SELECT id FROM devices.sites ORDER BY name OFFSET g % 3 LIMIT 1), \
                'bulk-' || g, 'bulk-' || g || '.lan', 'server', 'linux', now(), now() \
         FROM generate_series(1, 100) g",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO devices.ssh_settings (device_id, enabled, port, username, auth) \
         SELECT id, false, 22, '', 'private_key' FROM devices.devices WHERE name LIKE 'bulk-%'",
    )
    .execute(&pool)
    .await
    .unwrap();

    let many = data(&schema, NESTED).await;
    let many_statements = statements.load(Ordering::SeqCst);

    assert_eq!((rendered_devices(&few), rendered_devices(&many)), (13, 113));
    // Three when every key reaches its loader inside one 1 ms batch window: the sites, one batch
    // of devices for all sites, one batch of sites for all devices. Under heavy load a window can
    // split, so the bound allows one split per level. Without batching, 13 devices alone would
    // cost 17 statements and 113 devices 117.
    assert!(
        few_statements <= BATCHED_AT_MOST && many_statements <= BATCHED_AT_MOST,
        "{few_statements} statements for 13 devices, {many_statements} for 113"
    );
}
