//! PostgreSQL wire counting for batching and exposition tests (API-005, D-24).
use crate::local_socket::Stream as UnixStream;
use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions, PgSslMode},
};
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    net::{TcpListener, TcpStream},
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
pub(crate) async fn counting_pool(
    hub: &PgPool,
    max_connections: u32,
) -> (PgPool, Arc<AtomicUsize>) {
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
    // Preserve password authentication for TCP databases. A Unix socket must be
    // discarded so the counted connection goes through our TCP proxy instead.
    let options = if real.get_socket().is_some() {
        PgConnectOptions::new()
    } else {
        (*real).clone()
    };
    let options = options
        .host("127.0.0.1")
        .port(port)
        .username(real.get_username())
        .database(real.get_database().unwrap_or("postgres"))
        .ssl_mode(PgSslMode::Disable);
    let pool = PgPoolOptions::new()
        .max_connections(max_connections)
        .connect_with(options)
        .await
        .unwrap();
    (pool, count)
}
