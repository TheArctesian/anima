use axum::http::StatusCode;
use chrono::TimeDelta;
use serde_json::{Value, json};
use sqlx::PgPool;

use super::{Work, build, claim_of, report_of};
use crate::daemon::harness::{DOCKER_HOST, PVE_01};

fn refusal(code: &str) -> Value {
    json!({ "error": code })
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn k10_11_the_token_is_the_credential_and_names_the_one_device_that_can_claim(pool: PgPool) {
    let work = Work::new(&pool).await;
    let token = work.daemons.token_for(PVE_01).await;
    work.enqueue(build()).await;

    let no_token = work
        .post("claim", None, &claim_of(PVE_01, &json!([])))
        .await;
    let other = work.claim(&token, &claim_of(DOCKER_HOST, &json!([]))).await;
    let not_json = work.claim(&token, &json!({ "device": PVE_01 })).await;

    assert_eq!(
        no_token,
        (StatusCode::UNAUTHORIZED, refusal("TOKEN_REFUSED"))
    );
    assert_eq!(other, (StatusCode::FORBIDDEN, refusal("WRONG_DEVICE")));
    assert_eq!(not_json, (StatusCode::BAD_REQUEST, refusal("MALFORMED")));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn k10_4_a_machine_whose_switch_is_off_or_that_is_not_clean_gets_no_job(pool: PgPool) {
    let work = Work::new(&pool).await;
    let token = work.daemons.token_for(PVE_01).await;
    work.enqueue(build()).await;
    for (member, value) in [("switch", json!("off")), ("unclean", json!(true))] {
        let mut claim = claim_of(PVE_01, &json!([]));
        claim[member] = value;

        let (status, _) = work.claim(&token, &claim).await;

        assert_eq!(status, StatusCode::NO_CONTENT, "{member}");
    }
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn k10_7_a_report_of_a_machine_that_does_not_hold_the_job_is_not_holder(pool: PgPool) {
    let work = Work::new(&pool).await;
    let holder = work.daemons.token_for(PVE_01).await;
    let other = work.daemons.token_for(DOCKER_HOST).await;
    work.enqueue(build()).await;
    let (_, job) = work.claim(&holder, &claim_of(PVE_01, &json!([]))).await;

    let by_other = work.report(&other, &report_of(&job, "preparing")).await;
    let out_of_order = work.report(&holder, &report_of(&job, "running")).await;
    let mut no_result = report_of(&job, "preparing");
    no_result["result"] = report_of(&job, "done")["result"].clone();
    let with_result = work.report(&holder, &no_result).await;
    let unknown = work
        .report(&holder, &json!({ "id": "0192f6aa-0000-7000-8000-00000000ffff", "attempt": 1, "state": "preparing" }))
        .await;

    assert_eq!(by_other, (StatusCode::CONFLICT, refusal("NOT_HOLDER")));
    assert_eq!(out_of_order, (StatusCode::CONFLICT, refusal("NOT_HOLDER")));
    assert_eq!(with_result, (StatusCode::BAD_REQUEST, refusal("MALFORMED")));
    assert_eq!(unknown, (StatusCode::CONFLICT, refusal("NOT_HOLDER")));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn k10_11_a_machine_that_refuses_five_jobs_in_one_hour_gets_no_work_for_one_hour(
    pool: PgPool,
) {
    let work = Work::new(&pool).await;
    let token = work.daemons.token_for(PVE_01).await;
    for _ in 0..6 {
        work.enqueue(build()).await;
        work.daemons.clock.advance(TimeDelta::seconds(1));
    }
    for _ in 0..5 {
        let (status, job) = work.claim(&token, &claim_of(PVE_01, &json!([]))).await;
        assert_eq!(status, StatusCode::OK, "{job}");
        let (status, _) = work.report(&token, &report_of(&job, "refused")).await;
        assert_eq!(status, StatusCode::OK);
    }

    let off = work.claim(&token, &claim_of(PVE_01, &json!([]))).await;
    assert_eq!(off, (StatusCode::FORBIDDEN, refusal("WORK_OFF")));

    work.daemons.clock.advance(TimeDelta::minutes(61));
    let (status, job) = work.claim(&token, &claim_of(PVE_01, &json!([]))).await;
    assert_eq!(status, StatusCode::OK, "{job}");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn k10_8_no_secret_is_given_and_the_refusal_does_not_say_why(pool: PgPool) {
    let work = Work::new(&pool).await;
    let token = work.daemons.token_for(PVE_01).await;
    work.enqueue(super::runner()).await;
    let (_, job) = work.claim(&token, &claim_of(PVE_01, &json!([]))).await;
    let asked = json!({ "id": job["id"], "attempt": 1, "purpose": "jit_config" });

    let held = work.post("secret", Some(&token), &asked).await;
    let no_token = work.post("secret", None, &asked).await;
    let too_large = work
        .post(
            "secret",
            Some(&token),
            &json!({ "id": job["id"], "attempt": 1, "purpose": "x".repeat(5000) }),
        )
        .await;

    assert_eq!(held, (StatusCode::CONFLICT, refusal("SECRET_REFUSED")));
    assert_eq!(
        no_token,
        (StatusCode::UNAUTHORIZED, refusal("TOKEN_REFUSED"))
    );
    assert_eq!(
        too_large,
        (StatusCode::PAYLOAD_TOO_LARGE, refusal("TOO_LARGE"))
    );
}
