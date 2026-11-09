//! The keeper (S-5, S-6, K4 §5, §10): a share ends at expiry, on `endShare`, and when the
//! owner's session ends (`endSession`, the tunnel's own end, hub shutdown); it leaves
//! `Session.shares` and writes `share.end` with its cause and an Info event.

use std::sync::Arc;

use chrono::TimeDelta;
use serde_json::json;
use sqlx::PgPool;
use tokio::sync::watch;
use uuid::Uuid;

use super::{Bench, CLIENT, bench, events, share_audits, share_end};
use crate::{
    clock::{Clock, FixedClock},
    events::Bus,
    ids::ShareId,
    sessions::{
        Actor,
        share::{
            Sharing, model::ShareAccess, opening::share_session, registry::Share, terms::Terms,
        },
        shut_down,
        tests::fixture::{address, now, operator},
    },
};

/// The event every end of `cause` writes about `docker-host`.
fn end_event(sentence: &str) -> (String, String, Option<String>, String) {
    (
        "info".to_owned(),
        "session".to_owned(),
        Some("docker-host".to_owned()),
        sentence.to_owned(),
    )
}

/// The `share.end` entries written so far, as `(share id, change)`, without waiting.
async fn ends_now(pool: &PgPool) -> Vec<(String, serde_json::Value)> {
    share_audits(pool)
        .await
        .into_iter()
        .filter(|(action, ..)| action == "end")
        .map(|(_, id, _, change)| (id, change))
        .collect()
}

/// The sharing a resolver would build on the bench, with a clock fixed at `now()`.
pub(super) fn sharing(bench: &Bench) -> Sharing {
    let clock: Arc<dyn Clock> = Arc::new(FixedClock(now()));
    Sharing {
        db: bench.pool.clone(),
        bus: Bus::new(16),
        clock,
        live: bench.live.clone(),
    }
}

/// Shares the bench's session for `lifetime`, past K1's bounds, as the core allows.
pub(super) async fn shared_for(bench: &Bench, lifetime: TimeDelta) -> ShareId {
    let by = Actor {
        operator: operator(),
        client: address(CLIENT),
        at: now(),
    };
    let terms = Terms {
        access: ShareAccess::View,
        lifetime,
        max_viewers: 3,
    };
    let (share, _token) = share_session(&sharing(bench), &by, bench.session, terms)
        .await
        .unwrap();
    share.id
}

