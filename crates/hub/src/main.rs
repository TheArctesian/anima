//! Hub binary: reads the environment, builds the dependencies and serves the router, or runs
//! `anima-hub migrate` (R-016), `anima-hub operator set <username>` (D-9) or `anima-hub health`
//! (the compose healthcheck).

use std::{
    io::{BufRead as _, IsTerminal as _},
    process::ExitCode,
    sync::Arc,
};

use anima_hub::{
    access::{self, listen},
    clock::{Clock as _, SystemClock},
    config::{self, Config},
    credentials::Vault,
    events::{self, Bus, process_log},
    router,
    schema::{Dependencies, build_schema},
};
use sqlx::PgPool;

/// Signals buffered for each live subscriber (LOG-010).
const BUS_CAPACITY: usize = 1024;

#[derive(Debug, thiserror::Error)]
enum StartError {
    #[error("invalid configuration")]
    Config(#[from] config::Error),
    #[error("the database URL file cannot be read ({0})")]
    DatabaseUrl(std::io::ErrorKind),
    #[error("the database is unavailable")]
    Database(#[from] sqlx::Error),
    #[error("the database migrations failed")]
    Migrate(#[from] sqlx::migrate::MigrateError),
    #[error("invalid log setup")]
    Log(#[from] process_log::Error),
    /// Names the path only; the source error never holds key bytes (SEC-004).
    #[error("cannot load the credential key from {path}: {source}")]
    Key {
        path: String,
        source: anima_hub::credentials::Error,
    },
    #[error("{0}")]
    RuntimeRole(#[from] events::RoleError),
    #[error("hub failed to serve")]
    Serve(#[from] std::io::Error),
    /// Names the address or the certificate fault kind, never file content (SEC-001).
    #[error("{0}")]
    Listen(#[from] listen::Error),
    #[error("the hub is not healthy: {0}")]
    Health(#[from] listen::HealthError),
    #[error("usage: anima-hub [migrate | operator set <username> | health]")]
    Usage,
    #[error("the password cannot be read from standard input")]
    Stdin,
    #[error("{0}")]
    Operator(#[from] access::Error),
}

/// Prints the plain error (`Display`, never `Debug`) and exits with a failure code.
#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("anima-hub: {error}");
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<(), StartError> {
    let env = config::collect_env(std::env::vars_os())?;
    let config = Arc::new(config::from_env(&env)?);
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        [] => serve(config).await,
        ["migrate"] => migrate(&config).await,
        ["operator", "set", username] => set_operator(&config, username).await,
        ["health"] => Ok(listen::check_health(config.listen, tls_files(&config)).await?),
        _ => Err(StartError::Usage),
    }
}

/// The certificate files when both TLS variables are set (WP-04 names one `tls.pem` twice).
fn tls_files(config: &Config) -> Option<listen::TlsFiles> {
    let files = config
        .tls_cert_file
        .clone()
        .zip(config.tls_key_file.clone());
    files.map(|(cert, key)| listen::TlsFiles { cert, key })
}

/// An empty `ANIMA_PROBE_DENY` is allowed but logged at start (R-030, K7).
fn warn_without_probe_deny(config: &Config) {
    if config.probe_deny.is_empty() {
        tracing::warn!("ANIMA_PROBE_DENY is empty: checks may probe the hub's own networks");
    }
}

async fn serve(config: Arc<Config>) -> Result<(), StartError> {
    process_log::init(&config.log)?;
    let clock: Arc<dyn anima_hub::clock::Clock> = Arc::new(SystemClock);
    // Bind check and certificate first: a refused address needs no key or database (NET-003).
    let listening = listen::listen(config.listen, tls_files(&config), clock.clone()).await?;
    warn_without_probe_deny(&config);
    let vault = Vault::from_key_file(&config.key_file).map_err(|source| StartError::Key {
        path: config.key_file.display().to_string(),
        source,
    })?;
    let pool = connect(&config).await?;
    events::check_runtime_role(&pool).await?;
    access::permit_networks(&pool, &config.permitted_networks).await?;
    tracing::info!(listen = %listening.local_addr()?, "hub started");
    let bus = Bus::new(BUS_CAPACITY);
    let schema = build_schema(Dependencies {
        config,
        clock: clock.clone(),
        pool: pool.clone(),
        bus: bus.clone(),
        vault,
    });
    // Stand-in for WP-05 T8's `GET /health`; remove this merge when T8's route lands.
    let liveness = axum::Router::new().route("/health", axum::routing::get(|| async { "ok" }));
    let edge = access::Edge::new(pool.clone(), bus, clock.clone());
    let app = router(schema, pool, clock)
        .merge(liveness)
        .layer(access::permitted_networks_layer(edge.clone()));
    match listening.serve(app, edge).await {}
}

/// Applies the migrations as the owner role and exits; serving never migrates (R-016).
async fn migrate(config: &Config) -> Result<(), StartError> {
    let pool = connect(config).await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    Ok(())
}

/// Reads the password (without echo on a terminal, one line when piped) and sets the operator.
async fn set_operator(config: &Config, username: &str) -> Result<(), StartError> {
    let typed = read_password().map_err(|_| StartError::Stdin)?;
    let pool = connect(config).await?;
    access::set_operator(
        &pool,
        username,
        access::Password::new(typed),
        SystemClock.now(),
    )
    .await?;
    println!("Operator {username} is set.");
    Ok(())
}

/// The password from standard input: prompted without echo on a terminal, else one line.
fn read_password() -> std::io::Result<String> {
    if std::io::stdin().is_terminal() {
        return rpassword::prompt_password("Password: ");
    }
    let mut line = String::new();
    std::io::stdin().lock().read_line(&mut line)?;
    let typed = line.trim_end_matches(['\r', '\n']).to_owned();
    zeroize::Zeroize::zeroize(&mut line);
    Ok(typed)
}

/// Connects to the database named in `ANIMA_DATABASE_URL_FILE` (DB-002).
async fn connect(config: &Config) -> Result<PgPool, StartError> {
    let url = zeroize::Zeroizing::new(
        std::fs::read_to_string(&config.database_url_file)
            .map_err(|error| StartError::DatabaseUrl(error.kind()))?,
    );
    Ok(PgPool::connect(url.trim()).await?)
}
