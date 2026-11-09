//! A domain expiry check through the attempt path against a local registry (D-29, P3-003):
//! the stored status and expiry date, and a probe of a domain the check no longer names.

use chrono::TimeDelta;
use sqlx::PgPool;

use super::support::{
    DOCKER_HOST, add_check, beats, context, database_now, device, due_at, new_check, stored, vault,
};
use crate::{
    checks::{
        self, CheckChange, CheckSettings, Definition,
        config::CheckConfig,
        probe::rdap::tests::registry::{expiring_at, rdap, registry},
        scheduler::attempt,
    },
    ids::CheckId,
    status::Status,
};

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
}
