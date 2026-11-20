//! Gate-only throwaway PostgreSQL for native Windows `SQLx` tests. No existing cluster is used.
use crate::{Layout, Limits, Settings, Tree, cluster, startup, windows::Endpoint};
use sqlx::{ConnectOptions as _, Connection as _};
use std::{
    ffi::OsString,
    io,
    path::Path,
    process::{Command, ExitStatus},
    time::Duration,
};
use zeroize::Zeroizing;

const LIMITS: Limits = Limits {
    ready: Duration::from_secs(30),
    stop: Duration::from_secs(30),
};

/// Run one gate with its own password-authenticated database, stopped before removing its files.
/// # Errors
/// Missing runtime, startup, command execution or database shutdown failed. No URL is logged.
pub fn run(arguments: &[OsString]) -> Result<ExitStatus, Box<dyn std::error::Error>> {
    let (program, args) = arguments
        .split_first()
        .ok_or_else(|| io::Error::from(io::ErrorKind::InvalidInput))?;
    let runtime = std::env::var_os("ANIMA_POSTGRES_TREE")
        .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))?;
    let tree = Tree::at(Path::new(&runtime))?;
    let temporary = tempfile::tempdir()?;
    anima_windows::files::protect(temporary.path())?;
    let layout = Layout::new(
        temporary.path().join("database"),
        temporary.path().join("s"),
    )?;
    let settings = Settings {
        max_connections: 200,
        // Match PostgreSQL's default used by the Unix gate runner.
        shared_buffers_mb: 128,
        flush_through_drive_cache: false,
    };
    let running = startup::start(&tree, &layout, settings, LIMITS)?;
    let endpoint = Endpoint::load(&layout)?;
    let password = endpoint.password()?;
    // SQLx's lossy formatter prefixes certificate files with "file: "; its parser
    // expects the raw path. Build these query pairs explicitly for a round trip.
    let mut url = endpoint
        .options()
        .username(cluster::SUPERUSER)
        .password(password.expose())
        .database("postgres")
        .to_url_lossy();
    url.set_query(None);
    url.query_pairs_mut()
        .append_pair("sslmode", "verify-full")
        .append_pair("sslrootcert", &endpoint.certificate().to_string_lossy());
    fixture_connections(&layout)?;
    disposable_writes(url.as_str())?;
    // SQLx sends Terminate before TLS close_notify. PostgreSQL can already have
    // closed the socket, so Windows reports a reset while SQLx closes a fixture.
    // Like the Unix gate's local socket, shared fixtures need no TLS layer.
    // Independent database tests still exercise the installed TLS-only rules.
    url.set_query(Some("sslmode=disable"));
    let url = Zeroizing::new(url.to_string());
    let status = Command::new(program)
        .args(args)
        .env("DATABASE_URL", url.as_str())
        .status();
    running.stop(LIMITS)?;
    Ok(status?)
}

/// Shared fixtures use local plaintext connections, including wire counting.
/// Only this disposable cluster permits them, on loopback with SCRAM passwords.
/// Production and independent database fixtures retain their TLS-only rules.
fn fixture_connections(layout: &Layout) -> io::Result<()> {
    let path = layout.cluster().join("pg_hba.conf");
    let rules = std::fs::read_to_string(&path)?;
    std::fs::write(
        path,
        format!("hostnossl all all 127.0.0.1/32 scram-sha-256\n{rules}"),
    )
}

/// Match the Unix gate's `fsync=off`, only in the cluster this runner just created.
/// The bundled database and its separate durability tests keep their normal settings.
fn disposable_writes(url: &str) -> Result<(), Box<dyn std::error::Error>> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let mut connection = sqlx::PgConnection::connect(url).await?;
            sqlx::query("alter system set fsync = off")
                .execute(&mut connection)
                .await?;
            sqlx::query("select pg_reload_conf()")
                .execute(&mut connection)
                .await?;
            let until = std::time::Instant::now() + Duration::from_secs(5);
            loop {
                let setting: String = sqlx::query_scalar("show fsync")
                    .fetch_one(&mut connection)
                    .await?;
                if setting == "off" {
                    connection.close().await?;
                    return Ok(());
                }
                if std::time::Instant::now() >= until {
                    return Err(io::Error::other("test database settings did not reload").into());
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        })
}
