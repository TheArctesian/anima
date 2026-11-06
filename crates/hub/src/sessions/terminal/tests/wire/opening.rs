//! The opening's outcomes after the upgrade (K4 Sections 8.2, 8.3 and 8.6): a changed host
//! key ends `HOST_KEY_MISMATCH` before any authentication request, a refused or missing
//! credential ends `AUTH_FAILED`, and a device that only prompts gets the password (D-5).

use russh::keys::{PrivateKey, ssh_key::private::Ed25519Keypair};
use sqlx::PgPool;

use super::{
    bench::{Setup, bench},
    fake_ssh::{Heard, Script},
    socket,
};
use crate::sessions::{
    PublicKey,
    tests::fixture::{LOOPBACK, PASSWORD, docker_host, ended, ended_once},
};

/// A host key the fake server does not present.
fn other_key() -> PublicKey {
    let key = PrivateKey::from(Ed25519Keypair::from_seed(&[42; 32]));
    PublicKey::from_blob(key.public_key().to_bytes().unwrap()).unwrap()
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn sec_007_k4_8_3_a_changed_host_key_is_4001_and_the_device_sees_no_sign_in(pool: PgPool) {
    let setup = Setup {
        accepted: Some(other_key()),
        ..Setup::default()
    };
    let mut bench = bench(&pool, setup).await;
    let mut client = socket::open(&bench.url).await;
    assert_eq!(
        client.ended().await,
        ("HOST_KEY_MISMATCH".to_owned(), Some(4001))
    );
    let ended = ended(&pool, bench.session).await;
    ended_once(&ended, "host_key_mismatch", "error", LOOPBACK);
    assert!(
        ended.events[0].3.contains("not the accepted one"),
        "{ended:?}"
    );
    assert_eq!(
        bench.ssh.heard_so_far(),
        [],
        "no authentication request, no PTY, nothing"
    );
}

/// The client opens the terminal and hears `AUTH_FAILED` with 1011; the event says `message`.
async fn refused(pool: &PgPool, bench: &super::Bench, message: &str) {
    let mut client = socket::open(&bench.url).await;
    assert_eq!(client.ended().await, ("AUTH_FAILED".to_owned(), Some(1011)));
    let ended = ended(pool, bench.session).await;
    ended_once(&ended, "auth_failed", "warning", LOOPBACK);
    assert_eq!(ended.events[0].3, message);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_8_6_a_refused_password_is_auth_failed_1011(pool: PgPool) {
    let script = Script {
        password: "not the stored one",
        ..Script::default()
    };
    let mut bench = bench(&pool, Setup::script(script)).await;
    refused(&pool, &bench, "The device refused the SSH sign-in.").await;
    let heard = bench.ssh.heard_so_far();
    assert_eq!(
        heard,
        [Heard::Auth("password")],
        "one try, then nothing more"
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn d_3_no_stored_credential_is_auth_failed_and_nothing_is_sent(pool: PgPool) {
    let mut bench = bench(&pool, Setup::default()).await;
    sqlx::query("delete from credentials.secrets where device_id = $1")
        .bind(docker_host())
        .execute(&pool)
        .await
        .unwrap();
    let message = "No SSH credential is stored for the device, so the hub could not sign in. \
                   Store one in the device's SSH settings.";
    refused(&pool, &bench, message).await;
    assert_eq!(bench.ssh.heard_so_far(), [], "no authentication request");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn d_5_a_device_that_only_prompts_gets_the_password_as_the_answer(pool: PgPool) {
    let script = Script {
        prompts_only: true,
        ..Script::default()
    };
    assert_eq!(script.password, PASSWORD);
    let mut bench = bench(&pool, Setup::script(script)).await;
    let mut client = socket::open(&bench.url).await;
    client.resize(80, 24).await;
    let mut methods = Vec::new();
    loop {
        match bench.ssh.next().await {
            Heard::Auth(method) => methods.push(method),
            Heard::Pty { .. } => break,
            other => panic!("{other:?} before the PTY"),
        }
    }
    assert_eq!(methods.last(), Some(&"keyboard-interactive"), "{methods:?}");
    assert_eq!(bench.ssh.next().await, Heard::Shell);
    bench.ssh.say(b"$ ").await;
    assert_eq!(client.bytes_until(b"$ ").await, b"$ ");
    assert_eq!(client.close().await, Some(1000));
}
