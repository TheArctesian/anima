//! The real program with the real hub, database and gateway (one-click plan S4).
//!
//! The hub, the database program and the gateway are the ones this workspace builds: `cargo
//! nextest run --workspace` and `cargo test --workspace` build them beside this program. The
//! PostgreSQL tree is the one the gates already need.
//!
//! `anima-link` is a Go program that needs a relay on the network, so each test puts a
//! stand-in script beside links to the built programs, in a `programs` folder of its own. The
//! stand-in prints the link's ready line and waits for SIGTERM: the tests stay offline.

use std::io::{BufRead as _, BufReader, Read as _, Write as _};
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use tempfile::TempDir;

const LIMIT: Duration = Duration::from_secs(120);
const LOCAL_HUB: &str = env!("CARGO_BIN_EXE_anima-local-hub");

/// The stand-in for `anima-link hub`: the ready line of the real one, on standard error.
const LINK: &str = r#"#!/bin/sh
trap 'exit 0' TERM
echo 'time=2026-11-15T10:00:00Z level=INFO msg="the hub link is ready" relay=stand-in devices=0' >&2
while :; do sleep 0.1; done
"#;

/// The folder cargo builds every program of the workspace into.
fn built() -> PathBuf {
    let folder = Path::new(LOCAL_HUB).parent().unwrap().to_path_buf();
    for name in ["anima-hub", "anima-database", "anima-gateway"] {
        assert!(
            folder.join(name).is_file(),
            "{name} is not built beside anima-local-hub: build the workspace (`cargo build --workspace`)"
        );
    }
    folder
}

/// A folder with the built programs and the stand-in link, as `install` would carry them.
fn programs(rig: &Rig) -> PathBuf {
    use std::os::unix::fs::PermissionsExt as _;
    let folder = rig.folder.path().join("programs");
    if !folder.is_dir() {
        std::fs::create_dir(&folder).unwrap();
        for name in [
            "anima-local-hub",
            "anima-hub",
            "anima-database",
            "anima-gateway",
        ] {
            std::os::unix::fs::symlink(built().join(name), folder.join(name)).unwrap();
        }
        std::fs::write(folder.join("anima-link"), LINK).unwrap();
        let mode = std::fs::Permissions::from_mode(0o700);
        std::fs::set_permissions(folder.join("anima-link"), mode).unwrap();
    }
    folder
}

/// The PostgreSQL 17 the gates already need (`quality/bin/with-postgres.ts`); never skipped.
fn tree() -> PathBuf {
    let named = std::env::var_os("ANIMA_POSTGRES_TREE").map(PathBuf::from);
    let known = [
        "/opt/homebrew/opt/postgresql@17",
        "/usr/local/opt/postgresql@17",
        "/usr/lib/postgresql/17",
    ]
    .map(PathBuf::from);
    named
        .into_iter()
        .chain(known)
        .find(|folder| folder.join("bin/postgres").is_file())
        .expect("no PostgreSQL 17 found: install it or set ANIMA_POSTGRES_TREE")
}

struct Rig {
    folder: TempDir,
}

/// A `run` that is killed with its members when a test fails before it stops it.
struct Running(Child);

impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

impl Rig {
    fn new() -> Self {
        Self {
            folder: tempfile::tempdir().unwrap(),
        }
    }

    fn root(&self) -> PathBuf {
        self.folder.path().join("hub root")
    }

    fn command(&self, command: &str) -> Command {
        let mut program = Command::new(LOCAL_HUB);
        program
            .env_clear()
            .env("HOME", self.folder.path())
            .arg(command)
            .arg("--root")
            .arg(self.root())
            .arg("--socket-folder")
            .arg(self.folder.path().join("s"));
        program
    }

    /// Starts `run` and waits for its `ready` line.
    fn run(&self) -> Running {
        let mut child = self
            .command("run")
            .args(["--listen", "127.0.0.1:0", "--programs"])
            .arg(programs(self))
            .arg("--tree")
            .arg(tree())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let stdout = child.stdout.take().unwrap();
        let mut running = Running(child);
        let (lines, received) = mpsc::channel();
        thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                let _ = lines.send(line);
            }
        });
        match received.recv_timeout(LIMIT) {
            Ok(line) if line == "ready" => running,
            other => {
                let ended = running.0.try_wait();
                panic!(
                    "the hub did not get ready: {other:?}, {ended:?}, {}",
                    self.logs()
                );
            }
        }
    }

    fn status(&self) -> serde_json::Value {
        let output = self.command("status").output().unwrap();
        assert!(output.status.success(), "{output:?}");
        serde_json::from_slice(&output.stdout).unwrap()
    }

    fn logs(&self) -> String {
        [
            "local-hub.log",
            "anima-database.log",
            "anima-hub.log",
            "anima-gateway.log",
            "anima-link.log",
        ]
        .map(|name| {
            let text = std::fs::read_to_string(self.root().join("log").join(name));
            format!("--- {name}\n{}", text.unwrap_or_default())
        })
        .join("\n")
    }
}