/// Makes each `share.end` insert take 300 ms, so a caller that does not wait for the keeper
/// returns before the entry commits.
async fn slow_share_ends(pool: &PgPool) {
    for statement in [
        "create function public.slow_share_end() returns trigger language plpgsql as \
         $$ begin perform pg_sleep(0.3); return new; end $$",
        "create trigger slow_share_end before insert on events.audit_log for each row \
         when (new.target_type = 'share' and new.action = 'end') \
         execute function public.slow_share_end()",
    ] {
        sqlx::query(statement).execute(pool).await.unwrap();
    }
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn ra_035_end_share_ends_it_once_with_cause_ended_from_the_caller_s_address(pool: PgPool) {
    let bench = bench(&pool).await;
    let id = bench.shared().await["share"]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let other = bench.shared().await["share"]["id"]
        .as_str()
        .unwrap()
        .to_owned();

    let ended = bench.end_share_as(operator(), "100.64.0.9", &id).await;
    assert!(ended.errors.is_empty(), "{:?}", ended.errors);
    assert_eq!(bench.listed().await, [other], "gone at once");

    let (client, change) = share_end(&pool, &id).await;
    assert_eq!(client, "100.64.0.9");
    assert_eq!(change, json!({ "cause": "ended" }));
    assert_eq!(
        events(&pool).await.last().unwrap(),
        &end_event("An operator ended a session share.")
    );
    assert_eq!(bench.live.shares().hashes().len(), 1, "removed");
    assert_eq!(ends_now(&pool).await.len(), 1, "the other share goes on");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn ra_035_a_share_ends_at_its_expiry_and_not_before(pool: PgPool) {
    let bench = bench(&pool).await;
    // Long enough that a loaded machine still lists it before it expires.
    let id = shared_for(&bench, TimeDelta::seconds(2)).await;
    assert_eq!(
        bench.listed().await,
        [id.to_string()],
        "live until it expires"
    );
    assert_eq!(ends_now(&pool).await, []);

    let (client, change) = share_end(&pool, &id.to_string()).await;
    assert_eq!(client, CLIENT, "where the owner shared from");
    assert_eq!(change, json!({ "cause": "expired" }));
    assert_eq!(
        events(&pool).await.last().unwrap(),
        &end_event("A session share expired.")
    );
    assert_eq!(bench.listed().await, Vec::<String>::new());
    assert!(bench.live.shares().hashes().is_empty());
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_5_end_session_returns_once_its_shares_have_ended(pool: PgPool) {
    let mut bench = bench(&pool).await;
    bench.ends_when_told();
    slow_share_ends(&pool).await;
    let id = bench.shared().await["share"]["id"]
        .as_str()
        .unwrap()
        .to_owned();

    let end = "mutation($id: ID!) { endSession(id: $id) { endReason } }";
    let response = bench
        .run(end, json!({ "id": bench.session.to_string() }))
        .await;
    assert!(response.errors.is_empty(), "{:?}", response.errors);

    // No wait: `endSession` returned after the keeper wrote.
    assert_eq!(
        ends_now(&pool).await,
        [(id, json!({ "cause": "session_ended" }))]
    );
    assert_eq!(
        events(&pool).await.last().unwrap(),
        &end_event("A session share ended with its session.")
    );
    assert_eq!(bench.listed().await, Vec::<String>::new());
    assert!(bench.live.shares().hashes().is_empty());
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_5_a_share_ends_when_the_owner_s_tunnel_ends(pool: PgPool) {
    let mut bench = bench(&pool).await;
    let id = bench.shared().await["share"]["id"]
        .as_str()
        .unwrap()
        .to_owned();

    // The tunnel task ends on its own (the client left) and deregisters.
    drop(bench.tunnel.take());

    let (client, change) = share_end(&pool, &id).await;
    assert_eq!(
        (client.as_str(), change),
        (CLIENT, json!({ "cause": "session_ended" }))
    );
    assert_eq!(bench.listed().await, Vec::<String>::new());
    assert!(bench.live.shares().hashes().is_empty());
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_5_hub_shutdown_ends_every_share_before_it_returns(pool: PgPool) {
    let mut bench = bench(&pool).await;
    bench.ends_when_told();
    slow_share_ends(&pool).await;
    let first = bench.shared().await["share"]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let second = bench.shared().await["share"]["id"]
        .as_str()
        .unwrap()
        .to_owned();

    shut_down(&pool, &bench.live, now()).await.unwrap();

    let mut ended = ends_now(&pool).await;
    ended.sort_by(|a, b| a.0.cmp(&b.0));
    let cause = json!({ "cause": "session_ended" });
    assert_eq!(ended, [(first, cause.clone()), (second, cause)]);
    assert!(bench.live.shares().hashes().is_empty());
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn ra_035_an_ended_session_lists_no_share_whatever_the_registry_holds(pool: PgPool) {
    let bench = bench(&pool).await;
    let share = Share {
        id: ShareId::from(Uuid::now_v7()),
        session: bench.session,
        owner: operator(),
        access: ShareAccess::View,
        max_viewers: 3,
        viewers: 0,
        created_at: now(),
        expires_at: now() + TimeDelta::hours(1),
        end: watch::Sender::new(None),
    };
    let token = crate::sessions::tickets::TicketToken::from_random([9; 32]);
    let _ended = bench.live.shares().open(token.hash(), share).unwrap();
    assert_eq!(bench.listed().await.len(), 1);

    sqlx::query(
        "update sessions.sessions set ended_at = started_at, end_reason = 'remote_closed' \
         where id = $1",
    )
    .bind(bench.session)
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(bench.listed().await, Vec::<String>::new());
}
