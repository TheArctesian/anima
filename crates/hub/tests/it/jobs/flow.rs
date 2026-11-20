use anima_hub::{
    fleet::events::{ClientAddr, Operator},
    ids::JobId,
    work::jobs::{self, State},
};
use axum::http::StatusCode;
use chrono::TimeDelta;
use daemon_core::wire::StopReason;
use serde_json::json;
use sqlx::PgPool;

use super::{Work, build, claim_of, report_of, runner};
use crate::daemon::harness::{DOCKER_HOST, PERMITTED, PVE_01};

fn id_of(job: &serde_json::Value) -> JobId {
    job["id"].as_str().unwrap().parse().unwrap()
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn k10_4_a_machine_claims_the_oldest_job_and_holds_it_by_its_lease(pool: PgPool) {
    let work = Work::new(&pool).await;
    let token = work.daemons.token_for(PVE_01).await;
    let (status, nothing) = work.claim(&token, &claim_of(PVE_01, &json!([]))).await;
    assert_eq!(
        (status, nothing),
        (StatusCode::NO_CONTENT, serde_json::Value::Null)
    );

    let first = work.enqueue(build()).await;
    work.daemons.clock.advance(TimeDelta::seconds(1));
    work.enqueue(runner()).await;
    let (status, job) = work.claim(&token, &claim_of(PVE_01, &json!([]))).await;

    assert_eq!(status, StatusCode::OK, "{job}");
    assert_eq!(id_of(&job), first.id);
    assert_eq!(
        (job["type"].as_str(), job["attempt"].as_u64()),
        (Some("build"), Some(1))
    );
    assert_eq!(job["parameters"]["recipe"], "desktop_release");
    assert_eq!(
        job["repository"],
        json!({ "forge": "github", "owner": "example", "name": "app" })
    );
    let (held, _) = work.job(first.id).await;
    assert_eq!(held.state, State::Claimed);

    // One slot: while it holds the job and names it, it gets no second one.
    let holds = json!([{ "id": job["id"], "attempt": 1 }]);
    let (status, _) = work.claim(&token, &claim_of(PVE_01, &holds)).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn k10_7_the_reports_of_the_holder_take_a_job_to_its_end_with_its_log(pool: PgPool) {
    let work = Work::new(&pool).await;
    let token = work.daemons.token_for(PVE_01).await;
    let queued = work.enqueue(build()).await;
    let (_, job) = work.claim(&token, &claim_of(PVE_01, &json!([]))).await;

    let (status, answer) = work.report(&token, &report_of(&job, "preparing")).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    assert_eq!(answer["stop"], json!(null));
    let mut running = report_of(&job, "running");
    running["log"] = json!({ "from": 0, "text": "step 1\nAuthorization: Bearer abcdefghijklmnopqrstuvwxyz0123456789\n\u{1b}[31mred\u{202e}\n" });
    let (_, answer) = work.report(&token, &running).await;
    let sent = running["log"]["text"].as_str().unwrap().len();
    assert_eq!(answer["logEnd"].as_u64(), Some(sent as u64));

    // A part that does not start at the end of the log is dropped, and the answer says the end.
    let mut again = report_of(&job, "running");
    again["log"] = json!({ "from": 0, "text": "step 1 again\n" });
    let (status, answer) = work.report(&token, &again).await;
    assert_eq!(
        (status, answer["logEnd"].as_u64()),
        (StatusCode::OK, Some(sent as u64))
    );

    work.report(&token, &report_of(&job, "finishing")).await;
    let (status, _) = work.report(&token, &report_of(&job, "done")).await;
    assert_eq!(status, StatusCode::OK);

    let (ended, log) = work.job(queued.id).await;
    assert_eq!((ended.state, ended.holder), (State::Done, None));
    assert_eq!(ended.result.unwrap().outcome, "succeeded");
    assert!(log.starts_with("step 1\n"), "{log:?}");
    assert!(
        !log.contains("abcdefghijklmnopqrstuvwxyz0123456789"),
        "a secret is redacted again"
    );
    assert!(
        !log.contains('\u{1b}') && !log.contains('\u{202e}'),
        "{log:?}"
    );
    assert!(!log.contains("again"));

    // The job ended: one more report is not from a holder.
    let (status, refusal) = work.report(&token, &report_of(&job, "done")).await;
    assert_eq!(
        (status, refusal),
        (StatusCode::CONFLICT, json!({ "error": "NOT_HOLDER" }))
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn k10_7_a_build_whose_machine_went_silent_goes_to_a_different_machine(pool: PgPool) {
    let work = Work::new(&pool).await;
    let first = work.daemons.token_for(PVE_01).await;
    let second = work.daemons.token_for(DOCKER_HOST).await;
    let queued = work.enqueue(build()).await;
    let (_, job) = work.claim(&first, &claim_of(PVE_01, &json!([]))).await;
    work.report(&first, &report_of(&job, "preparing")).await;

    work.daemons.clock.advance(TimeDelta::seconds(91));
    let changed = jobs::end_lost(work.queue(), work.now()).await.unwrap();

    assert_eq!(changed, 1);
    let (back, _) = work.job(queued.id).await;
    assert_eq!((back.state, back.lost), (State::Queued, 1));
    let (status, refusal) = work.report(&first, &report_of(&job, "running")).await;
    assert_eq!(
        (status, refusal),
        (StatusCode::CONFLICT, json!({ "error": "NOT_HOLDER" }))
    );
    let (status, again) = work
        .claim(&second, &claim_of(DOCKER_HOST, &json!([])))
        .await;
    assert_eq!(status, StatusCode::OK, "{again}");
    assert_eq!(
        (id_of(&again), again["attempt"].as_u64()),
        (queued.id, Some(2))
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn k10_7_a_runner_whose_machine_went_silent_fails_and_is_not_given_again(pool: PgPool) {
    let work = Work::new(&pool).await;
    let token = work.daemons.token_for(PVE_01).await;
    let queued = work.enqueue(runner()).await;
    work.claim(&token, &claim_of(PVE_01, &json!([]))).await;

    work.daemons.clock.advance(TimeDelta::seconds(91));
    jobs::end_lost(work.queue(), work.now()).await.unwrap();

    let (failed, _) = work.job(queued.id).await;
    assert_eq!(failed.state, State::Failed);
    let (status, _) = work.claim(&token, &claim_of(PVE_01, &json!([]))).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn sf_011_a_claim_that_does_not_name_a_held_job_ends_that_attempt_with_no_lost_lease(
    pool: PgPool,
) {
    let work = Work::new(&pool).await;
    let token = work.daemons.token_for(PVE_01).await;
    let queued = work.enqueue(build()).await;
    work.claim(&token, &claim_of(PVE_01, &json!([]))).await;

    // The answer was lost: the machine claims again and names no job.
    let (status, job) = work.claim(&token, &claim_of(PVE_01, &json!([]))).await;

    assert_eq!(status, StatusCode::OK, "{job}");
    assert_eq!((id_of(&job), job["attempt"].as_u64()), (queued.id, Some(2)));
    assert_eq!(work.job(queued.id).await.0.lost, 0);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn k10_7_a_stop_reaches_the_holder_in_the_answer_to_its_next_report(pool: PgPool) {
    let work = Work::new(&pool).await;
    let token = work.daemons.token_for(PVE_01).await;
    let queued = work.enqueue(build()).await;
    let (_, job) = work.claim(&token, &claim_of(PVE_01, &json!([]))).await;
    work.report(&token, &report_of(&job, "preparing")).await;
    let asker = (Operator::hub(), ClientAddr(PERMITTED.parse().unwrap()));
    let now = work.now();

    let told = jobs::stop(work.queue(), (queued.id, StopReason::Operator), asker, now).await;

    assert_eq!(told.unwrap().unwrap().state, State::Preparing);
    let (_, answer) = work.report(&token, &report_of(&job, "preparing")).await;
    assert_eq!(answer["stop"], json!({ "reason": "operator" }));
    let (status, _) = work.report(&token, &report_of(&job, "stopped")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(work.job(queued.id).await.0.state, State::Stopped);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn sf_011_an_attempt_that_reports_with_no_end_is_ended_at_its_limit(pool: PgPool) {
    let work = Work::new(&pool).await;
    let token = work.daemons.token_for(PVE_01).await;
    let queued = work.enqueue(build()).await;
    let (_, job) = work.claim(&token, &claim_of(PVE_01, &json!([]))).await;
    work.report(&token, &report_of(&job, "preparing")).await;

    // Its limit is 600 seconds; it reports each 60 seconds for 15 minutes and 30 seconds.
    for _ in 0..15 {
        work.daemons.clock.advance(TimeDelta::seconds(60));
        work.report(&token, &report_of(&job, "preparing")).await;
    }
    assert_eq!(jobs::end_lost(work.queue(), work.now()).await.unwrap(), 1);

    let (ended, _) = work.job(queued.id).await;
    assert_eq!(
        (ended.state, ended.reason.as_deref()),
        (State::Stopped, Some("expired"))
    );
}
