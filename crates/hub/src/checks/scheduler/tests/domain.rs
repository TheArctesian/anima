//! A domain expiry check through the attempt path against a local registry (D-29, P3-003):
//! the stored status and expiry date, a probe of a domain the check no longer names, and the
//! hour after the last query before the next, on every path that sets a due time (R-102). The
//! expiry notices such a beat queues: none under maintenance (Section 6.5, NOT-006), and none
//! for a domain the check no longer names.

use std::time::Duration;

use chrono::{DateTime, TimeDelta, Utc};
use sqlx::PgPool;

use super::support::{
    DOCKER_HOST, add_check, beats, context, database_now, device, due_at, new_check, notify_on,
    queued, rng, stored, vault,
};
use crate::{
    checks::{
        self, Attempt, CheckChange, CheckSettings, Definition,
        config::CheckConfig,
        probe::{
            ProbeResult,
            rdap::tests::{
                registry::{expiring_at, rdap, registry},
                server::Server,
            },
        },
        scheduler::{
            attempt,
            tick::{Limits, Scheduler},
        },
    },
    devices,
    ids::CheckId,
    status::Status,
};

/// What the registry is asked for the check's domain.
const QUERY: &str = "/rdap/domain/example.test";
const HOUR: TimeDelta = TimeDelta::hours(1);

fn domain_expiry(domain: &str) -> Definition {
    Definition::Config(CheckConfig::DomainExpiry {
        domain: domain.to_owned(),
        warn_days: 30,
        timeout_ms: 5000,
    })
}

/// A domain expiry check on `example.test` with no retries, at the hourly floor.
async fn add_domain_check(pool: &PgPool) -> CheckId {
    let mut new = new_check(
        device(DOCKER_HOST),
        "Domain",
        domain_expiry("example.test"),
        0,
    );
    new.settings = CheckSettings {
        interval_seconds: 3600,
        ..new.settings
    };
    add_check(pool, new).await
}

/// A scheduler whose domain expiry probes ask `server`.
fn domain_scheduler(pool: &PgPool, server: &Server) -> Scheduler {
    let mut ctx = context(pool);
    ctx.rdap = rdap(server);
    Scheduler::new(
        ctx,
        Limits {
            probes: 32,
            due_per_tick: 256,
        },
    )
}

/// One tick at `at`, then the attempts it started.
async fn tick_at(scheduler: &mut Scheduler, at: DateTime<Utc>) {
    scheduler.tick(at, &mut rng()).await;
    scheduler.finish_running(at).await;
}

/// Records that the check's last beat, and so its last RDAP query, was at `at`.
async fn beaten_at(pool: &PgPool, id: CheckId, at: DateTime<Utc>) {
    sqlx::query("UPDATE checks.checks SET last_beat_at = $2 WHERE id = $1")
        .bind(id)
        .bind(at)
        .execute(pool)
        .await
        .unwrap();
}

/// Makes every beat write fail after the probe ran.
async fn refuse_beats(pool: &PgPool) {
    for sql in [
        "CREATE FUNCTION checks.refuse_beat() RETURNS trigger LANGUAGE plpgsql AS \
         $$ BEGIN RAISE EXCEPTION 'refused'; END $$",
        "CREATE TRIGGER refuse_beat BEFORE INSERT ON checks.uptime_minute \
         FOR EACH ROW EXECUTE FUNCTION checks.refuse_beat()",
    ] {
        sqlx::query(sql).execute(pool).await.unwrap();
    }
}

/// A domain expiry check last queried 10 minutes before `now`, its registry and a
/// scheduler that has not ticked yet.
struct Queried {
    now: DateTime<Utc>,
    id: CheckId,
    server: Server,
    scheduler: Scheduler,
}

async fn queried_10_minutes_ago(pool: &PgPool) -> Queried {
    let now = database_now(pool).await;
    let server = registry(expiring_at(now + TimeDelta::days(90))).await;
    let id = add_domain_check(pool).await;
    beaten_at(pool, id, now - TimeDelta::minutes(10)).await;
    let scheduler = domain_scheduler(pool, &server);
    Queried {
        now,
        id,
        server,
        scheduler,
    }
}

