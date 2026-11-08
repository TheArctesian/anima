//! Running commands on a link (D-3, CHK-012, MET-002): one channel and one `exec` with the
//! exact line per run, standard error unread, the exit status, the output cap and the limit.

use std::time::Duration;

use sqlx::PgPool;

use super::{
    LIMIT, assert_quiet, bench,
    server::{Answer, Heard},
};
use crate::ssh::link::{Error, FixedCommand, OUTPUT_CAP};

/// What the server hears for a sign-in and one command with `line`.
fn one_command(line: &str) -> Vec<Heard> {
    vec![
        Heard::Auth("password"),
        Heard::Session,
        Heard::Exec(line.to_owned()),
    ]
}

/// An answer that writes `stdout`, `stderr` and exits with `status`.
fn exit(stdout: &[u8], stderr: &[u8], status: u32) -> Answer {
    Answer::Exit {
        stdout: stdout.to_vec(),
        stderr: stderr.to_vec(),
        status,
    }
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn chk_012_the_server_hears_one_exec_with_the_exact_line_and_nothing_else(pool: PgPool) {
    let bench = bench(&pool, Answer::Echo).await;
    let mut link = bench.opened().await;
    let command = FixedCommand::new("docker")
        .arg("ps")
        .arg("--format")
        .arg("json");
    link.run(&command, LIMIT).await.unwrap();
    assert_eq!(
        bench.server.requests(),
        one_command("docker ps --format json")
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn met_002_two_runs_share_the_link_each_on_its_own_channel(pool: PgPool) {
    let bench = bench(&pool, Answer::Echo).await;
    let mut link = bench.opened().await;
    let first = link.run(&FixedCommand::new("uptime"), LIMIT).await.unwrap();
    let second = link
        .run(&FixedCommand::new("cat").arg("/proc/meminfo"), LIMIT)
        .await
        .unwrap();
    assert_eq!(first.stdout, b"uptime\n");
    assert_eq!(second.stdout, b"cat /proc/meminfo\n");
    let mut expected = one_command("uptime");
    expected.extend([Heard::Session, Heard::Exec("cat /proc/meminfo".to_owned())]);
    assert_eq!(bench.server.requests(), expected);
    assert_eq!(bench.server.connections(), 1);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn d_3_standard_error_is_dropped_and_a_non_zero_status_is_returned(pool: PgPool) {
    let bench = bench(&pool, exit(b"payload\n", b"secret complaint\n", 3)).await;
    let mut link = bench.opened().await;
    let output = link.run(&FixedCommand::new("false"), LIMIT).await.unwrap();
    assert_eq!(output.stdout, b"payload\n");
    assert_eq!(output.exit_status, Some(3));
    assert!(!output.cut);
    let shown = format!("{output:?}");
    assert!(!shown.contains("payload"), "debug shows no output: {shown}");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn d_3_output_of_exactly_the_cap_is_whole(pool: PgPool) {
    let bench = bench(&pool, exit(&vec![b'x'; OUTPUT_CAP], b"", 0)).await;
    let mut link = bench.opened().await;
    let output = link.run(&FixedCommand::new("big"), LIMIT).await.unwrap();
    assert_eq!(output.stdout.len(), OUTPUT_CAP);
    assert_eq!((output.cut, output.exit_status), (false, Some(0)));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn d_3_output_past_the_cap_is_cut_and_the_link_runs_on(pool: PgPool) {
    let bench = bench(&pool, exit(&vec![b'x'; OUTPUT_CAP + 1], b"", 0)).await;
    let mut link = bench.opened().await;
    let output = link.run(&FixedCommand::new("big"), LIMIT).await.unwrap();
    assert_eq!(output.stdout.len(), OUTPUT_CAP);
    assert_eq!((output.cut, output.exit_status), (true, None));
    let again = link.run(&FixedCommand::new("big"), LIMIT).await.unwrap();
    assert!(again.cut, "the next command runs on the same link");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn met_002_a_command_past_its_limit_ends_and_its_channel_closes(pool: PgPool) {
    let bench = bench(&pool, Answer::Hang).await;
    let mut link = bench.opened().await;
    let limit = Duration::from_millis(300);
    let started = std::time::Instant::now();
    let error = link.run(&FixedCommand::new("sleep"), limit).await;
    let Err(error) = error else {
        panic!("the command finished");
    };
    assert!(matches!(error, Error::TimeLimit), "{error:?}");
    assert!(started.elapsed() < LIMIT, "the limit, not the test's wait");
    assert_quiet(&error, bench.server.port);
    for _ in 0..100 {
        if bench.server.heard().contains(&Heard::Close) {
            return;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("the hub did not close the channel");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn chk_012_a_refused_exec_is_an_error(pool: PgPool) {
    let bench = bench(&pool, Answer::Refuse).await;
    let mut link = bench.opened().await;
    let error = link.run(&FixedCommand::new("uptime"), LIMIT).await;
    assert!(matches!(error, Err(Error::CommandRefused)), "{error:?}");
}
