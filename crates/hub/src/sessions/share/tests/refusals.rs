//! Who may share what, and when (S-3, S-4, RA-035): an unknown session is `NOT_FOUND`, another
//! operator's `FORBIDDEN`, an ended or not yet open one `CONFLICT`, and a hub that is full or
//! shutting down `UNAVAILABLE`; `endShare` likewise. A refusal shares nothing and audits nothing.

use std::time::Duration;

use serde_json::json;
use sqlx::PgPool;

use super::{
    Bench, CLIENT, END, SHARE, bench, bench_waiting, error_of, other_operator, share_audits,
    started,
};
use crate::{ids::SessionId, sessions::tests::fixture::operator};

/// `shareSession` for `session` with K1's defaults, as the owner; its error's code.
async fn code_for(bench: &Bench, session: SessionId) -> String {
    let variables = json!({ "id": session.to_string(), "input": {} });
    error_of(&bench.run(SHARE, variables).await).0
}

/// No share is held and none was audited.
async fn nothing_shared(bench: &Bench) {
    assert!(bench.live.shares().hashes().is_empty());
    assert_eq!(share_audits(&bench.pool).await, []);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn s_4_an_unknown_session_is_not_found(pool: PgPool) {
    let bench = bench(&pool).await;
    let unknown: SessionId = "0190a000-0000-7000-8010-0000000000ee".parse().unwrap();
    let variables = json!({ "id": unknown.to_string(), "input": {} });
    let (code, message) = error_of(&bench.run(SHARE, variables).await);
    assert_eq!(code, "NOT_FOUND");
    assert_eq!(message, format!("No such session {unknown}."));
    let malformed = json!({ "id": "not-an-id", "input": {} });
    assert_eq!(error_of(&bench.run(SHARE, malformed).await).0, "VALIDATION");
    nothing_shared(&bench).await;
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn ra_035_another_operator_s_session_is_forbidden(pool: PgPool) {
    let bench = bench(&pool).await;
    let variables = json!({ "id": bench.session.to_string(), "input": {} });
    let response = bench
        .run_as((other_operator(), CLIENT), SHARE, variables)
        .await;
    assert_eq!(error_of(&response).0, "FORBIDDEN");
    nothing_shared(&bench).await;
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn s_4_a_session_not_yet_open_or_ended_is_a_conflict(pool: PgPool) {
    let bench = bench(&pool).await;
    // Started, its ticket not yet redeemed: a pending ticket is not an open tunnel.
    let (pending, _token) = started(&pool, &bench.live).await;
    assert_eq!(code_for(&bench, pending).await, "CONFLICT");

    // `endSession` withdraws the ticket and ends the row.
    let end = "mutation($id: ID!) { endSession(id: $id) { endReason } }";
    let ended = bench.run(end, json!({ "id": pending.to_string() })).await;
    assert!(ended.errors.is_empty(), "{:?}", ended.errors);
    let (code, message) = error_of(
        &bench
            .run(SHARE, json!({ "id": pending.to_string(), "input": {} }))
            .await,
    );
    assert_eq!(code, "CONFLICT");
    assert!(message.contains("not open"), "{message}");
    nothing_shared(&bench).await;
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn s_4_an_ended_row_is_a_conflict_even_while_its_tunnel_is_registered(pool: PgPool) {
    let bench = bench(&pool).await;
    sqlx::query(
        "update sessions.sessions set ended_at = started_at, end_reason = 'remote_closed' \
         where id = $1",
    )
    .bind(bench.session)
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(code_for(&bench, bench.session).await, "CONFLICT");
    nothing_shared(&bench).await;
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn s_4_a_session_whose_tunnel_is_ending_is_a_conflict(pool: PgPool) {
    // The tunnel is told to end and does not deregister within the wait: its row stays live.
    let bench = bench_waiting(&pool, Duration::from_millis(50)).await;
    let end = "mutation($id: ID!) { endSession(id: $id) { endReason } }";
    let response = bench
        .run(end, json!({ "id": bench.session.to_string() }))
        .await;
    assert!(response.errors.is_empty(), "{:?}", response.errors);
    assert_eq!(code_for(&bench, bench.session).await, "CONFLICT");
    nothing_shared(&bench).await;
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn s_4_a_session_whose_tunnel_deregistered_is_a_conflict(pool: PgPool) {
    let mut bench = bench(&pool).await;
    drop(bench.tunnel.take());
    assert_eq!(code_for(&bench, bench.session).await, "CONFLICT");
    nothing_shared(&bench).await;
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn d_9_the_33rd_live_share_is_unavailable(pool: PgPool) {
    let bench = bench(&pool).await;
    let mut ids = Vec::new();
    for _ in 0..32 {
        ids.push(
            bench.shared().await["share"]["id"]
                .as_str()
                .unwrap()
                .to_owned(),
        );
    }
    assert_eq!(code_for(&bench, bench.session).await, "UNAVAILABLE");
    assert_eq!(
        share_audits(&pool).await.len(),
        32,
        "the refused share wrote nothing"
    );

    let ended = bench.end_share_as(operator(), CLIENT, &ids[0]).await;
    assert!(ended.errors.is_empty(), "{:?}", ended.errors);
    for _ in 0..250 {
        if bench.live.shares().hashes().len() < 32 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(bench.share(json!({})).await.errors.is_empty(), "room again");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn s_4_a_hub_that_is_shutting_down_shares_nothing(pool: PgPool) {
    let bench = bench(&pool).await;
    drop(bench.live.shut_down());
    assert_eq!(code_for(&bench, bench.session).await, "UNAVAILABLE");
    nothing_shared(&bench).await;
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn ra_035_only_the_owner_ends_a_share_and_an_ended_one_is_not_found(pool: PgPool) {
    let bench = bench(&pool).await;
    let id = bench.shared().await["share"]["id"]
        .as_str()
        .unwrap()
        .to_owned();

    let (code, message) = error_of(&bench.end_share_as(other_operator(), CLIENT, &id).await);
    assert_eq!(code, "FORBIDDEN");
    assert!(
        message.starts_with("Only the operator who started"),
        "{message}"
    );
    assert_eq!(
        bench.listed().await,
        std::slice::from_ref(&id),
        "the share goes on"
    );

    let unknown = "0190a000-0000-7000-8020-0000000000ee";
    let (code, message) = error_of(&bench.end_share_as(operator(), CLIENT, unknown).await);
    assert_eq!(code, "NOT_FOUND");
    assert_eq!(message, format!("No such share {unknown}."));
    let malformed = bench.run(END, json!({ "id": "share-1" })).await;
    assert_eq!(error_of(&malformed).0, "VALIDATION");

    let ended = bench.end_share_as(operator(), CLIENT, &id).await;
    assert!(ended.errors.is_empty(), "{:?}", ended.errors);
    assert_eq!(ended.data.into_json().unwrap(), json!({ "endShare": id }));
    let again = bench.end_share_as(operator(), CLIENT, &id).await;
    assert_eq!(error_of(&again).0, "NOT_FOUND");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn s_6_a_share_whose_audit_entry_cannot_be_written_is_not_held(pool: PgPool) {
    let bench = bench(&pool).await;
    for statement in [
        "create function public.refuse_share() returns trigger language plpgsql as \
         $$ begin raise exception 'refused for the test'; end $$",
        "create trigger refuse_share before insert on events.audit_log for each row \
         when (new.target_type = 'share') execute function public.refuse_share()",
    ] {
        sqlx::query(statement).execute(&pool).await.unwrap();
    }
    let (code, message) = error_of(&bench.share(json!({})).await);
    assert_eq!(code, "INTERNAL");
    assert!(!message.contains("refused"), "no detail: {message}");
    nothing_shared(&bench).await;
}