/// The check asks nothing at `now` and is moved to the hour after its last query or later;
/// it runs at that new due time.
async fn assert_waits_for_the_hour(pool: &PgPool, mut queried: Queried) {
    let Queried { now, id, .. } = queried;
    tick_at(&mut queried.scheduler, now).await;

    assert_eq!(queried.server.requests_for(QUERY), 0);
    let next = stored(pool, id)
        .await
        .state
        .next_due_at
        .expect("a due time");
    assert!(next >= now + TimeDelta::minutes(50), "{next}");
    tick_at(&mut queried.scheduler, next - TimeDelta::seconds(1)).await;
    assert_eq!(queried.server.requests_for(QUERY), 0);
    tick_at(&mut queried.scheduler, next).await;
    assert_eq!(queried.server.requests_for(QUERY), 1);
}

tick_test! {
    async fn d_29_an_attempt_stores_the_status_and_the_expiry_date(pool: PgPool) {
        let now = database_now(&pool).await;
        let not_after = now + TimeDelta::days(12);
        let server = registry(expiring_at(not_after)).await;
        let id = add_domain_check(&pool).await;
        due_at(&pool, id, now).await;
        let due = checks::due_checks(&pool, now, 10).await.unwrap().remove(0);
        let mut ctx = context(&pool);
        ctx.rdap = rdap(&server);

        let finished = attempt::run(&ctx, due, now).await;

        assert!(finished.recorded.is_some());
        let check = stored(&pool, id).await;
        assert_eq!(check.state.status, Status::Degraded, "12 days left, 30 to warn");
        assert_eq!(check.domain_not_after, Some(not_after));
        assert_eq!(check.domain_days_left(now), Some(12));
        assert_eq!(beats(&pool, id).await, ["degraded"]);
        assert_eq!(server.requests_for("/rdap/domain/example.test"), 1);
    }

    async fn d_29_a_probe_of_a_domain_the_check_no_longer_names_keeps_no_date(pool: PgPool) {
        let now = database_now(&pool).await;
        let server = registry(expiring_at(now + TimeDelta::days(12))).await;
        let id = add_domain_check(&pool).await;
        due_at(&pool, id, now).await;
        let due = checks::due_checks(&pool, now, 10).await.unwrap().remove(0);
        let change = CheckChange {
            definition: Some(domain_expiry("other.test")),
            ..CheckChange::default()
        };
        let mut conn = pool.acquire().await.unwrap();
        checks::update_check(&mut conn, &vault(), id, change).await.unwrap();
        let mut ctx = context(&pool);
        ctx.rdap = rdap(&server);

        // The attempt was due for the old domain and asks about it.
        attempt::run(&ctx, due, now).await;

        assert_eq!(server.requests_for("/rdap/domain/example.test"), 1);
        assert_eq!(stored(&pool, id).await.domain_not_after, None);
    }

    async fn d_29_an_expired_domain_check_is_down_at_once_and_next_due_after_the_interval(pool: PgPool) {
        let now = database_now(&pool).await;
        let server = registry(expiring_at(now - TimeDelta::days(1))).await;
        let id = add_domain_check(&pool).await;
        due_at(&pool, id, now).await;
        let due = checks::due_checks(&pool, now, 10).await.unwrap().remove(0);
        let mut ctx = context(&pool);
        ctx.rdap = rdap(&server);

        attempt::run(&ctx, due, now).await;

        let state = stored(&pool, id).await.state;
        assert_eq!(state.status, Status::Down, "no retry holds it Unknown");
        assert_eq!(state.next_due_at, Some(now + TimeDelta::seconds(3600)), "not the 5-second retry");
        assert_eq!(beats(&pool, id).await, ["down"]);
    }

    async fn r_102_a_failed_write_after_an_rdap_query_waits_the_whole_interval(pool: PgPool) {
        let now = database_now(&pool).await;
        let server = registry(expiring_at(now + TimeDelta::days(90))).await;
        let id = add_domain_check(&pool).await;
        due_at(&pool, id, now).await;
        refuse_beats(&pool).await;
        let mut scheduler = domain_scheduler(&pool, &server);

        tick_at(&mut scheduler, now).await;

        assert_eq!(server.requests_for(QUERY), 1);
        assert_eq!(beats(&pool, id).await.len(), 0, "the write failed");
        let next = stored(&pool, id).await.state.next_due_at;
        assert_eq!(next, Some(now + HOUR), "not the 5-second retry interval");
        tick_at(&mut scheduler, now + HOUR - TimeDelta::seconds(1)).await;
        assert_eq!(server.requests_for(QUERY), 1);
    }

    async fn r_102_a_resumed_check_waits_the_hour_after_its_last_query(pool: PgPool) {
        let queried = queried_10_minutes_ago(&pool).await;
        let mut conn = pool.acquire().await.unwrap();
        checks::set_paused(&mut conn, queried.id, true).await.unwrap();
        let resumed = checks::set_paused(&mut conn, queried.id, false).await.unwrap();
        assert_eq!(resumed.state.next_due_at, None, "resume clears the due time");

        assert_waits_for_the_hour(&pool, queried).await;
    }

    async fn r_102_after_a_restart_a_check_waits_the_hour_after_its_last_query(pool: PgPool) {
        let queried = queried_10_minutes_ago(&pool).await;
        // Due before this scheduler started, as after a hub restart.
        due_at(&pool, queried.id, queried.now - TimeDelta::seconds(1)).await;

        assert_waits_for_the_hour(&pool, queried).await;
    }

    async fn r_102_a_due_time_before_the_hour_is_moved_to_it(pool: PgPool) {
        let queried = queried_10_minutes_ago(&pool).await;
        due_at(&pool, queried.id, queried.now).await;

        assert_waits_for_the_hour(&pool, queried).await;
    }

    async fn r_102_a_check_with_no_beat_yet_runs_at_once(pool: PgPool) {
        let now = database_now(&pool).await;
        let server = registry(expiring_at(now + TimeDelta::days(90))).await;
        let id = add_domain_check(&pool).await;
        due_at(&pool, id, now).await;
        let mut scheduler = domain_scheduler(&pool, &server);

        tick_at(&mut scheduler, now).await;

        assert_eq!(server.requests_for(QUERY), 1);
        assert_eq!(beats(&pool, id).await, ["up"]);
    }
}

