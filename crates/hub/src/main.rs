//! Hub binary: reads the environment, builds the dependencies and serves the router, or runs
//! `anima-hub migrate` (R-016) or `anima-hub operator set <username>` (D-9).

use std::{io::BufRead as _, net::SocketAddr, sync::Arc};

use anima_hub::{
    access,
    clock::{Clock as _, SystemClock},
    config::{self, Config},
    events::Bus,
    router,
    schema::build_schema,
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
    #[error("hub failed to serve")]
    Serve(#[from] std::io::Error),
    #[error("usage: anima-hub [migrate | operator set <username>]")]
    Usage,
    #[error("the password cannot be read from standard input")]
    Stdin,
    #[error("{0}")]
    Operator(#[from] access::Error),
}

#[tokio::main]
async fn main() -> Result<(), StartError> {
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
        _ => Err(StartError::Usage),
    }
}

async fn serve(config: Arc<Config>) -> Result<(), StartError> {
    let pool = connect(&config).await?;
    // SEAM (R-016): `anima_hub::events::check_runtime_role(&pool).await?;` once WP-05 T1 adds it.
    let clock: Arc<dyn anima_hub::clock::Clock> = Arc::new(SystemClock);
    let listener = tokio::net::TcpListener::bind(config.listen).await?;
    let schema = build_schema(config, clock.clone(), pool.clone(), Bus::new(BUS_CAPACITY));
    let app = router(schema, pool, clock).into_make_service_with_connect_info::<SocketAddr>();
    Ok(axum::serve(listener, app).await?)
}

/// Applies the migrations as the owner role and exits; serving never migrates (R-016).
async fn migrate(config: &Config) -> Result<(), StartError> {
    let pool = connect(config).await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    Ok(())
}

/// Reads one line from standard input as the password and creates or replaces the operator.
async fn set_operator(config: &Config, username: &str) -> Result<(), StartError> {
    let mut line = String::new();
    std::io::stdin()
        .lock()
        .read_line(&mut line)
        .map_err(|_| StartError::Stdin)?;
    let typed = line.trim_end_matches(['\r', '\n']).to_owned();
    zeroize::Zeroize::zeroize(&mut line);
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

/// Connects to the database named in `ANIMA_DATABASE_URL_FILE` (DB-002).
async fn connect(config: &Config) -> Result<PgPool, StartError> {
    let url = zeroize::Zeroizing::new(
        std::fs::read_to_string(&config.database_url_file)
            .map_err(|error| StartError::DatabaseUrl(error.kind()))?,
    );
    Ok(PgPool::connect(url.trim()).await?)
}
