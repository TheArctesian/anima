//! Native PostgreSQL acceptance: private TLS, role separation, backup, restore and restart.
use super::*;
use crate::{Limits, Role, Settings, Stopped, backup, startup};
use sqlx::{ConnectOptions, Connection};
use std::{
    io::Read,
    time::{Duration, Instant},
};
use zeroize::Zeroizing;

const SETTINGS: Settings = Settings {
    max_connections: 32,
    shared_buffers_mb: 16,
    flush_through_drive_cache: false,
};
const LIMITS: Limits = Limits {
    ready: Duration::from_secs(30),
    stop: Duration::from_secs(10),
};

fn options(layout: &Layout, role: Role) -> PgConnectOptions {
    let mut text = Zeroizing::new(String::new());
    anima_windows::files::read(&layout.url_file(role), true)
        .unwrap()
        .read_to_string(&mut text)
        .unwrap();
    text.parse().unwrap()
}

fn tree() -> crate::Tree {
    let folder = std::env::var_os("ANIMA_POSTGRES_TREE")
        .expect("Windows database tests require ANIMA_POSTGRES_TREE with native PostgreSQL 17");
    crate::Tree::at(Path::new(&folder)).unwrap()
}

#[test]
fn native_database_verifies_tls_and_preserves_data_across_backup_and_restart() {
    let temporary = tempfile::tempdir().unwrap();
    anima_windows::files::protect(temporary.path()).unwrap();
    let layout = Layout::new(
        temporary.path().join("db root"),
        temporary.path().join("socket"),
    )
    .unwrap();
    let tree = tree();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let running = match startup::start(&tree, &layout, SETTINGS, LIMITS) {
        Ok(running) => running,
        Err(error) => {
            let folder = temporary.keep();
            panic!(
                "{error:?}; diagnostic database retained at {}",
                folder.display()
            );
        }
    };
    runtime.block_on(check_connections(&layout));
    let backup_file = temporary.path().join("backup.dump");
    backup::write(&tree, &layout, &backup_file).unwrap();
    assert!(anima_windows::files::secure(&backup_file, true).unwrap());
    assert_eq!(running.stop(LIMITS).unwrap(), Stopped::Clean);
    let restarted = startup::start(&tree, &layout, SETTINGS, LIMITS).unwrap();
    assert!(!restarted.recovered);
    runtime.block_on(check_data(&layout));
    assert_eq!(restarted.stop(LIMITS).unwrap(), Stopped::Clean);
    let moved = Layout::new(
        temporary.path().join("moved database"),
        temporary.path().join("moved socket"),
    )
    .unwrap();
    move_stopped(&layout, &moved);
    let running = startup::start(&tree, &moved, SETTINGS, LIMITS).unwrap();
    runtime.block_on(check_data(&moved));
    assert_eq!(running.stop(LIMITS).unwrap(), Stopped::Clean);
    let restored = Layout::new(
        temporary.path().join("restored"),
        temporary.path().join("other socket"),
    )
    .unwrap();
    let running = startup::start(&tree, &restored, SETTINGS, LIMITS).unwrap();
    backup::restore(&tree, &restored, &backup_file).unwrap();
    runtime.block_on(check_data(&restored));
    assert_eq!(running.stop(LIMITS).unwrap(), Stopped::Clean);
}

/// Windows can retain file handles briefly after process exit (including scanners).
/// Wait for the move itself; a persistent permission failure still fails the test.
fn move_stopped(from: &Layout, to: &Layout) {
    let until = Instant::now() + Duration::from_secs(5);
    loop {
        match fs::rename(from.root(), to.root()) {
            Ok(()) => return,
            Err(error)
                if matches!(error.raw_os_error(), Some(5 | 32 | 33)) && Instant::now() < until =>
            {
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(error) => panic!("stopped database could not be moved: {error}"),
        }
    }
}

async fn check_connections(layout: &Layout) {
    let mut owner = options(layout, Role::Owner).connect().await.unwrap();
    let address: String = sqlx::query_scalar("select host(inet_server_addr())")
        .fetch_one(&mut owner)
        .await
        .unwrap();
    assert_eq!(address, "127.0.0.1");
    sqlx::query("create table public.win2_test (value integer)")
        .execute(&mut owner)
        .await
        .unwrap();
    sqlx::query("insert into public.win2_test values (42)")
        .execute(&mut owner)
        .await
        .unwrap();
    let mut hub = options(layout, Role::Hub).connect().await.unwrap();
    let elevated: bool =
        sqlx::query_scalar("select rolsuper from pg_roles where rolname = current_user")
            .fetch_one(&mut hub)
            .await
            .unwrap();
    assert!(!elevated);
    assert!(
        sqlx::query("create table public.forbidden (value integer)")
            .execute(&mut hub)
            .await
            .is_err()
    );
    owner.close().await.unwrap();
    hub.close().await.unwrap();
    reject_untrusted_connections(layout).await;
}

async fn reject_untrusted_connections(layout: &Layout) {
    let wrong = rcgen::generate_simple_self_signed(vec!["127.0.0.1".into()]).unwrap();
    let certificate = layout.root().join("wrong.crt");
    fs::write(&certificate, wrong.cert.pem()).unwrap();
    assert!(
        options(layout, Role::Owner)
            .ssl_root_cert(&certificate)
            .connect()
            .await
            .is_err()
    );
    assert!(
        options(layout, Role::Owner)
            .ssl_mode(PgSslMode::Disable)
            .connect()
            .await
            .is_err()
    );
}

async fn check_data(layout: &Layout) {
    let mut owner = options(layout, Role::Owner).connect().await.unwrap();
    let value: i32 = sqlx::query_scalar("select value from public.win2_test")
        .fetch_one(&mut owner)
        .await
        .unwrap();
    assert_eq!(value, 42);
    owner.close().await.unwrap();
}