/// An answered probe at `at` of `domain` whose certificate and registration both end at
/// `ends`, recorded under maintenance or not.
async fn expiring(
    pool: &PgPool,
    id: CheckId,
    (at, ends): (DateTime<Utc>, DateTime<Utc>),
    (domain, maintained): (&str, bool),
) {
    let mut result = ProbeResult::up(Duration::from_millis(1));
    result.cert_not_after = Some(ends);
    result.domain_not_after = Some(ends);
    let device = devices::get(pool, device(DOCKER_HOST)).await.unwrap();
    let attempt = Attempt {
        result: &result,
        in_maintenance: maintained,
        at,
        domain: Some(domain),
        device: &device,
    };
    let mut conn = pool.acquire().await.unwrap();
    let beat = checks::record_attempt(&mut conn, id, &attempt)
        .await
        .unwrap();
    assert!(beat.is_some(), "the beat is written");
}

tick_test! {
    async fn not_006_an_expiry_seen_under_maintenance_queues_nothing(pool: PgPool) {
        let id = add_domain_check(&pool).await;
        notify_on(&pool, id).await;
        let at = database_now(&pool).await;

        let ends = at + TimeDelta::days(3);
        expiring(&pool, id, (at, ends), ("example.test", true)).await;
        assert_eq!(queued(&pool, id).await, Vec::<String>::new());

        let later = at + TimeDelta::seconds(1);
        expiring(&pool, id, (later, ends), ("example.test", false)).await;
        assert_eq!(queued(&pool, id).await, ["certificate_expiry", "domain_expiry"]);
    }

    async fn d_29_a_date_for_a_domain_the_check_no_longer_names_queues_no_notice(pool: PgPool) {
        let id = add_domain_check(&pool).await;
        notify_on(&pool, id).await;
        let change = CheckChange {
            definition: Some(domain_expiry("other.test")),
            ..CheckChange::default()
        };
        let mut conn = pool.acquire().await.unwrap();
        checks::update_check(&mut conn, &vault(), id, change).await.unwrap();
        let at = database_now(&pool).await;

        let ends = at + TimeDelta::days(3);
        expiring(&pool, id, (at, ends), ("example.test", false)).await;
        assert_eq!(queued(&pool, id).await, ["certificate_expiry"], "no domain notice");

        let later = at + TimeDelta::seconds(1);
        expiring(&pool, id, (later, ends), ("other.test", false)).await;
        assert_eq!(queued(&pool, id).await, ["certificate_expiry", "domain_expiry"]);
    }
}
