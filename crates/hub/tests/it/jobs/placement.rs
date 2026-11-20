use anima_hub::{
    fleet::events::{ClientAddr, Operator},
    ids::{DeviceId, JobId, ProjectId},
    work::jobs::{self, Error, NewJob},
};
use axum::http::StatusCode;
use chrono::TimeDelta;
use serde_json::{Value, json};
use sqlx::PgPool;

use super::{Work, build, claim_of};
use crate::daemon::harness::{DOCKER_HOST, PERMITTED, PVE_01};

fn asker() -> (Operator, ClientAddr) {
    (Operator::hub(), ClientAddr(PERMITTED.parse().unwrap()))
}

fn device(text: &str) -> DeviceId {
    text.parse().unwrap()
}

/// The claim of a machine that says that it has `gb` GB of free memory.
fn claim_with(device: &str, gb: u64) -> Value {
    let mut claim = claim_of(device, &json!([]));
    claim["free"]["memoryBytes"] = json!(gb << 30);
    claim
}

async fn name_of(pool: &PgPool, device: &str) -> String {
    sqlx::query_scalar("select name from devices.devices where id = $1")
        .bind(self::device(device))
        .fetch_one(pool)
        .await
        .unwrap()
}

/// The change of the audit entry of the start of the job.
async fn start_entry(pool: &PgPool, job: JobId) -> Value {
    sqlx::query_scalar(
        "select change from events.audit_log where target_id = $1 and action = 'start'",
    )
    .bind(job.to_string())
    .fetch_one(pool)
    .await
    .unwrap()
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn r_259_the_machine_with_more_free_memory_gets_a_new_job_and_the_entry_gives_the_reason(
    pool: PgPool,
) {
    let work = Work::new(&pool).await;
    let small = work.daemons.token_for(PVE_01).await;
    let large = work.daemons.token_for(DOCKER_HOST).await;
    // The large machine asks while the queue is empty: the hub knows what it said.
    let (status, _) = work.claim(&large, &claim_with(DOCKER_HOST, 32)).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let queued = work.enqueue(build()).await;
    work.daemons.clock.advance(TimeDelta::seconds(5));
    let (status, _) = work.claim(&small, &claim_with(PVE_01, 2)).await;
    assert_eq!(status, StatusCode::NO_CONTENT, "inside the 20 seconds");
    let waits = jobs::placement_reason(work.queue(), queued.id)
        .await
        .unwrap();
    let small_name = name_of(&pool, PVE_01).await;
    assert_eq!(
        waits,
        Some(format!(
            "{small_name} refused: a machine with more free memory asked a short time before"
        ))
    );

    let (status, job) = work.claim(&large, &claim_with(DOCKER_HOST, 32)).await;
    assert_eq!(status, StatusCode::OK, "{job}");
    let entry = start_entry(&pool, queued.id).await;
    let large_name = name_of(&pool, DOCKER_HOST).await;
    assert_eq!(entry["device"], json!(DOCKER_HOST));
    assert_eq!(
        entry["reason"],
        json!(format!(
            "{large_name}: fits, 32 GB of memory and 64 GB of disk free; {small_name} refused: \
             a machine with more free memory asked a short time before"
        ))
    );
    assert_eq!(entry["refused"][0]["device"], json!(PVE_01));
    assert_eq!(entry["refused"][0]["why"], json!("waits_for_better"));
    assert_eq!(entry["asked"]["memoryBytes"], json!(1_u64 << 30));
    assert!(entry.get("parameters").is_none(), "no parameter of the job");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn r_259_after_20_seconds_the_first_machine_that_asks_gets_the_job(pool: PgPool) {
    let work = Work::new(&pool).await;
    let small = work.daemons.token_for(PVE_01).await;
    let large = work.daemons.token_for(DOCKER_HOST).await;
    work.enqueue(build()).await;
    work.daemons.clock.advance(TimeDelta::seconds(15));
    let mut busy = claim_with(DOCKER_HOST, 32);
    busy["free"]["slots"] = json!(0);
    work.claim(&large, &busy).await;
    work.daemons.clock.advance(TimeDelta::seconds(5));

    let (status, job) = work.claim(&small, &claim_with(PVE_01, 2)).await;
    assert_eq!(status, StatusCode::OK, "{job}");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn rou_006_a_named_machine_is_never_replaced_and_one_that_cannot_run_the_job_is_refused(
    pool: PgPool,
) {
    let work = Work::new(&pool).await;
    let first = work.daemons.token_for(PVE_01).await;
    let named = work.daemons.token_for(DOCKER_HOST).await;
    let names = |device_id: &str| NewJob {
        machine: Some(device(device_id)),
        ..build()
    };

    // A machine that never asked this hub for work cannot be named.
    let refused = jobs::enqueue(work.queue(), names(DOCKER_HOST), asker(), work.now()).await;
    let Err(Error::NoFit(reason)) = refused else {
        panic!("{refused:?}");
    };
    assert_eq!(reason, "the machine never asked this hub for work");

    work.claim(&named, &claim_with(DOCKER_HOST, 8)).await;
    let queued = work.enqueue(names(DOCKER_HOST)).await;
    work.daemons.clock.advance(TimeDelta::hours(1));
    let (status, _) = work.claim(&first, &claim_with(PVE_01, 64)).await;
    assert_eq!(status, StatusCode::NO_CONTENT, "it waits for its machine");
    let (status, job) = work.claim(&named, &claim_with(DOCKER_HOST, 8)).await;
    assert_eq!(status, StatusCode::OK, "{job}");
    assert_eq!(job["id"], json!(queued.id));
    let entry = start_entry(&pool, queued.id).await;
    assert_eq!(entry["refused"][0]["why"], json!("names_other"));

    // The named machine is a Linux machine: a job for a Mac is refused at once.
    let mut mac = names(DOCKER_HOST);
    mac.os = Some(daemon_core::wire::WorkOs::Macos);
    let refused = jobs::enqueue(work.queue(), mac, asker(), work.now()).await;
    assert!(matches!(refused, Err(Error::NoFit(_))), "{refused:?}");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn rou_006_an_excluded_machine_gets_only_work_that_names_it(pool: PgPool) {
    let work = Work::new(&pool).await;
    let pve = work.daemons.token_for(PVE_01).await;
    work.claim(&pve, &claim_with(PVE_01, 8)).await;
    let queue = work.queue();
    let unknown = jobs::exclude(queue, (device(DOCKER_HOST), true), asker(), work.now()).await;
    assert!(!unknown.unwrap(), "a machine that never asked for work");
    let excluded = jobs::exclude(queue, (device(PVE_01), true), asker(), work.now()).await;
    assert!(excluded.unwrap());

    let open = work.enqueue(build()).await;
    work.daemons.clock.advance(TimeDelta::hours(1));
    let (status, _) = work.claim(&pve, &claim_with(PVE_01, 8)).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let reason = jobs::placement_reason(queue, open.id)
        .await
        .unwrap()
        .unwrap();
    assert!(
        reason.ends_with("the operator excluded it from work that names no machine"),
        "{reason}"
    );

    let for_it = work
        .enqueue(NewJob {
            machine: Some(device(PVE_01)),
            ..build()
        })
        .await;
    let (status, job) = work.claim(&pve, &claim_with(PVE_01, 8)).await;
    assert_eq!((status, &job["id"]), (StatusCode::OK, &json!(for_it.id)));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn rou_006_each_job_of_a_project_goes_only_to_the_machine_that_is_pinned_for_it(
    pool: PgPool,
) {
    let work = Work::new(&pool).await;
    let pve = work.daemons.token_for(PVE_01).await;
    let docker = work.daemons.token_for(DOCKER_HOST).await;
    work.claim(&docker, &claim_with(DOCKER_HOST, 8)).await;
    let project: ProjectId = "0191c000-0000-7000-8000-000000000001".parse().unwrap();
    let queue = work.queue();
    let pinned = jobs::pin_project(queue, (project, device(DOCKER_HOST)), asker(), work.now());
    assert!(pinned.await.unwrap());

    let of_project = work
        .enqueue(NewJob {
            project: Some(project),
            ..build()
        })
        .await;
    work.daemons.clock.advance(TimeDelta::hours(1));
    let (status, _) = work.claim(&pve, &claim_with(PVE_01, 64)).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let reason = jobs::placement_reason(queue, of_project.id)
        .await
        .unwrap()
        .unwrap();
    assert!(
        reason.ends_with("the job names a different machine"),
        "{reason}"
    );
    let (status, job) = work.claim(&docker, &claim_with(DOCKER_HOST, 8)).await;
    assert_eq!(
        (status, &job["id"]),
        (StatusCode::OK, &json!(of_project.id))
    );

    // With no pin, the next job of the project goes to the first machine that asks.
    assert!(
        jobs::unpin_project(queue, project, asker(), work.now())
            .await
            .unwrap()
    );
    assert!(
        !jobs::unpin_project(queue, project, asker(), work.now())
            .await
            .unwrap()
    );
    let next = work
        .enqueue(NewJob {
            project: Some(project),
            ..build()
        })
        .await;
    work.daemons.clock.advance(TimeDelta::hours(1));
    let (status, job) = work.claim(&pve, &claim_with(PVE_01, 64)).await;
    assert_eq!((status, &job["id"]), (StatusCode::OK, &json!(next.id)));
}
