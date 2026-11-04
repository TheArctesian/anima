//! The watchdog through the tick: one Error event per lateness episode, never for the hub's
//! own downtime, retried when it cannot be written (CHK-010).

use chrono::{DateTime, TimeDelta, Utc};
use sqlx::PgPool;

use super::support::{
    DOCKER_HOST, Listener, add, allow_event_writes, database_now, due_at, events_of,
    fail_event_writes, on_loopback, rng, scheduler, tcp,
};
use crate::{checks::scheduler::tick::Scheduler, events::Bus, ids::DeviceId};

const LATE: &str = "Check Service port on docker-host is late by more than 2 intervals";

/// A scheduler that started a minute before `now`.
async fn started_before(pool: &PgPool, now: DateTime<Utc>) -> Scheduler {
    let mut scheduler = scheduler(pool, &Bus::new(64));
    scheduler
        .tick(now - TimeDelta::seconds(60), &mut rng())
        .await;
    scheduler
}

async fn errors(pool: &PgPool, device: DeviceId) -> Vec<String> {
    events_of(pool, device)
        .await
        .into_iter()
        .filter(|(severity, source, _)| severity == "error" && source == "check")
        .map(|(_, _, message)| message)
        .collect()
}

tick_test! {
    async fn chk_010_a_late_check_gives_exactly_one_error_event(pool: PgPool) {
        let listener = Listener::start().await;
        let device = on_loopback(&pool, DOCKER_HOST).await;
        let late = add(&pool, device, listener.silent_tls(), 2).await;
        let now = database_now(&pool).await;
        let mut scheduler = started_before(&pool, now).await;
        due_at(&pool, late, now - TimeDelta::seconds(25)).await;

        scheduler.tick(now, &mut rng()).await;
        listener.wait_for(1).await;
        scheduler.tick(now + TimeDelta::seconds(1), &mut rng()).await;
        scheduler.finish_running(now + TimeDelta::seconds(1)).await;

        assert_eq!(errors(&pool, device).await, [LATE]);
    }

    async fn chk_010_a_check_late_again_after_it_ran_gives_a_new_event(pool: PgPool) {
        let listener = Listener::start().await;
        let device = on_loopback(&pool, DOCKER_HOST).await;
        let late = add(&pool, device, tcp(listener.port), 2).await;
        let now = database_now(&pool).await;
        let mut scheduler = started_before(&pool, now).await;
        for at in [now, now + TimeDelta::seconds(2)] {
            due_at(&pool, late, at - TimeDelta::seconds(25)).await;
            scheduler.tick(at, &mut rng()).await;
            scheduler.finish_running(at).await;
        }

        assert_eq!(errors(&pool, device).await, [LATE, LATE]);
    }

    async fn chk_010_a_probe_slower_than_2_intervals_is_one_episode(pool: PgPool) {
        let listener = Listener::start().await;
        let device = on_loopback(&pool, DOCKER_HOST).await;
        let late = add(&pool, device, listener.silent_tls(), 2).await;
        let now = database_now(&pool).await;
        let mut scheduler = started_before(&pool, now).await;
        due_at(&pool, late, now - TimeDelta::seconds(25)).await;

        scheduler.tick(now, &mut rng()).await;
        listener.wait_for(1).await;
        let slow = now + TimeDelta::seconds(30);
        scheduler.finish_running(slow).await;
        scheduler.tick(slow, &mut rng()).await;
        listener.wait_for(2).await;
        scheduler.finish_running(slow).await;

        assert_eq!(errors(&pool, device).await, [LATE]);
    }

    async fn chk_010_a_check_within_2_intervals_is_not_late(pool: PgPool) {
        let listener = Listener::start().await;
        let device = on_loopback(&pool, DOCKER_HOST).await;
        let id = add(&pool, device, tcp(listener.port), 2).await;
        let now = database_now(&pool).await;
        let mut scheduler = started_before(&pool, now).await;
        due_at(&pool, id, now - TimeDelta::seconds(20)).await;

        scheduler.tick(now, &mut rng()).await;
        scheduler.finish_running(now).await;

        assert_eq!(errors(&pool, device).await.len(), 0);
    }

    async fn chk_010_the_hubs_own_downtime_is_not_reported_as_late(pool: PgPool) {
        let listener = Listener::start().await;
        let device = on_loopback(&pool, DOCKER_HOST).await;
        let id = add(&pool, device, tcp(listener.port), 2).await;
        let now = database_now(&pool).await;
        due_at(&pool, id, now - TimeDelta::hours(1)).await;
        let mut scheduler = scheduler(&pool, &Bus::new(64));

        scheduler.tick(now, &mut rng()).await;
        scheduler.tick(now + TimeDelta::seconds(15), &mut rng()).await;
        scheduler.finish_running(now + TimeDelta::seconds(15)).await;

        assert_eq!(errors(&pool, device).await.len(), 0);
    }

    async fn chk_010_a_late_event_that_cannot_be_written_is_retried(pool: PgPool) {
        let listener = Listener::start().await;
        let device = on_loopback(&pool, DOCKER_HOST).await;
        let late = add(&pool, device, listener.silent_tls(), 2).await;
        let now = database_now(&pool).await;
        let mut scheduler = started_before(&pool, now).await;
        due_at(&pool, late, now - TimeDelta::seconds(25)).await;
        fail_event_writes(&pool).await;

        scheduler.tick(now, &mut rng()).await;
        listener.wait_for(1).await;
        assert_eq!(errors(&pool, device).await.len(), 0);
        allow_event_writes(&pool).await;
        scheduler.tick(now + TimeDelta::seconds(1), &mut rng()).await;
        scheduler.finish_running(now + TimeDelta::seconds(1)).await;

        assert_eq!(errors(&pool, device).await, [LATE]);
    }
}