fn stop(mut running: Running) -> ExitStatus {
    let process = nix::unistd::Pid::from_raw(i32::try_from(running.0.id()).unwrap());
    nix::sys::signal::kill(process, nix::sys::signal::Signal::SIGTERM).unwrap();
    let started = Instant::now();
    loop {
        if let Some(status) = running.0.try_wait().unwrap() {
            return status;
        }
        assert!(started.elapsed() < LIMIT, "the hub did not stop");
        thread::sleep(Duration::from_millis(50));
    }
}

fn hub_address(status: &serde_json::Value) -> SocketAddr {
    status["hub"].as_str().unwrap().parse().unwrap()
}

/// `anima-hub health`, the probe that the Docker installation uses.
fn hub_is_healthy(address: SocketAddr) -> bool {
    Command::new(built().join("anima-hub"))
        .arg("health")
        .env_clear()
        .env("ANIMA_LISTEN", address.to_string())
        .status()
        .unwrap()
        .success()
}

/// The first line of the hub's answer to a request from loopback.
fn status_line_of(address: SocketAddr, path: &str) -> String {
    let mut stream = TcpStream::connect(address).unwrap();
    stream.set_read_timeout(Some(LIMIT)).unwrap();
    write!(
        stream,
        "GET {path} HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n\r\n"
    )
    .unwrap();
    let mut answer = String::new();
    let _ = stream.read_to_string(&mut answer);
    answer.lines().next().unwrap_or_default().to_owned()
}

/// Nothing of the group is left: no hub on its port, no database socket, a clean shutdown.
fn assert_all_stopped(rig: &Rig, address: SocketAddr) {
    let status = rig.status();
    assert_eq!(status["running"], false, "{status}");
    assert_eq!(status["state"], "stopped", "{status}");
    assert!(!hub_is_healthy(address));
    let socket = rig.folder.path().join("s/.s.PGSQL.5432");
    assert!(!socket.exists(), "the database still runs");
    let database_log = std::fs::read_to_string(rig.root().join("database/server.log")).unwrap();
    assert!(
        database_log.contains("database system is shut down"),
        "{database_log}"
    );
}

#[test]
fn dbl_012_the_real_group_serves_stops_in_order_and_starts_again_on_the_same_database() {
    let rig = Rig::new();
    assert_eq!(rig.status()["running"], false);

    let running = rig.run();
    let status = rig.status();
    assert_eq!(status["state"], "ready", "{status}");
    assert_eq!(status["recovered"], false, "{status}");
    let address = hub_address(&status);
    assert!(address.ip().is_loopback());
    assert!(hub_is_healthy(address), "{}", rig.logs());
    let key = std::fs::read(rig.root().join("vault.key")).unwrap();

    assert!(stop(running).success(), "{}", rig.logs());
    assert_all_stopped(&rig, address);

    let again = rig.run();
    assert!(hub_is_healthy(hub_address(&rig.status())), "{}", rig.logs());
    assert_eq!(std::fs::read(rig.root().join("vault.key")).unwrap(), key);
    assert!(stop(again).success());
}

#[test]
fn f_6_the_hub_is_in_the_link_mode_and_its_link_is_up() {
    let rig = Rig::new();
    let running = rig.run();
    let address = hub_address(&rig.status());

    // NET-001 in the link mode: this computer is a permitted client; the hub's API answers.
    assert_eq!(status_line_of(address, "/health"), "HTTP/1.1 200 OK");
    let graphql = status_line_of(address, "/graphql");
    assert!(
        graphql.starts_with("HTTP/1.1 ") && !graphql.contains("403"),
        "{graphql}"
    );
    // The hub made the link's socket in the short socket folder (S5).
    let socket = std::fs::symlink_metadata(rig.folder.path().join("s/link.sock")).unwrap();
    assert!(std::os::unix::fs::FileTypeExt::is_socket(
        &socket.file_type()
    ));
    let started = Instant::now();
    while rig.status()["link"]["state"] != "up" {
        assert!(started.elapsed() < LIMIT, "{}", rig.logs());
        thread::sleep(Duration::from_millis(50));
    }
    assert!(
        rig.status()["link"]["since"]
            .as_str()
            .unwrap()
            .ends_with('Z')
    );

    assert!(stop(running).success());
}

#[test]
fn frn_007_account_refuses_with_code_20_while_the_hub_does_not_run_and_checks_the_name() {
    let rig = Rig::new();

    let not_ready = rig.command("account").arg("status").output().unwrap();
    let bad_name = rig
        .command("account")
        .args(["create", "ada/x"])
        .output()
        .unwrap();

    assert_eq!(not_ready.status.code(), Some(20), "{not_ready:?}");
    assert_eq!(not_ready.stdout, [] as [u8; 0]);
    assert_eq!(bad_name.status.code(), Some(2), "{bad_name:?}");
}

#[test]
fn a_second_run_of_the_same_folder_ends_with_code_10() {
    let rig = Rig::new();
    let running = rig.run();

    let second = rig
        .command("run")
        .arg("--programs")
        .arg(programs(&rig))
        .arg("--tree")
        .arg(tree())
        .output()
        .unwrap();

    assert_eq!(second.status.code(), Some(10), "{second:?}");
    assert_eq!(rig.status()["state"], "ready");
    assert!(stop(running).success());
}
