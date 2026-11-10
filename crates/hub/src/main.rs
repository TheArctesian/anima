//! Hub binary: reads the environment, builds the dependencies and serves the router, or runs
//! `anima-hub migrate` (R-016), `anima-hub operator set <username>`,
//! `anima-hub operator totp <username> [--disable]` (D-9) or `anima-hub health` (the compose
//! healthcheck).

mod upkeep;

use std::{
    io::{BufRead as _, IsTerminal as _},
    process::ExitCode,
    sync::Arc,
};

use anima_hub::{
    DRAIN_LIMIT, Shutdown,
    access::{self, listen},
    all_routes, checks,
    clock::{Clock as _, SystemClock},
    config::{self, Config},
    credentials::Vault,
    events::{self, Bus, process_log},
    metrics, notifications,
    schema::Dependencies,
    serve_until, sessions,
    targets::AddressPolicy,
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
    /// The source's text is fixed and holds no value (LOG-007).
    #[error("the sessions a previous hub left open cannot be ended ({0})")]
    StaleSessions(#[from] sessions::Error),
    #[error("hub failed to serve")]
    Serve(#[from] std::io::Error),
    /// Names the address or the certificate fault kind, never file content (SEC-001).
    #[error("{0}")]
    Listen(#[from] listen::Error),
    #[error("the hub is not healthy: {0}")]
    Health(#[from] listen::HealthError),
    #[error(
        "usage: anima-hub [migrate | operator set <username> | operator totp <username> \
         [--disable] | health]"
    )]
    Usage,
    /// The enrolment URI holds the secret, so it is shown only on a terminal (LOG-007).
    #[error("run operator totp on a terminal: the enrolment URI is shown only there")]
    NotATerminal,
    #[error("the password cannot be read from standard input")]
    Stdin,
    #[error("{0}")]
    Operator(#[from] access::Error),
    /// The cause is in the process log, by fault word and SQLSTATE (LOG-007).
    #[error("the permitted networks cannot be stored")]
    PermittedNetworks,
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
        // `--disable` is a valid username, so it alone is refused rather than enrolled.
        ["operator", "totp", "--disable"] => Err(StartError::Usage),
        ["operator", "totp", username] => enrol_totp(&config, username).await,
        ["operator", "totp", username, "--disable"] => disable_totp(&config, username).await,
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
    let vault = vault(&config)?;
    let pool = connect(&config).await?;
    events::check_runtime_role(&pool).await?;
    permit_networks(&pool, &config).await?;
    // One deny list for every outbound connection; `tunnels` says which path takes it how.
    let policy = AddressPolicy::new(config.probe_deny.clone());
    let live = open_sessions(&pool, clock.now())
        .await?
        .denying(policy.clone());
    tracing::info!(listen = %listening.local_addr()?, "hub started");
    let bus = Bus::new(BUS_CAPACITY);
    let (probing, collecting, sending) =
        reaching(&vault, &policy, config.tailscale_localapi.clone());
    let feed = metrics::Feed::new();
    let dependencies = Dependencies {
        config,
        clock: clock.clone(),
        pool: pool.clone(),
        bus: bus.clone(),
        vault,
        live: live.clone(),
        policy,
        metrics_feed: feed.clone(),
    };
    let upkeep = upkeep::spawn(&pool, &clock);
    let scheduler = checks::spawn_scheduler(pool.clone(), bus.clone(), clock.clone(), probing);
    let collector =
        metrics::spawn_collector(pool.clone(), feed, bus.clone(), clock.clone(), collecting);
    let dispatcher =
        notifications::spawn_dispatcher(pool.clone(), bus.clone(), clock.clone(), sending);
    // Every route is behind the same permitted-networks check (K3, NET-002).
    let edge = access::Edge::new(pool.clone(), bus, clock.clone());
    let app = all_routes(dependencies, edge.clone());
    let shutdown = Shutdown {
        db: pool,
        live,
        clock,
        drain_limit: DRAIN_LIMIT,
    };
    serve_until(
        listening.admitting(edge),
        app,
        shutdown,
        first_stop_signal(),
    )
    .await;
    upkeep.iter().for_each(tokio::task::JoinHandle::abort);
    scheduler.abort();
    collector.abort();
    dispatcher.abort();
    Ok(())
}

/// What checks, metrics and notifications need to reach the outside: the vault and the one
/// address policy (P2-004, D-16).
fn reaching(
    vault: &Vault,
    policy: &AddressPolicy,
    localapi: Option<String>,
) -> (checks::Probing, metrics::Collecting, notifications::Sending) {
    let (vault, policy) = (vault.clone(), policy.clone());
    (
        checks::Probing {
            vault: vault.clone(),
            policy: policy.clone(),
            localapi,
        },
        metrics::Collecting {
            vault: vault.clone(),
            policy: policy.clone(),
        },
        notifications::Sending { vault, policy },
    )
}

/// Stores `ANIMA_PERMITTED_NETWORKS` (D-10). A hub fault is logged by its fault word and
/// SQLSTATE, as the API boundary logs it, never with content (LOG-007); a refused network
/// keeps its plain message.
async fn permit_networks(pool: &PgPool, config: &Config) -> Result<(), StartError> {
    let stored = access::permit_networks(pool, &config.permitted_networks).await;
    stored.map_err(|error| {
        if error.fault().is_none() {
            return StartError::Operator(error);
        }
        process_log::log_failure(&error.into_response_error());
        StartError::PermittedNetworks
    })
}

/// Opens the sessions before the hub serves, and logs how many stale rows ended (a number only).
async fn open_sessions(
    pool: &PgPool,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<sessions::Live, StartError> {
    let (live, ended) = sessions::open(pool, now).await?;
    tracing::info!(ended, "stale sessions ended");
    Ok(live)
}

/// The first stop signal. A second one ends the process at once, without waiting for the
/// sessions or the drain.
async fn first_stop_signal() {
    stop_signal().await;
    tracing::info!("hub stopping");
    tokio::spawn(async {
        stop_signal().await;
        tracing::warn!("second stop signal; exiting now");
        std::process::exit(1);
    });
}

/// Resolves on Ctrl-C or SIGTERM. A signal that cannot be watched is logged and never
/// resolves, so the other one still stops the hub.
async fn stop_signal() {
    let interrupt = async {
        if let Err(error) = tokio::signal::ctrl_c().await {
            tracing::warn!(%error, "cannot watch Ctrl-C");
            std::future::pending::<()>().await;
        }
    };
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut terminate) => {
                terminate.recv().await;
            }
            Err(error) => {
                tracing::warn!(%error, "cannot watch SIGTERM");
                std::future::pending::<()>().await;
            }
        }
    };
    tokio::select! {
        () = interrupt => {}
        () = terminate => {}
    }
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

/// Enrols TOTP and writes the `otpauth://` URI once, to the terminal only: never to a pipe,
/// a file or the process log (LOG-007).
async fn enrol_totp(config: &Config, username: &str) -> Result<(), StartError> {
    if !std::io::stdout().is_terminal() {
        return Err(StartError::NotATerminal);
    }
    let vault = vault(config)?;
    let pool = connect(config).await?;
    let enrolment = access::enrol_totp(&pool, &vault, username).await?;
    println!(
        "Add this to an authenticator app, then sign in again with its code:\n{}",
        enrolment.uri()
    );
    Ok(())
}

/// Turns TOTP off: the password alone signs in again.
async fn disable_totp(config: &Config, username: &str) -> Result<(), StartError> {
    let pool = connect(config).await?;
    access::disable_totp(&pool, username).await?;
    println!("TOTP is off for operator {username}.");
    Ok(())
}

fn vault(config: &Config) -> Result<Vault, StartError> {
    Vault::from_key_file(&config.key_file).map_err(|source| StartError::Key {
        path: config.key_file.display().to_string(),
        source,
    })
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
