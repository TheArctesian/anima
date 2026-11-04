//! Hub binary: reads the environment, builds the dependencies and serves the router, or runs
//! `anima-hub migrate` (R-016), `anima-hub operator set <username>` or
//! `anima-hub operator totp <username> [--disable]` (D-9).

use std::{
    io::{BufRead as _, IsTerminal as _},
    net::SocketAddr,
    process::ExitCode,
    sync::Arc,
};

use anima_hub::{
    access,
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
    #[error(
        "usage: anima-hub [migrate | operator set <username> | operator totp <username> [--disable]]"
    )]
    Usage,
    /// The enrolment URI holds the secret, so it is shown only on a terminal (LOG-007).
    #[error("run operator totp on a terminal: the enrolment URI is shown only there")]
    NotATerminal,
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
        ["operator", "totp", username] => enrol_totp(&config, username).await,
        ["operator", "totp", username, "--disable"] => {
            let pool = connect(&config).await?;
            access::disable_totp(&pool, username).await?;
            println!("TOTP is off for operator {username}.");
            Ok(())
        }
        _ => Err(StartError::Usage),
    }
}

async fn serve(config: Arc<Config>) -> Result<(), StartError> {
    process_log::init(&config.log)?;
    let vault = vault(&config)?;
    let pool = connect(&config).await?;
    events::check_runtime_role(&pool).await?;
    let clock: Arc<dyn anima_hub::clock::Clock> = Arc::new(SystemClock);
    let listener = tokio::net::TcpListener::bind(config.listen).await?;
    tracing::info!(listen = %config.listen, "hub started");
    let schema = build_schema(Dependencies {
        config,
        clock: clock.clone(),
        pool: pool.clone(),
        bus: Bus::new(BUS_CAPACITY),
        vault,
    });
    let app = router(schema, pool, clock).into_make_service_with_connect_info::<SocketAddr>();
    Ok(axum::serve(listener, app).await?)
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
