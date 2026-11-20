//! Hub binary: reads the environment, builds the dependencies and serves the router, or runs
//! `anima-hub migrate` (R-016), `anima-hub operator set <username>`,
//! `anima-hub operator create <username>`, `anima-hub operator show`,
//! `anima-hub operator totp <username> [--disable | --new]` (D-9, FRN-003, FRN-004) or
//! `anima-hub health` (the compose healthcheck), or
//! `anima-hub release <sign | check | publish> <target> <version> <folder>`, the commands of
//! the release workflow, which need no hub (R-179).

mod core_dumps;
mod operator;
mod upkeep;

use std::{collections::BTreeMap, process::ExitCode, sync::Arc};

use anima_hub::{
    DRAIN_LIMIT, Shutdown, all_routes,
    clock::SystemClock,
    config::{self, Config},
    engine::{files, locations, sealing_keys::SealingKeys},
    fleet::{
        access::{self, listen},
        builds,
        credentials::Vault,
        events::{self, Bus, process_log},
        metrics, sessions,
        targets::AddressPolicy,
        vpn,
    },
    schema::Dependencies,
    serve_until,
    work::projects,
};
use sqlx::PgPool;

/// Signals buffered for each live subscriber (LOG-010).
const BUS_CAPACITY: usize = 1024;

