//! One tick against the database: beats, first runs, pause and the in-flight guard
//! (CHK-001, CHK-003, CHK-005, HUB-003).

use chrono::TimeDelta;

use super::support::{
    DOCKER_HOST, Listener, add, beats, database_now, due_at, on_loopback, rng, scheduler,
    settle_briefly, stored, tcp,
};
use crate::{checks, events::Bus, status::Status};

tick_test! {
    async fn hub_003_a_tick_probes_a_due_check_and_records_its_beat(pool: PgPool) {
        let listener = Listener::start().await;
        let device = on_loopback(&pool, DOCKER_HOST).await;
        let id = add(&pool, device, tcp(listener.port), 2).await;
        let now = database_now(&pool).await;
        due_at(&pool, id, now).await;
        let mut scheduler = scheduler(&pool, &Bus::new(16));

        scheduler.tick(now, &mut rng()).await;
        scheduler.finish_running(now).await;

        assert_eq!(beats(&pool, id).await, ["up"]);
        let check = stored(&pool, id).await;
        assert_eq!((check.state.status, check.state.failures), (Status::Up, 0));
        assert_eq!(check.state.next_due_at, Some(now + TimeDelta::seconds(10)));
        assert_eq!(check.last_beat_at, Some(now));
        assert_eq!(check.resolved_ip, Some("127.0.0.1".parse().unwrap()));
    }

    async fn chk_001_a_first_run_is_scheduled_within_the_interval_and_not_probed(pool: PgPool) {
        let listener = Listener::start().await;
        let device = on_loopback(&pool, DOCKER_HOST).await;
        let id = add(&pool, device, tcp(listener.port), 2).await;
        let now = database_now(&pool).await;
        let mut scheduler = scheduler(&pool, &Bus::new(16));

        scheduler.tick(now, &mut rng()).await;
        scheduler.finish_running(now).await;

        let next = stored(&pool, id).await.state.next_due_at.unwrap();
        assert!(next >= now && next < now + TimeDelta::seconds(10), "{next}");
        assert_eq!(beats(&pool, id).await.len(), 0);
        scheduler.tick(now + TimeDelta::seconds(10), &mut rng()).await;
        scheduler.finish_running(now).await;
        assert_eq!(beats(&pool, id).await, ["up"]);
    }

    async fn chk_001_a_check_in_flight_is_not_started_again(pool: PgPool) {
        let listener = Listener::start().await;
        let device = on_loopback(&pool, DOCKER_HOST).await;
        let id = add(&pool, device, listener.silent_tls(), 2).await;
        let now = database_now(&pool).await;
        due_at(&pool, id, now).await;
        let mut scheduler = scheduler(&pool, &Bus::new(16));

        scheduler.tick(now, &mut rng()).await;
        settle_briefly().await;
        scheduler.tick(now + TimeDelta::seconds(1), &mut rng()).await;
        settle_briefly().await;

        assert_eq!(listener.accepted(), 1);
        scheduler.finish_running(now).await;
        assert_eq!(beats(&pool, id).await.len(), 1);
    }

    async fn chk_005_a_paused_check_is_not_probed(pool: PgPool) {
        let listener = Listener::start().await;
        let device = on_loopback(&pool, DOCKER_HOST).await;
        let id = add(&pool, device, tcp(listener.port), 2).await;
        let now = database_now(&pool).await;
        let mut conn = pool.acquire().await.unwrap();
        checks::set_paused(&mut conn, id, true).await.unwrap();
        due_at(&pool, id, now).await;
        let mut scheduler = scheduler(&pool, &Bus::new(16));

        scheduler.tick(now, &mut rng()).await;
        scheduler.finish_running(now).await;

        assert_eq!(listener.accepted(), 0);
        assert_eq!(beats(&pool, id).await.len(), 0);
    }
}
