//! `anima ssh` and `anima files` through the real hub to an SSH device on loopback (K4
//! Sections 8 and 9): the stored credential, the trusted host key, a terminal session that
//! ends, a file round trip, and SEC-011: after a host change the hub sends no credential.

mod device;
mod sftp;

use std::os::unix::fs::PermissionsExt as _;

use serde_json::{Value, json};
use sqlx::PgPool;

use self::device::{Device, PASSWORD, USER};
use super::hub::RealHub;
use super::words;
use crate::kit::Anima;

/// The device `box` on the hub at 127.0.0.1, SSH on `device`'s port with a stored password
/// and its host key trusted, all through the command line; the device's ID.
async fn reachable_device(anima: &Anima, device: &Device) -> String {
    let site = anima
        .run(&words("site create lab --subnet 127.0.0.0/8"))
        .await;
    assert_eq!(site.code, 0, "{site:?}");
    let line = format!(
        "device create box --site lab --host 127.0.0.1 --no-default-check --ssh-enabled true \
         --ssh-port {} --ssh-username {USER} --ssh-auth password --json",
        device.port
    );
    let made = anima.run(&words(&line)).await;
    assert_eq!(made.code, 0, "{made:?}");
    let id = made.json()["id"].as_str().expect("an id").to_owned();

    let secret = tempfile::NamedTempFile::new().expect("a file");
    std::fs::write(secret.path(), format!("{PASSWORD}\n")).expect("written");
    let private = std::fs::Permissions::from_mode(0o600);
    std::fs::set_permissions(secret.path(), private).expect("0600");
    let path = secret.path().to_str().expect("UTF-8");
    let line = format!("device credential set ssh box --from-file {path}");
    let stored = anima.run(&words(&line)).await;
    assert_eq!(stored.code, 0, "{stored:?}");
    stored.assert_never_shows(PASSWORD);

    let tested = anima.run(&words("device test box --json")).await;
    assert_eq!(tested.code, 0, "{tested:?}");
    let fingerprint = tested.json()["hostKey"]["fingerprintSha256"]
        .as_str()
        .map(str::to_owned);
    let fingerprint = fingerprint.unwrap_or_else(|| panic!("a host key: {tested:?}"));
    let line = format!("device trust accept-host-key box {fingerprint}");
    let trusted = anima.run(&words(&line)).await;
    assert_eq!(trusted.code, 0, "{trusted:?}");
    id
}

/// The device's SSH sessions on the hub, newest first.
async fn sessions_of(hub: &RealHub, reader: &str, id: &str) -> Vec<Value> {
    let query = "query($id: ID!) { sessions(deviceId: $id) { edges { node { \
        protocol endedAt endReason } } } }";
    let data = hub.data(reader, query, json!({ "id": id })).await;
    let edges = data["sessions"]["edges"].as_array().expect("edges").iter();
    edges.map(|edge| edge["node"].clone()).collect()
}

#[sqlx::test(migrations = "../hub/migrations")]
async fn k4_ssh_and_files_reach_the_device_and_a_moved_host_gets_no_credential(pool: PgPool) {
    let hub = RealHub::start(pool).await;
    let reader = hub.login_token().await;
    let anima = hub.anima().with("ANIMA_TOKEN", &reader);
    let device = Device::start(9).await;
    let id = reachable_device(&anima, &device).await;

    let shell = anima
        .run_with_input(&words("ssh box"), "echo anima-e2e\nexit\n")
        .await;
    assert_eq!(shell.code, 0, "{shell:?}");
    assert!(shell.stdout.contains("anima-e2e"), "{shell:?}");
    shell.assert_never_shows(PASSWORD);
    let sessions = sessions_of(&hub, &reader, &id).await;
    assert_eq!(sessions.len(), 1, "{sessions:?}");
    assert_eq!(sessions[0]["protocol"], "SSH", "{sessions:?}");
    assert!(
        sessions[0]["endedAt"].is_string(),
        "the session ended: {sessions:?}"
    );

    let local = tempfile::tempdir().expect("a folder");
    let bytes: Vec<u8> = (0..=255_u8).cycle().take(70_000).collect();
    std::fs::write(local.path().join("payload.bin"), &bytes).expect("written");
    let source = local.path().join("payload.bin");
    let line = format!("files put {} box:/", source.display());
    let put = anima.run(&words(&line)).await;
    assert_eq!(put.code, 0, "{put:?}");
    let on_device = std::fs::read(device.root.path().join("payload.bin")).expect("uploaded");
    assert_eq!(on_device, bytes);
    let listed = anima.run(&words("files ls box:/")).await;
    assert_eq!(listed.code, 0, "{listed:?}");
    assert!(listed.stdout.contains("payload.bin"), "{listed:?}");
    let back = local.path().join("back.bin");
    let line = format!("files get box:/payload.bin {}", back.display());
    let got = anima.run(&words(&line)).await;
    assert_eq!(got.code, 0, "{got:?}");
    assert_eq!(std::fs::read(&back).expect("downloaded"), bytes);

    let moved = anima
        .run(&words("device update box --host localhost"))
        .await;
    assert_eq!(moved.code, 0, "{moved:?}");
    let attempts = device.attempts();
    assert!(attempts > 0, "the device counts sign-ins");
    let refused = anima
        .run_with_input(&words("ssh box --json"), "echo moved\nexit\n")
        .await;
    assert_eq!(refused.code, 6, "{refused:?}");
    let error = refused.error();
    assert_eq!(error["code"], "CONFLICT", "{refused:?}");
    let message = error["message"].as_str().unwrap_or_default();
    assert!(
        message.contains("credential"),
        "says what to do: {refused:?}"
    );
    assert_eq!(device.attempts(), attempts, "the moved host got a sign-in");
    refused.assert_never_shows(PASSWORD);
}
