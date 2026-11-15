//! The hub on the PostgreSQL that the application carries (one-click plan S3, OR-6). The hub
//! is given two file names and nothing else: it cannot tell this database from the one in
//! Docker (DB-002). `migrate` runs with the owner's URL file and the hub serves with its own,
//! as on the VPS (R-016).

use std::{
    io::{BufRead as _, BufReader, Read as _, Write as _},
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use anima_database::{Layout, Limits, Role, Settings, Tree, startup};
use base64::{Engine as _, engine::general_purpose::STANDARD};

use super::{DEADLINE, Running, finished, spawn};

const SETTINGS: Settings = Settings {
    max_connections: 30,
    shared_buffers_mb: 16,
    flush_through_drive_cache: false,
};
const LIMITS: Limits = Limits {
    ready: Duration::from_secs(60),
    stop: Duration::from_secs(30),
};

/// The PostgreSQL 17 the gates already need (`quality/bin/with-postgres.ts`); never skipped.
fn tree() -> Tree {
    let named = std::env::var_os("ANIMA_POSTGRES_TREE").map(PathBuf::from);
    let known = [
        "/opt/homebrew/opt/postgresql@17",
        "/usr/local/opt/postgresql@17",
        "/usr/lib/postgresql/17",
    ]
    .map(PathBuf::from);
    named
        .iter()
        .chain(&known)
        .find_map(|folder| Tree::at(folder).ok())
        .expect("no PostgreSQL 17 found: install it or set ANIMA_POSTGRES_TREE")
}

fn text(path: &Path) -> &str {
    path.to_str().unwrap()
}

/// Starts the hub and returns it with the address it serves on.
fn serve(envs: &[(&str, &str)]) -> (Running, SocketAddr) {
    let mut hub = spawn(&[], envs);
    let stdout = hub.0.stdout.take().unwrap();
    let (lines, received) = mpsc::channel();
    thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            let _ = lines.send(line);
        }
    });
    let started = Instant::now();
    loop {
        let left = DEADLINE.saturating_sub(started.elapsed());
        let line = received.recv_timeout(left).unwrap_or_else(|_| {
            let mut stderr = String::new();
            let _ = hub.0.stderr.take().unwrap().read_to_string(&mut stderr);
            panic!("the hub did not start: {stderr}");
        });
        if line.contains("hub started") {
            let line: serde_json::Value = serde_json::from_str(&line).unwrap();
            let listen = line["fields"]["listen"].as_str().unwrap().parse().unwrap();
            return (hub, listen);
        }
    }
}

#[test]
fn db_002_r_016_the_hub_migrates_and_serves_on_the_bundled_database() {
    let folder = tempfile::tempdir().unwrap();
    let layout = Layout::new(folder.path().join("db root"), folder.path().join("s")).unwrap();
    let database = startup::start(&tree(), &layout, SETTINGS, LIMITS).unwrap();
    let owner_url = layout.url_file(Role::Owner);
    let hub_url = layout.url_file(Role::Hub);
    let key = folder.path().join("anima_key");
    std::fs::write(&key, STANDARD.encode([5_u8; 32])).unwrap();

    // The one-shot migration, as the owner.
    let (migrated, stderr) = finished(spawn(
        &["migrate"],
        &[
            ("ANIMA_DATABASE_URL_FILE", text(&owner_url)),
            ("ANIMA_KEY_FILE", text(&key)),
        ],
    ));
    assert!(migrated, "{stderr}");

    // A write through the hub's own code, as the hub login that owns nothing.
    let serving = [
        ("ANIMA_DATABASE_URL_FILE", text(&hub_url)),
        ("ANIMA_KEY_FILE", text(&key)),
        ("ANIMA_LISTEN", "127.0.0.1:0"),
    ];
    let mut operator = super::spawn_with_input(&["operator", "set", "ada"], &serving);
    operator
        .0
        .stdin
        .take()
        .unwrap()
        .write_all(b"a long test passphrase for ada\n")
        .unwrap();
    let (set, stderr) = finished(operator);
    assert!(set, "{stderr}");

    // The hub passes its start-up checks on this database (not a superuser, no right to
    // change the audit log: R-016, LOG-006) and answers its own health check.
    let (hub, address) = serve(&serving);
    let listen = address.to_string();
    let (healthy, stderr) = finished(spawn(&["health"], &[("ANIMA_LISTEN", &listen)]));
    assert!(healthy, "{stderr}");

    drop(hub);
    assert_eq!(
        database.stop(LIMITS).unwrap(),
        anima_database::Stopped::Clean
    );
}
