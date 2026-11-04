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
    time::Duration,
};

use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions, PgSslMode},
};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    net::{TcpListener, TcpStream, UnixStream},
};

use anima_hub::{
    clock::{Clock, FixedClock},
    devices::graphql::{Mutation, Query, credential_loader, devices_by_site_loader, site_loader},
    events::Bus,
};
use async_graphql::{EmptySubscription, Schema};
use serde_json::{Value, json};

use super::{
    credential::{PVE_01, store_password},
    graphql_read::add_bulk,
    harness::{TestSchema, data, now, vault},
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
pub(super) async fn counting_pool(hub: &PgPool) -> (PgPool, Arc<AtomicUsize>) {
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

const BATCHED_AT_MOST: usize = 12;
pub(super) const SITES: i32 = 30;
pub(super) const BATCH_WINDOW: Duration = Duration::from_millis(20);
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
async fn api_005_nested_site_device_site_costs_a_constant_few_statements_for_13_or_163_devices(
    pool: PgPool,
) {
    let (counted, statements) = counting_pool(&pool).await;
    let schema = Schema::build(Query, Mutation, EmptySubscription)
        .data(counted.clone())
        .data(site_loader(counted.clone()).delay(BATCH_WINDOW))
        .data(devices_by_site_loader(counted.clone()).delay(BATCH_WINDOW))
        .finish();

    let few = data(&schema, NESTED).await;
    let few_statements = statements.swap(0, Ordering::SeqCst);

    add_bulk(&pool, SITES, 150).await;

    let many = data(&schema, NESTED).await;
    let many_statements = statements.load(Ordering::SeqCst);

    assert_eq!((rendered_devices(&few), rendered_devices(&many)), (13, 163));
    // The batched cost is 3: the sites, one batch of devices for all sites, one batch of sites
    // for all devices. A loader that queries once per key costs about one statement per site, so
    // at least 33 on this data. The test loaders wait 20 ms, not the default 1 ms, so a loaded
    // machine cannot split a batch; the bound sits far below the per-key cost so it does not
    // depend on timing.
    assert!(
        few_statements <= BATCHED_AT_MOST && many_statements <= BATCHED_AT_MOST,
        "{few_statements} statements for 13 devices, {many_statements} for 163"
    );
}

const STORED: &str = "{ devices(first: 200) { edges { node { name ssh { credentialStored } } } } }";

/// The names of the devices whose `credentialStored` is true, and how many devices there are.
fn stored_flags(result: &serde_json::Value) -> (Vec<&str>, usize) {
    let edges = result["devices"]["edges"].as_array().unwrap();
    let stored = edges
        .iter()
        .filter(|edge| edge["node"]["ssh"]["credentialStored"] == true)
        .map(|edge| edge["node"]["name"].as_str().unwrap())
        .collect();
    (stored, edges.len())
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn api_005_credential_stored_costs_a_constant_few_statements_for_13_or_163_devices(
    pool: PgPool,
) {
    let (counted, statements) = counting_pool(&pool).await;
    let schema = Schema::build(Query, Mutation, EmptySubscription)
        .data(counted.clone())
        .data(credential_loader(counted.clone()).delay(BATCH_WINDOW))
        .finish();
    store_password(&pool, PVE_01, "a-stored-password").await;

    let few = data(&schema, STORED).await;
    let few_statements = statements.swap(0, Ordering::SeqCst);

    add_bulk(&pool, SITES, 150).await;

    let many = data(&schema, STORED).await;
    let many_statements = statements.load(Ordering::SeqCst);

    assert_eq!(stored_flags(&few), (vec!["pve-01"], 13));
    assert_eq!(stored_flags(&many), (vec!["pve-01"], 163));
    // The batched cost is 3: the page, its count, one batch of secrets for every device. A
    // per-device lookup costs one statement per device, at least 15 on this data.
    assert!(
        few_statements <= BATCHED_AT_MOST && many_statements <= BATCHED_AT_MOST,
        "{few_statements} statements for 13 devices, {many_statements} for 163"
    );
}

/// The statements one signed-in request costs on the counted pool, and its data.
async fn cost(schema: &TestSchema, statements: &AtomicUsize, query: &str) -> (usize, Value) {
    statements.store(0, Ordering::SeqCst);
    let result = data(schema, query).await;
    (statements.load(Ordering::SeqCst), result)
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn api_005_credential_stored_in_a_set_or_clear_response_costs_no_statement(pool: PgPool) {
    let (counted, statements) = counting_pool(&pool).await;
    let schema = Schema::build(Query, Mutation, EmptySubscription)
        .data(counted.clone())
        .data(credential_loader(counted.clone()))
        .data(vault())
        .data(Bus::new(4))
        .data(Arc::new(FixedClock(now())) as Arc<dyn Clock>)
        .finish();
    let set = |fields: &str| {
        format!(
            r#"mutation {{ setSshCredential(deviceId: "{PVE_01}", credential: {{ password: "a-password" }}) {{ {fields} }} }}"#
        )
    };
    let clear = |fields: &str| {
        format!(r#"mutation {{ clearSshCredential(deviceId: "{PVE_01}") {{ {fields} }} }}"#)
    };
    // Each pair runs on the same state: both sets replace a stored secret, both clears find none.
    cost(&schema, &statements, &set("enabled")).await;
    let (set_plain, _) = cost(&schema, &statements, &set("enabled")).await;
    let (set_known, set_data) = cost(&schema, &statements, &set("credentialStored")).await;
    cost(&schema, &statements, &clear("enabled")).await;
    let (clear_plain, _) = cost(&schema, &statements, &clear("enabled")).await;
    let (clear_known, clear_data) = cost(&schema, &statements, &clear("credentialStored")).await;

    assert_eq!(
        set_data["setSshCredential"]["credentialStored"],
        json!(true)
    );
    assert_eq!(
        clear_data["clearSshCredential"]["credentialStored"],
        json!(false)
    );
    assert_eq!(
        (set_known, clear_known),
        (set_plain, clear_plain),
        "credentialStored added statements to the response"
    );
}
