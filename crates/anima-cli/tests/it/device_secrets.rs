//! `anima device credential` and `anima device trust` against the scripted hub. A credential
//! comes from a file or standard input and never shows in anything the program prints
//! (CMD-014, CMD-025).

use std::os::unix::fs::PermissionsExt as _;
use std::path::PathBuf;

use serde_json::{Value, json};
use tempfile::TempDir;

use crate::kit::Anima;
use crate::kit::fake_hub::{FakeHub, data, refusal};

const TOKEN: &str = "login-token-for-device-secret-tests";
const DEVICE: &str = "0190a000-0000-7000-8002-000000000001";
const PASSWORD: &str = "correct-horse-battery-staple-9x";
const KEY: &str = "-----BEGIN OPENSSH PRIVATE KEY-----\nb3BlbnNzaC1rZXktdjEAAAAAbm9uZQ\n-----END OPENSSH PRIVATE KEY-----\n";
const PASSPHRASE: &str = "key-passphrase-in-a-file-77";

fn signed_in() -> Anima {
    Anima::new().with("ANIMA_TOKEN", TOKEN)
}

/// A file in a new folder with `mode`.
fn secret_file(contents: &str, mode: u32) -> (TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("a folder");
    let path = dir.path().join("secret");
    std::fs::write(&path, contents).expect("a file");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode)).expect("a mode");
    (dir, path)
}

fn ssh_stored(stored: bool) -> Value {
    data(json!({ "setSshCredential": {
        "enabled": true, "port": 22, "username": "operator", "credentialStored": stored,
    }}))
}

fn variables(hub: &FakeHub, operation: &str) -> Value {
    hub.seen()
        .into_iter()
        .find(|seen| seen.operation == operation)
        .unwrap_or_else(|| panic!("no {operation} request"))
        .variables
}

#[tokio::test]
async fn cmd_025_an_ssh_password_from_a_file_reaches_the_hub_and_nothing_printed() {
    let hub = FakeHub::start().await;
    hub.answer("SshCredentialSet", ssh_stored(true));
    let (_dir, path) = secret_file(&format!("{PASSWORD}\n"), 0o600);
    let path = path.to_str().expect("a path");
    for json_flag in [false, true] {
        let mut args = vec![
            "device",
            "credential",
            "set",
            "ssh",
            DEVICE,
            "--from-file",
            path,
            "--hub",
            &hub.origin,
        ];
        if json_flag {
            args.push("--json");
        }
        let run = signed_in().run(&args).await;
        assert_eq!(run.code, 0, "{run:?}");
        run.assert_never_shows(PASSWORD);
        run.assert_never_shows(TOKEN);
        assert!(run.stdout.contains("stored") || run.json()["credentialStored"] == true);
    }
    let sent = variables(&hub, "SshCredentialSet");
    assert_eq!(sent["deviceId"], DEVICE);
    assert_eq!(
        sent["credential"],
        json!({ "password": PASSWORD }),
        "the line break is not part of it"
    );
}

#[tokio::test]
async fn cmd_025_a_password_from_standard_input_and_a_hub_refusal_never_show_it() {
    let hub = FakeHub::start().await;
    hub.answer(
        "RdpCredentialSet",
        refusal("VALIDATION", "The credential was refused."),
    );
    let run = signed_in()
        .run_with_input(
            &[
                "device",
                "credential",
                "set",
                "rdp",
                DEVICE,
                "--from-file",
                "-",
                "--hub",
                &hub.origin,
            ],
            &format!("{PASSWORD}\r\n"),
        )
        .await;
    assert_eq!(run.code, 2, "{run:?}");
    assert!(run.stderr.contains("The credential was refused."));
    run.assert_never_shows(PASSWORD);
    assert_eq!(
        variables(&hub, "RdpCredentialSet")["credential"],
        json!({ "password": PASSWORD })
    );
}

