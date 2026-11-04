//! The serving hub runs its daily upkeep at start: a week of partitions ahead and old login
//! attempts pruned (D-4, DB-004, K2 retention).

use std::{
    io::Write as _,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use sqlx::PgPool;
use uuid::Uuid;

use crate::start::url_for;

/// How long the hub may take to start and run its first upkeep.
const DEADLINE: Duration = Duration::from_secs(30);

/// `url_for(database)` with `login` as the user.
fn url_as(login: &str, database: &str) -> String {
    let url = url_for(database);
    let (scheme, rest) = url.split_once("://").unwrap();
    let host = rest.split_once('@').map_or(rest, |(_, host)| host);
    format!("{scheme}://{login}@{host}")
}

/// Kills the hub when the test ends, also when it fails.
struct Running(Child);

impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

async fn upkeep_done(pool: &PgPool) -> bool {
    sqlx::query_scalar(
        "select to_regclass('events.event_log_' || \
             to_char((now() at time zone 'UTC') + interval '7 days', 'YYYYMMDD')) is not null \
         and to_regclass('events.audit_log_' || \
             to_char((now() at time zone 'UTC') + interval '7 days', 'YYYYMMDD')) is not null \
         and not exists (select from access.login_attempts where username = 'long-ago')",
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn db_004_the_serving_hub_runs_the_upkeep_at_start(pool: PgPool) {
    let login = format!("anima_test_{}_hub", Uuid::now_v7().simple());
    sqlx::query(&format!("create role {login} login in role anima_runtime"))
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        "insert into access.login_attempts (id, at, username, client_addr, ok) \
         values ($1, now() - interval '91 days', 'long-ago', '100.64.0.7', false)",
    )
    .bind(Uuid::now_v7())
    .execute(&pool)
    .await
    .unwrap();
    let database = pool.connect_options().get_database().unwrap().to_owned();
    let mut url_file = tempfile::NamedTempFile::new().unwrap();
    write!(url_file, "{}", url_as(&login, &database)).unwrap();
    let mut key_file = tempfile::NamedTempFile::new().unwrap();
    write!(key_file, "{}", STANDARD.encode([3_u8; 32])).unwrap();

    let hub = Command::new(env!("CARGO_BIN_EXE_anima-hub"))
        .env_clear()
        .env("ANIMA_KEY_FILE", key_file.path())
        .env("ANIMA_DATABASE_URL_FILE", url_file.path())
        .env("ANIMA_LOG", "error")
        .env("ANIMA_LISTEN", "127.0.0.1:0")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut hub = Running(hub);
    let started = Instant::now();
    while !upkeep_done(&pool).await {
        assert!(hub.0.try_wait().unwrap().is_none(), "the hub stopped");
        assert!(
            started.elapsed() < DEADLINE,
            "no upkeep before the deadline"
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    drop(hub);
    sqlx::query(&format!("drop role {login}"))
        .execute(&pool)
        .await
        .unwrap();
}