#[derive(Debug, thiserror::Error)]
enum StartError {
    #[error("invalid configuration")]
    Config(#[from] config::Error),
    #[error("{0}")]
    Sources(#[from] anima_hub::engine::sources::BuildFailed),
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
        source: anima_hub::fleet::credentials::Error,
    },
    #[error("{0}")]
    RuntimeRole(#[from] events::RoleError),
    /// The events error's text is fixed and holds no value (LOG-007).
    #[error("the audit entries cannot be linked in the hash chain ({0})")]
    AuditChain(#[from] events::Error),
    /// The builds error's text is fixed and holds no value (LOG-007).
    #[error("the builders that went with the operator's tokens cannot be audited ({0})")]
    Builds(#[from] builds::Error),
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
        "usage: anima-hub [migrate | operator set <username> | operator create <username> | \
         operator show | operator totp <username> [--disable | --new] | health | \
         release <sign | check | publish> <target> <version> <folder>]"
    )]
    Usage,
    /// The enrolment URI holds the secret, so it is shown only on a terminal (LOG-007).
    #[error("run operator totp on a terminal: the enrolment URI is shown only there")]
    NotATerminal,
    #[error("the password cannot be read from standard input")]
    Stdin,
    #[error("the password, the TOTP secret and the code cannot be read from standard input")]
    CreateInput,
    /// A terminal would echo the password and the secret (LOG-007).
    #[error(
        "operator create reads the password, the TOTP secret and the code from a pipe: pipe the \
         three lines in; a terminal would show them"
    )]
    CreateFromTerminal,
    #[error("{0}")]
    Operator(#[from] access::Error),
    /// The cause is in the process log, by fault word and SQLSTATE (LOG-007).
    #[error("the permitted networks cannot be stored")]
    PermittedNetworks,
    /// Names the setting, never a value (SEC-004, LOG-007).
    #[error("{0}")]
    Bucket(#[from] builds::BucketError),
    #[error("core dumps cannot be turned off: {0}")]
    CoreDumps(std::io::Error),
    /// Names a setting, a file or a check, never a value (LOG-007).
    #[error("{0}")]
    Release(#[from] builds::ReleaseError),
    #[error("the HTTP client cannot start")]
    Http,
    /// Names a key, never its bytes (VLT-003, LOG-007).
    #[error("the data keys cannot be used: {0}")]
    SealingKeys(#[from] anima_hub::engine::sealing_keys::SealingKeyError),
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
    // Before any secret is read: the vault key, the R2 token and the updater key (D-45).
    core_dumps::forbid().map_err(StartError::CoreDumps)?;
    let env = config::collect_env(std::env::vars_os())?;
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    // Before the hub's own settings are read: the release commands need none of them.
    if let ["release", rest @ ..] = args.as_slice() {
        return release(&env, rest).await;
    }
    let config = Arc::new(config::from_env(&env)?);
    match args.as_slice() {
        [] => serve(config).await,
        ["migrate"] => migrate(&config).await,
        ["operator", "set", username] => operator::set(&config, username).await,
        ["operator", "create", username] => operator::create(&config, username).await,
        ["operator", "show"] => operator::show(&config).await,
        ["health"] => Ok(listen::check_health(config.listen, tls_files(&config)).await?),
        // Each flag is a valid username, so a flag alone is refused rather than enrolled.
        ["operator", "totp", "--disable" | "--new"] => Err(StartError::Usage),
        ["operator", "totp", username] => operator::enrol_totp(&config, username).await,
        ["operator", "totp", username, "--disable"] => {
            operator::disable_totp(&config, username).await
        }
        ["operator", "totp", username, "--new"] => operator::new_totp(username),
        _ => Err(StartError::Usage),
    }
}

/// Runs a command of the release workflow and prints its result (R-179).
async fn release(env: &BTreeMap<String, String>, args: &[&str]) -> Result<(), StartError> {
    let http = upkeep::publishing_client().map_err(|_| StartError::Http)?;
    println!("{}", builds::release_command(env, args, &http).await?);
    Ok(())
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

/// The database of the hub, connected and ready for its use: the role is the runtime role,
/// the audit entries are linked and the permitted networks are stored.
async fn database(config: &Config) -> Result<PgPool, StartError> {
    let pool = connect(config).await?;
    events::check_runtime_role(&pool).await?;
    link_audit_entries(&pool).await?;
    permit_networks(&pool, config).await?;
    Ok(pool)
}

async fn serve(config: Arc<Config>) -> Result<(), StartError> {
    process_log::init(&config.log)?;
    let clock: Arc<dyn anima_hub::clock::Clock> = Arc::new(SystemClock);
    // Bind check and certificate first: a refused address needs no key or database (NET-003).
    let listening = listen::listen(config.listen, tls_files(&config), clock.clone()).await?;
    // In the link mode `anima-link` reaches the hub through this socket (one-click plan S5).
    let link = config.link_socket.as_deref();
    let link = link.map(listen::link_socket).transpose()?;
    warn_without_probe_deny(&config);
    let vault = vault(&config)?;
    let files = file_store(&config, &vault)?;
    let bucket = release_bucket(&config)?;
    let pool = database(&config).await?;
    // One deny list for every outbound connection; `tunnels` says which path takes it how.
    // A hub with a link reaches the computers of the link through its dial socket (HMD-048).
    let link_dial = config.link_control.as_deref();
    let policy = AddressPolicy::new(config.probe_deny.clone())
        .reaching_link(link_dial.and_then(anima_hub::config::dial_socket));
    let live = open_sessions(&pool, clock.now())
        .await?
        .denying(policy.clone());
    tracing::info!(listen = %listening.local_addr()?, "hub started");
    let bus = Bus::new(BUS_CAPACITY);
    // A hosted hub is a device of its own list, and it watches the computers of its link;
    // its own checks are ready before the scheduler runs.
    let hosted = upkeep::hosted(&config, &pool, &bus, &clock).await;
    let feed = metrics::Feed::new();
    // The sources reach only their declared API bases, through the same deny list (K-E1).
    // The Dawarich pull stores its points with the sealing keys of the positions (R-NEW-dawarichkey).
    let positions = locations::Seal::new(files.sealing_keys().clone());
    let intakes = vec![projects::intake(), locations::dawarich_intake(positions)];
    let sources = anima_hub::source_reach(intakes, policy.clone())?;
    let dependencies = Dependencies {
        vpn_broker: config.vpn_broker.clone().map(vpn::Broker::new),
        config,
        clock: clock.clone(),
        pool: pool.clone(),
        bus: bus.clone(),
        vault,
        live: live.clone(),
        policy,
        metrics_feed: feed,
        build_updates: builds::Updates::new(),
        bucket,
        vpn_feed: vpn::Feed::new(),
        sources,
        files,
    };
    let background = upkeep::spawn(&dependencies, hosted);
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
        listening.admitting(edge).with_link(link),
        app,
        shutdown,
        first_stop_signal(),
    )
    .await;
    background.iter().for_each(tokio::task::JoinHandle::abort);
    Ok(())
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

fn vault(config: &Config) -> Result<Vault, StartError> {
    Vault::from_key_file(&config.key_file).map_err(|source| StartError::Key {
        path: config.key_file.display().to_string(),
        source,
    })
}

/// The file store of `ANIMA_FILES_DIR`, sealed with the data keys of `ANIMA_DATA_KEY_DIR`. A
/// data key that is the credential key or another data key stops the start (VLT-003); without
/// either folder the hub starts and an upload answers UNAVAILABLE.
fn file_store(config: &Config, vault: &Vault) -> Result<files::Store, StartError> {
    let keys = SealingKeys::open(config.data_key_dir.as_deref(), vault.key_id().as_bytes())?;
    let store = files::Store::in_folder(config.files_dir.as_deref(), keys);
    tracing::info!(files = store.available(), "file store");
    Ok(store)
}

/// The release bucket when `ANIMA_RELEASES_ENDPOINT` is set, its token and the updater key read
/// from their files now (BLD-060, R-108, R-117). An updater key that cannot be used turns
/// publishing off and the hub still starts; the log names the check that failed, never a value
/// (LOG-007).
fn release_bucket(config: &Config) -> Result<Option<Arc<builds::Bucket>>, StartError> {
    let opened = config.releases.as_ref().map(builds::open_bucket);
    let bucket = opened.transpose()?.flatten();
    tracing::info!(publishing = bucket.is_some(), "release bucket");
    Ok(bucket.map(Arc::new))
}

/// Links the audit entries that exist when the hash chain is empty, before anything writes an
/// audit entry (LOG-026). Each command that writes to the audit log calls it first.
async fn link_audit_entries(pool: &PgPool) -> Result<(), StartError> {
    let linked = events::link_existing(pool).await?;
    if linked > 0 {
        tracing::info!(linked, "audit entries linked in the hash chain");
    }
    Ok(())
}

/// Connects to the database named in `ANIMA_DATABASE_URL_FILE` (DB-002).
async fn connect(config: &Config) -> Result<PgPool, StartError> {
    let url = zeroize::Zeroizing::new(
        std::fs::read_to_string(&config.database_url_file)
            .map_err(|error| StartError::DatabaseUrl(error.kind()))?,
    );
    Ok(PgPool::connect(url.trim()).await?)
}