#[tokio::test]
async fn cmd_025_a_private_key_and_its_passphrase_go_as_the_one_of_input() {
    let hub = FakeHub::start().await;
    hub.answer("SshCredentialSet", ssh_stored(true));
    let (_key_dir, key) = secret_file(KEY, 0o600);
    let (_phrase_dir, phrase) = secret_file(PASSPHRASE, 0o600);
    let run = signed_in()
        .run(&[
            "device",
            "credential",
            "set",
            "ssh",
            DEVICE,
            "--private-key",
            "--from-file",
            key.to_str().expect("a path"),
            "--passphrase-file",
            phrase.to_str().expect("a path"),
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    run.assert_never_shows(PASSPHRASE);
    run.assert_never_shows("b3BlbnNzaC1rZXktdjEAAAAAbm9uZQ");
    assert_eq!(
        variables(&hub, "SshCredentialSet")["credential"],
        json!({ "privateKey": { "key": KEY, "passphrase": PASSPHRASE } })
    );
}

#[tokio::test]
async fn cmd_025_a_file_others_can_read_is_refused_without_reading_it_out() {
    let hub = FakeHub::start().await;
    let (_dir, path) = secret_file(PASSWORD, 0o644);
    let run = signed_in()
        .run(&[
            "device",
            "credential",
            "set",
            "vnc",
            DEVICE,
            "--from-file",
            path.to_str().expect("a path"),
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 1, "{run:?}");
    assert!(run.stderr.contains("chmod 600"), "{}", run.stderr);
    run.assert_never_shows(PASSWORD);
    assert!(
        hub.seen()
            .iter()
            .all(|seen| seen.operation != "VncCredentialSet")
    );
}

#[tokio::test]
async fn cmd_023_without_a_terminal_a_secret_must_come_from_a_file() {
    let hub = FakeHub::start().await;
    let run = signed_in()
        .run(&[
            "device",
            "credential",
            "set",
            "rdp",
            DEVICE,
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 2, "{run:?}");
    assert!(run.stderr.contains("--from-file"), "{}", run.stderr);

    let run = signed_in()
        .run(&[
            "device",
            "credential",
            "set",
            "ssh",
            DEVICE,
            "--private-key",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 2, "{run:?}");
    assert!(run.stderr.contains("--from-file"), "{}", run.stderr);
    assert!(
        hub.seen()
            .iter()
            .all(|seen| !seen.operation.contains("Credential"))
    );
}

#[tokio::test]
async fn cmd_024_clearing_a_credential_needs_yes() {
    let hub = FakeHub::start().await;
    hub.answer(
        "VncCredentialClear",
        data(json!({ "clearVncCredential": { "enabled": true, "port": 5900, "credentialStored": false } })),
    );
    let run = signed_in()
        .run(&[
            "device",
            "credential",
            "clear",
            "vnc",
            DEVICE,
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 2, "{run:?}");
    assert!(run.stderr.contains("--yes"));

    let run = signed_in()
        .run(&[
            "device",
            "credential",
            "clear",
            "vnc",
            DEVICE,
            "--yes",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert!(run.stdout.contains("none"), "{}", run.stdout);
    assert_eq!(variables(&hub, "VncCredentialClear")["deviceId"], DEVICE);
}

#[tokio::test]
async fn cmd_021_a_forbidden_credential_change_is_exit_5() {
    let hub = FakeHub::start().await;
    hub.answer(
        "SshCredentialClear",
        refusal("FORBIDDEN", "Only a login token may do this."),
    );
    let run = signed_in()
        .run(&[
            "device",
            "credential",
            "clear",
            "ssh",
            DEVICE,
            "--yes",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 5, "{run:?}");
}

#[tokio::test]
async fn sec_006_accept_host_key_sends_the_fingerprint_and_prints_what_was_accepted() {
    let hub = FakeHub::start().await;
    hub.answer(
        "HostKeyAccept",
        data(json!({ "acceptHostKey": { "algorithm": "ssh-ed25519",
            "fingerprintSha256": "SHA256:abc", "firstSeenAt": "2026-11-13T09:00:00+00:00" } })),
    );
    let run = signed_in()
        .run(&[
            "device",
            "trust",
            "accept-host-key",
            DEVICE,
            "SHA256:abc",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert!(run.stdout.contains("SHA256:abc") && run.stdout.contains("ssh-ed25519"));
    assert_eq!(
        variables(&hub, "HostKeyAccept"),
        json!({ "deviceId": DEVICE, "fingerprintSha256": "SHA256:abc" })
    );
}

#[tokio::test]
async fn sec_006_nothing_is_accepted_without_a_fingerprint_or_with_a_wrong_one() {
    let hub = FakeHub::start().await;
    hub.answer(
        "CertificateAccept",
        refusal("CONFLICT", "That is not the certificate the hub saw."),
    );
    let run = signed_in()
        .run(&[
            "device",
            "trust",
            "accept-host-key",
            DEVICE,
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 2, "{run:?}");
    let run = signed_in()
        .run(&[
            "device",
            "trust",
            "accept-certificate",
            DEVICE,
            "",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 2, "{run:?}");
    assert!(hub.seen().is_empty(), "no request without a fingerprint");

    let run = signed_in()
        .run(&[
            "device",
            "trust",
            "accept-certificate",
            DEVICE,
            "SHA256:other",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 6, "{run:?}");
    assert!(run.stderr.contains("not the certificate the hub saw"));
}
