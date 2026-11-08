//! The opening's outcomes after the upgrade (K4 Sections 8.2, 8.3 and 8.6): a changed host
//! key ends `HOST_KEY_MISMATCH` before any authentication request, a refused, missing or
//! unreadable credential ends `AUTH_FAILED`, and a device that only prompts gets the
//! password, for at most 4 rounds of at most 4 prompts (D-5).

mod waking;

use russh::keys::{
    PrivateKey,
    ssh_key::{Cipher, Kdf, LineEnding, private::Ed25519Keypair},
};
use sqlx::PgPool;

use super::{
    bench::{Setup, bench},
    fake_ssh::{Heard, Prompts, Script},
    socket,
};
use crate::{
    credentials::{self, Secret, SshSecret},
    sessions::tests::fixture::{LOOPBACK, PASSWORD, docker_host, ended, ended_once},
    trust::PublicKey,
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
    let received = String::from_utf8_lossy(&client.received).into_owned();
    assert!(!received.contains(PASSWORD), "no secret reaches the client");
}

/// Stores `secret` as `docker-host`'s SSH credential, replacing the password.
async fn store(pool: &PgPool, bench: &super::Bench, secret: SshSecret) {
    let mut conn = pool.acquire().await.unwrap();
    credentials::store_ssh(&mut conn, &bench.terminal.vault, docker_host(), secret)
        .await
        .unwrap();
}

/// The keyboard-interactive requests the server heard, after the client heard `AUTH_FAILED`
/// with the refused sign-in's message.
async fn prompted(pool: &PgPool, script: Script) -> usize {
    let mut bench = bench(pool, Setup::script(script)).await;
    refused(pool, &bench, "The device refused the SSH sign-in.").await;
    let heard = bench.ssh.heard_so_far();
    assert!(
        !heard.iter().any(|heard| matches!(heard, Heard::Pty { .. })),
        "{heard:?}"
    );
    heard
        .iter()
        .filter(|heard| **heard == Heard::Auth("keyboard-interactive"))
        .count()
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

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn d_5_four_prompts_in_a_round_are_answered(pool: PgPool) {
    let script = Script {
        prompts_only: true,
        prompts: Prompts::Round(4),
        ..Script::default()
    };
    let mut bench = bench(&pool, Setup::script(script)).await;
    let mut client = socket::open(&bench.url).await;
    client.resize(80, 24).await;
    assert!(matches!(
        bench.ssh.next_after_sign_in().await,
        Heard::Pty { .. }
    ));
    assert_eq!(client.close().await, Some(1000));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn d_5_a_round_of_five_prompts_is_auth_failed_unanswered(pool: PgPool) {
    let script = Script {
        prompts_only: true,
        prompts: Prompts::Round(5),
        ..Script::default()
    };
    assert_eq!(
        prompted(&pool, script).await,
        1,
        "the round is not answered"
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn d_5_endless_rounds_are_auth_failed_after_four_answers(pool: PgPool) {
    let script = Script {
        prompts_only: true,
        prompts: Prompts::Endless,
        ..Script::default()
    };
    assert_eq!(
        prompted(&pool, script).await,
        5,
        "the start and four answers"
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn d_5_a_device_that_accepts_the_fourth_answer_gives_a_shell(pool: PgPool) {
    let script = Script {
        prompts_only: true,
        prompts: Prompts::Rounds(4),
        ..Script::default()
    };
    let mut bench = bench(&pool, Setup::script(script)).await;
    let mut client = socket::open(&bench.url).await;
    client.resize(80, 24).await;
    assert!(matches!(
        bench.ssh.next_after_sign_in().await,
        Heard::Pty { .. }
    ));
    assert_eq!(client.close().await, Some(1000));
}

/// The device's key, as text, and the same key locked with "the right passphrase".
fn keys() -> (Vec<u8>, Vec<u8>) {
    let key = PrivateKey::from(Ed25519Keypair::from_seed(&[9; 32]));
    let bcrypt = Kdf::Bcrypt {
        salt: vec![3; 16],
        rounds: 1,
    };
    let locked = key
        .encrypt_with(Cipher::Aes256Ctr, bcrypt, 7, "the right passphrase")
        .unwrap();
    let text = |key: &PrivateKey| key.to_openssh(LineEnding::LF).unwrap().as_bytes().to_vec();
    (text(&key), text(&locked))
}

/// With `key` and `passphrase` stored, the session ends `AUTH_FAILED` saying the key cannot
/// be read, and the device sees no key.
async fn unreadable(pool: &PgPool, key: Vec<u8>, passphrase: Option<Vec<u8>>) {
    let mut bench = bench(pool, Setup::default()).await;
    let secret = SshSecret::PrivateKey {
        key: Secret::new(key),
        passphrase: passphrase.map(Secret::new),
    };
    store(pool, &bench, secret).await;
    let message = "The stored SSH private key or its passphrase cannot be read, so the hub \
                   could not sign in.";
    refused(pool, &bench, message).await;
    let heard = bench.ssh.heard_so_far();
    assert!(!heard.contains(&Heard::Auth("publickey")), "{heard:?}");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_8_2_a_key_that_does_not_decode_is_auth_failed(pool: PgPool) {
    unreadable(&pool, b"not a private key".to_vec(), None).await;
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_8_2_a_wrong_passphrase_is_auth_failed(pool: PgPool) {
    let (_, locked) = keys();
    unreadable(&pool, locked, Some(b"a wrong passphrase".to_vec())).await;
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_8_2_a_passphrase_that_is_not_utf_8_is_auth_failed(pool: PgPool) {
    let (key, _) = keys();
    unreadable(&pool, key, Some(vec![0xff, 0xfe])).await;
}
