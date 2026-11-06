//! The hub binary's edge as `main.rs` wires it: the bind check stops start-up (NET-003,
//! SEC-008), and a hub served over TLS as a runtime login seeds `ANIMA_PERMITTED_NETWORKS`
//! and refuses other addresses (SEC-001, NET-001, NET-002). Ports are chosen by the system;
//! every spawn has a deadline and is killed when the test ends.

mod served;

use std::{
    io::Read as _,
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

const DEADLINE: Duration = Duration::from_secs(20);

/// A hub process, killed when dropped.
struct Running(Child);

impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn spawn(args: &[&str], envs: &[(&str, &str)]) -> Running {
    let child = Command::new(env!("CARGO_BIN_EXE_anima-hub"))
        .args(args)
        .env_clear()
        .env("ANIMA_LOG", "info")
        .envs(envs.iter().copied())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    Running(child)
}

/// The standard error of a hub that must stop by itself, and whether it succeeded.
fn refused_start(envs: &[(&str, &str)]) -> (bool, String) {
    finished(spawn(&[], envs))
}

/// Whether `hub`, which must stop by itself within [`DEADLINE`], succeeded, and its stderr.
fn finished(mut hub: Running) -> (bool, String) {
    let started = Instant::now();
    let status = loop {
        if let Some(status) = hub.0.try_wait().unwrap() {
            break status;
        }
        assert!(started.elapsed() < DEADLINE, "the hub did not stop");
        thread::sleep(Duration::from_millis(50));
    };
    let mut stderr = String::new();
    hub.0
        .stderr
        .take()
        .unwrap()
        .read_to_string(&mut stderr)
        .unwrap();
    (status.success(), stderr)
}

#[test]
fn net_003_start_up_refuses_an_unspecified_or_public_listen_address() {
    let cases = [
        (
            "0.0.0.0:0",
            "ANIMA_LISTEN=0.0.0.0 is the unspecified address",
        ),
        ("[::]:0", "ANIMA_LISTEN=:: is the unspecified address"),
        (
            "8.8.8.8:0",
            "ANIMA_LISTEN=8.8.8.8 is not a loopback, private",
        ),
        (
            "[2001:db8::1]:0",
            "ANIMA_LISTEN=2001:db8::1 is not a loopback, private",
        ),
        (
            "10.0.0.1:0",
            "ANIMA_TLS_CERT_FILE and ANIMA_TLS_KEY_FILE must be set",
        ),
    ];
    for (listen, expected) in cases {
        let (ok, stderr) = refused_start(&[("ANIMA_LISTEN", listen)]);
        assert!(!ok, "{listen}");
        assert!(
            stderr.starts_with(&format!("anima-hub: {expected}")),
            "{listen}: {stderr}"
        );
    }
}

#[test]
fn sec_001_a_bad_key_file_stops_start_up_without_its_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let made = rcgen::generate_simple_self_signed(vec!["localhost".to_owned()]).unwrap();
    let cert = dir.path().join("cert.pem");
    let key = dir.path().join("key.pem");
    std::fs::write(&cert, made.cert.pem()).unwrap();
    let pem = made.signing_key.serialize_pem();
    let broken = format!("{}!{}", &pem[..60], &pem[60..]);
    std::fs::write(&key, &broken).unwrap();
    let (ok, stderr) = refused_start(&[
        ("ANIMA_LISTEN", "127.0.0.1:0"),
        ("ANIMA_TLS_CERT_FILE", cert.to_str().unwrap()),
        ("ANIMA_TLS_KEY_FILE", key.to_str().unwrap()),
    ]);
    assert!(!ok);
    assert_eq!(
        stderr.trim(),
        "anima-hub: the TLS certificate cannot be loaded: the key file holds no valid PEM \
         private key"
    );
    assert!(!stderr.contains(&pem[30..60]), "{stderr}");
}
