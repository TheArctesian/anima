//! The Anima VPN client is a stand-in script that prints fixed Tailscale answers; no test asks
//! this computer's real client or reads its interfaces (ADR-005, R-060).

use std::{
    fs,
    net::Ipv4Addr,
    os::unix::fs::PermissionsExt,
    path::Path,
    time::{Duration, Instant},
};

use serde_json::{Value, json};

use super::super::vpn_client::{
    ANIMA_LOGIN_SERVER, CLIENT_PATIENCE, own_address, running_address, signed_in_to,
};

const PATIENCE: Duration = Duration::from_millis(500);
const ANIMA_ADDRESS: Ipv4Addr = Ipv4Addr::new(100, 101, 0, 9);

fn prefs(control_url: &str, logged_out: bool) -> Value {
    json!({ "ControlURL": control_url, "WantRunning": true, "LoggedOut": logged_out })
}

fn status(backend_state: &str, own_addresses: &[&str]) -> Value {
    json!({
        "BackendState": backend_state,
        "TailscaleIPs": own_addresses,
        "Self": { "HostName": "studio", "TailscaleIPs": own_addresses },
        "Peer": null,
    })
}

fn bytes(answer: &Value) -> Vec<u8> {
    serde_json::to_vec(answer).unwrap()
}

/// A stand-in client in `folder` that runs `script` (with the arguments in `$1`, `$2`).
fn stand_in(folder: &Path, script: &str) -> String {
    let program = folder.join("tailscale");
    fs::write(&program, format!("#!/bin/sh\n{script}\n")).unwrap();
    fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).unwrap();
    program.to_str().unwrap().to_owned()
}

/// A stand-in client that answers `debug prefs` and `status` as Tailscale does.
fn answering(folder: &Path, prefs: &Value, status: &Value) -> String {
    fs::write(folder.join("prefs.json"), bytes(prefs)).unwrap();
    fs::write(folder.join("status.json"), bytes(status)).unwrap();
    let here = folder.to_str().unwrap();
    stand_in(
        folder,
        &format!(
            r#"case "$1 $2" in
  "debug prefs") cat '{here}/prefs.json' ;;
  "status --json") cat '{here}/status.json' ;;
  *) exit 2 ;;
esac"#
        ),
    )
}

#[test]
fn the_vpn_address_is_the_one_the_anima_client_reports_even_beside_a_lower_overlay_address() {
    // NetBird holds 100.64.0.7 on its own interface. The client's answer is all that is read,
    // so that lower address in the same range never competes.
    let folder = tempfile::tempdir().unwrap();
    let client = answering(
        folder.path(),
        &prefs(ANIMA_LOGIN_SERVER, false),
        &status("Running", &["100.101.0.9", "fd7a:115c:a1e0::9"]),
    );
    assert_eq!(
        own_address(&client, ANIMA_LOGIN_SERVER, PATIENCE),
        Some(ANIMA_ADDRESS)
    );
}

#[test]
fn a_client_signed_in_to_another_tailnet_gives_no_vpn_address() {
    let folder = tempfile::tempdir().unwrap();
    let client = answering(
        folder.path(),
        &prefs("https://controlplane.tailscale.com", false),
        &status("Running", &["100.101.0.9"]),
    );
    assert_eq!(own_address(&client, ANIMA_LOGIN_SERVER, PATIENCE), None);
}

#[test]
fn the_login_server_must_be_anima_s_by_origin_and_logged_in() {
    let anima = |url: &str| signed_in_to(&bytes(&prefs(url, false)), ANIMA_LOGIN_SERVER);
    assert!(anima("https://hs.anima.ing"));
    assert!(anima("HTTPS://HS.anima.ing:443/"));
    assert!(!anima("http://hs.anima.ing"));
    assert!(!anima("https://hs.anima.ing.example.net"));
    assert!(!anima("https://controlplane.tailscale.com"));
    assert!(!anima(""));
    let logged_out = bytes(&prefs(ANIMA_LOGIN_SERVER, true));
    assert!(!signed_in_to(&logged_out, ANIMA_LOGIN_SERVER));
    assert!(!signed_in_to(b"not json", ANIMA_LOGIN_SERVER));
}

#[test]
fn a_client_that_is_not_running_gives_no_vpn_address() {
    // Tailscale keeps the node's addresses in its status while it is stopped.
    for state in ["Stopped", "NeedsLogin", "Starting", "NoState"] {
        let stopped = bytes(&status(state, &["100.101.0.9"]));
        assert_eq!(running_address(&stopped), None, "{state}");
    }
    let running = bytes(&status("Running", &["fd7a:115c:a1e0::9", "100.101.0.9"]));
    assert_eq!(running_address(&running), Some(ANIMA_ADDRESS));
    let outside = bytes(&status("Running", &["192.168.1.20"]));
    assert_eq!(running_address(&outside), None);
    let no_node = bytes(&json!({ "BackendState": "Running", "Self": null }));
    assert_eq!(running_address(&no_node), None);
}

#[test]
fn a_missing_failing_or_slow_client_gives_no_vpn_address_in_time() {
    let folder = tempfile::tempdir().unwrap();
    let missing = folder.path().join("missing").to_str().unwrap().to_owned();
    assert_eq!(own_address(&missing, ANIMA_LOGIN_SERVER, PATIENCE), None);

    // Without tailscaled the command line exits with a failure.
    let failing = stand_in(folder.path(), "exit 1");
    assert_eq!(own_address(&failing, ANIMA_LOGIN_SERVER, PATIENCE), None);

    let hanging = tempfile::tempdir().unwrap();
    let slow = stand_in(hanging.path(), "sleep 5");
    let started = Instant::now();
    assert_eq!(
        own_address(&slow, ANIMA_LOGIN_SERVER, Duration::from_millis(100)),
        None
    );
    assert!(started.elapsed() < Duration::from_secs(2));
}

#[test]
fn the_anima_client_is_the_one_on_hs_anima_ing_and_is_asked_for_at_most_a_second() {
    assert_eq!(ANIMA_LOGIN_SERVER, "https://hs.anima.ing");
    assert_eq!(CLIENT_PATIENCE, Duration::from_secs(1));
}
