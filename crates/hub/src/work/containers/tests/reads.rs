//! The two reads against the test SSH server: what the device hears, and what the hub gives
//! back (HOST-001, CHK-012).

use sqlx::PgPool;

use super::{ID, LISTED, SECRET, device, execs, exiting, policy, reach};
use crate::{
    fleet::{credentials, ssh::LinkError, ssh::tests::Answer},
    work::containers::{ContainerId, Error, LineCount, Tail, list, log},
};

fn tail(lines: Option<i32>) -> Tail {
    Tail {
        container: ContainerId::parse(ID).unwrap(),
        lines: LineCount::new(lines).unwrap(),
    }
}

fn now() -> chrono::DateTime<chrono::Utc> {
    crate::fleet::ssh::tests::fixture::now()
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn host_001_the_list_runs_one_fixed_read_only_docker_command(pool: PgPool) {
    let bench = device(&pool, exiting(LISTED, 0)).await;
    let policy = policy();
    let containers = list(reach(&bench, &policy), bench.device.id, now())
        .await
        .unwrap();
    assert_eq!(containers.len(), 2);
    assert_eq!(containers[0].name, "dawarich_app");
    let heard = execs(&bench);
    assert_eq!(heard.len(), 1);
    assert_eq!(
        heard[0],
        concat!(
            r#"docker ps -a --no-trunc --format '{"id":{{json .ID}},"names":{{json .Names}},"#,
            r#""image":{{json .Image}},"state":{{json .State}},"status":{{json .Status}},"#,
            r#""ports":{{json .Ports}},"created":{{json .CreatedAt}},"#,
            r#""project":{{json (.Label "com.docker.compose.project")}},"#,
            r#""service":{{json (.Label "com.docker.compose.service")}},"#,
            r#""dir":{{json (.Label "com.docker.compose.project.working_dir")}},"#,
            r#""files":{{json (.Label "com.docker.compose.project.config_files")}}}'"#,
        )
    );
    assert!(!heard[0].contains(".Labels"), "{heard:?}");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn host_001_the_log_command_holds_only_the_digits_and_the_number(pool: PgPool) {
    let text = format!("2026-11-18T18:00:00Z {SECRET}\n");
    let bench = device(&pool, exiting(&text, 0)).await;
    let policy = policy();
    let read = log(
        reach(&bench, &policy),
        bench.device.id,
        &tail(Some(50)),
        now(),
    )
    .await
    .unwrap();
    assert_eq!(read.lines.len(), 1);
    assert_eq!(read.lines[0].text, SECRET);
    assert_eq!(
        execs(&bench),
        [format!("docker logs --timestamps --tail 50 {ID} 2>&1")]
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn host_001_a_device_with_no_ssh_credential_has_no_list_and_hears_no_command(pool: PgPool) {
    let bench = device(&pool, exiting(LISTED, 0)).await;
    let mut conn = pool.acquire().await.unwrap();
    assert!(
        credentials::clear_ssh(&mut conn, bench.device.id)
            .await
            .unwrap()
    );
    let policy = policy();
    let refused = list(reach(&bench, &policy), bench.device.id, now()).await;
    assert!(matches!(refused, Err(Error::Link(LinkError::NoCredential))));
    assert_eq!(execs(&bench), [] as [String; 0]);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn host_001_an_unknown_device_is_not_found_and_nothing_connects(pool: PgPool) {
    let bench = device(&pool, Answer::Echo).await;
    let policy = policy();
    let unknown = "0190a000-0000-7000-8002-0000000000ff".parse().unwrap();
    let refused = list(reach(&bench, &policy), unknown, now()).await;
    assert!(matches!(refused, Err(Error::NotFound(id)) if id == unknown));
    assert_eq!(bench.server.connections(), 0);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn log_007_no_error_of_a_failed_read_holds_text_of_the_device(pool: PgPool) {
    let bench = device(&pool, exiting(SECRET, 1)).await;
    let policy = policy();
    let listed = list(reach(&bench, &policy), bench.device.id, now())
        .await
        .unwrap_err();
    let logged = log(reach(&bench, &policy), bench.device.id, &tail(None), now())
        .await
        .unwrap_err();
    for error in [listed, logged] {
        assert!(matches!(error, Error::DockerFailed), "{error:?}");
        for text in [error.to_string(), format!("{error:?}")] {
            assert!(!text.contains("hunter2"), "{text}");
        }
    }
}
