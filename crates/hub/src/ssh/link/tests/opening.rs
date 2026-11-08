//! Opening a link (P2-003, P2-004, SEC-007): each refusal before the connection, the pinned
//! key, and the credential sent only to the accepted key.

use sqlx::PgPool;
use tokio::net::TcpListener;

use super::{
    Bench, LIMIT, SEED, assert_quiet, bench, bench_accepting, reaching,
    server::{Answer, Heard, host_key, serve},
};
use crate::{
    credentials,
    ssh::{
        link::{Error, FixedCommand},
        tests::fixture::store_password,
    },
    targets::AddressPolicy,
};

/// The error the link gives under `policy`; it must not open.
async fn refused(bench: &Bench, policy: &AddressPolicy) -> Error {
    match bench.link_with(policy).await {
        Ok(_) => panic!("the link opened"),
        Err(error) => {
            assert_quiet(&error, bench.server.port);
            error
        }
    }
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn sec_007_the_accepted_key_opens_and_runs_a_command(pool: PgPool) {
    let bench = bench(&pool, Answer::Echo).await;
    let mut link = bench.opened().await;
    let output = link.run(&FixedCommand::new("uptime"), LIMIT).await.unwrap();
    assert_eq!(output.stdout, b"uptime\n");
    assert_eq!(output.exit_status, Some(0));
    assert!(!output.cut);
    assert_eq!(bench.server.connections(), 1);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn sec_007_another_key_sends_no_credential(pool: PgPool) {
    let server = serve(SEED, Answer::Echo).await;
    let bench = bench_accepting(&pool, server, host_key(SEED + 1)).await;
    assert!(matches!(
        refused(&bench, &reaching()).await,
        Error::HostKeyChanged
    ));
    assert_eq!(bench.server.connections(), 1, "the key exchange ran");
    assert_eq!(bench.server.heard(), [], "no sign-in, no channel");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn sec_006_no_accepted_key_refuses_before_connecting(pool: PgPool) {
    let bench = bench(&pool, Answer::Echo).await;
    sqlx::query("delete from trust.host_keys")
        .execute(&pool)
        .await
        .unwrap();
    assert!(matches!(
        refused(&bench, &reaching()).await,
        Error::NoHostKey
    ));
    assert_eq!(bench.server.connections(), 0);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn p2_003_ssh_off_refuses_before_connecting(pool: PgPool) {
    let mut bench = bench(&pool, Answer::Echo).await;
    bench.device.ssh.enabled = false;
    assert!(matches!(refused(&bench, &reaching()).await, Error::SshOff));
    assert_eq!(bench.server.connections(), 0);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn p2_004_the_deny_list_refuses_the_host_before_connecting(pool: PgPool) {
    let bench = bench(&pool, Answer::Echo).await;
    let denied = AddressPolicy::new(vec!["127.0.0.0/8".parse().unwrap()]).allowing_loopback();
    assert!(matches!(
        refused(&bench, &denied).await,
        Error::RefusedTarget
    ));
    assert_eq!(bench.server.connections(), 0);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn r_032_the_default_policy_does_not_reach_loopback(pool: PgPool) {
    let bench = bench(&pool, Answer::Echo).await;
    assert!(matches!(
        refused(&bench, &AddressPolicy::default()).await,
        Error::RefusedTarget
    ));
    assert_eq!(bench.server.connections(), 0);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn p2_004_a_host_that_does_not_resolve_is_refused(pool: PgPool) {
    let mut bench = bench(&pool, Answer::Echo).await;
    bench.device.host = "no-such-device.invalid".to_owned();
    assert!(matches!(
        refused(&bench, &reaching()).await,
        Error::Unresolved
    ));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn p2_003_a_closed_port_is_unreachable(pool: PgPool) {
    let mut bench = bench(&pool, Answer::Echo).await;
    let closed = TcpListener::bind("127.0.0.1:0").await.unwrap();
    bench.device.ssh.port = closed.local_addr().unwrap().port();
    drop(closed);
    assert!(matches!(
        refused(&bench, &reaching()).await,
        Error::Unreachable
    ));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn p2_003_a_device_that_never_speaks_times_out(pool: PgPool) {
    let mut bench = bench(&pool, Answer::Echo).await;
    let silent = TcpListener::bind("127.0.0.1:0").await.unwrap();
    bench.device.ssh.port = silent.local_addr().unwrap().port();
    let held = tokio::spawn(async move {
        let connection = silent.accept().await;
        std::future::pending::<()>().await;
        drop(connection);
    });
    assert!(matches!(refused(&bench, &reaching()).await, Error::Timeout));
    held.abort();
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn od_9_a_wrong_password_is_refused_after_one_try(pool: PgPool) {
    let bench = bench(&pool, Answer::Echo).await;
    store_password(&pool, &bench.vault, "not the password").await;
    assert!(matches!(
        refused(&bench, &reaching()).await,
        Error::CredentialRefused
    ));
    assert_eq!(bench.server.heard(), [Heard::Auth("password")]);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn od_9_no_stored_credential_is_refused_after_the_pin(pool: PgPool) {
    let bench = bench(&pool, Answer::Echo).await;
    let mut conn = pool.acquire().await.unwrap();
    assert!(
        credentials::clear_ssh(&mut conn, bench.device.id)
            .await
            .unwrap()
    );
    assert!(matches!(
        refused(&bench, &reaching()).await,
        Error::NoCredential
    ));
    assert_eq!(bench.server.heard(), []);
}
