//! One tick against the database: beats, first runs, pause, the in-flight guard and the
//! limits (CHK-001, CHK-003, CHK-005, HUB-003).

use chrono::TimeDelta;

use super::support::{
    DOCKER_HOST, Listener, add, add_check, beats, database_now, device, due_at, limited, new_check,
    on_loopback, rng, scheduler, settle_briefly, stored, tcp,
};
use crate::{
    checks::{self, scheduler::tick::Limits, store},
    events::Bus,
    status::Status,
};

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

    async fn chk_001_checks_due_before_the_start_are_spread_not_run_at_once(pool: PgPool) {
        let listener = Listener::start().await;
        let device = on_loopback(&pool, DOCKER_HOST).await;
        let id = add(&pool, device, tcp(listener.port), 2).await;
        let now = database_now(&pool).await;
        due_at(&pool, id, now - TimeDelta::hours(1)).await;
        let mut scheduler = scheduler(&pool, &Bus::new(16));

        scheduler.tick(now, &mut rng()).await;
        scheduler.finish_running(now).await;

        assert_eq!(listener.accepted(), 0);
        let next = stored(&pool, id).await.state.next_due_at.unwrap();
        assert!(next >= now && next < now + TimeDelta::seconds(10), "{next}");
    }

    async fn chk_001_a_new_due_time_never_replaces_one_set_meanwhile(pool: PgPool) {
        let id = add(&pool, device(DOCKER_HOST), tcp(22), 2).await;
        let now = database_now(&pool).await;
        due_at(&pool, id, now).await;

        let stale = store::set_next_due(&pool, id, None, now + TimeDelta::hours(1)).await;
        assert!(!stale.unwrap());
        assert_eq!(stored(&pool, id).await.state.next_due_at, Some(now));
        let moved = store::set_next_due(&pool, id, Some(now), now + TimeDelta::hours(1)).await;
        assert!(moved.unwrap());
        assert_eq!(stored(&pool, id).await.state.next_due_at, Some(now + TimeDelta::hours(1)));
    }

    async fn chk_001_a_check_in_flight_is_not_started_again(pool: PgPool) {
        let listener = Listener::start().await;
        let device = on_loopback(&pool, DOCKER_HOST).await;
        let id = add(&pool, device, listener.silent_tls(), 2).await;
        let now = database_now(&pool).await;
        due_at(&pool, id, now).await;
        let mut scheduler = scheduler(&pool, &Bus::new(16));

        scheduler.tick(now, &mut rng()).await;
        listener.wait_for(1).await;
        scheduler.tick(now + TimeDelta::seconds(1), &mut rng()).await;
        settle_briefly().await;

        assert_eq!(listener.accepted(), 1);
        scheduler.finish_running(now).await;
        assert_eq!(beats(&pool, id).await.len(), 1);
    }

    async fn chk_001_a_check_whose_write_keeps_failing_waits_its_retry_interval(pool: PgPool) {
        let listener = Listener::start().await;
        let device = on_loopback(&pool, DOCKER_HOST).await;
        let id = add(&pool, device, tcp(listener.port), 2).await;
        let now = database_now(&pool).await;
        due_at(&pool, id, now).await;
        for sql in [
            "CREATE FUNCTION checks.refuse_beat() RETURNS trigger LANGUAGE plpgsql AS \
             $$ BEGIN RAISE EXCEPTION 'refused'; END $$",
            "CREATE TRIGGER refuse_beat BEFORE INSERT ON checks.uptime_minute \
             FOR EACH ROW EXECUTE FUNCTION checks.refuse_beat()",
        ] {
            sqlx::query(sql).execute(&pool).await.unwrap();
        }
        let mut scheduler = scheduler(&pool, &Bus::new(16));

        for second in 0..5 {
            let at = now + TimeDelta::seconds(second);
            scheduler.tick(at, &mut rng()).await;
            scheduler.finish_running(at).await;
        }
        assert_eq!(listener.accepted(), 1);
        let retry = now + TimeDelta::seconds(5);
        assert_eq!(stored(&pool, id).await.state.next_due_at, Some(retry));
        scheduler.tick(retry, &mut rng()).await;
        scheduler.finish_running(retry).await;

        listener.wait_for(2).await;
        assert_eq!(beats(&pool, id).await.len(), 0);
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

    async fn d_7_no_more_probes_run_at_once_than_the_limit(pool: PgPool) {
        let listener = Listener::start().await;
        let device = on_loopback(&pool, DOCKER_HOST).await;
        let now = database_now(&pool).await;
        let mut ids = Vec::new();
        for name in ["A", "B"] {
            let id = add_check(&pool, new_check(device, name, listener.silent_tls(), 2)).await;
            due_at(&pool, id, now).await;
            ids.push(id);
        }
        let mut scheduler = limited(&pool, Limits { probes: 1, due_per_tick: 256 });

        scheduler.tick(now, &mut rng()).await;
        listener.wait_for(1).await;
        settle_briefly().await;
        assert_eq!(listener.accepted(), 1);
        scheduler.finish_running(now).await;
        scheduler.tick(now + TimeDelta::seconds(1), &mut rng()).await;
        listener.wait_for(2).await;

        scheduler.finish_running(now).await;
        for id in ids {
            assert_eq!(beats(&pool, id).await.len(), 1);
        }
    }

    async fn d_7_a_held_permit_blocks_a_later_tick(pool: PgPool) {
        let listener = Listener::start().await;
        let device = on_loopback(&pool, DOCKER_HOST).await;
        let now = database_now(&pool).await;
        for name in ["A", "B"] {
            let id = add_check(&pool, new_check(device, name, listener.silent_tls(), 2)).await;
            due_at(&pool, id, now).await;
        }
        let mut scheduler = limited(&pool, Limits { probes: 1, due_per_tick: 256 });

        scheduler.tick(now, &mut rng()).await;
        listener.wait_for(1).await;
        scheduler.tick(now + TimeDelta::seconds(1), &mut rng()).await;
        settle_briefly().await;

        assert_eq!(listener.accepted(), 1);
        scheduler.finish_running(now).await;
    }

    async fn d_7_a_tick_reads_no_more_due_checks_than_the_limit(pool: PgPool) {
        let listener = Listener::start().await;
        let device = on_loopback(&pool, DOCKER_HOST).await;
        let now = database_now(&pool).await;
        for name in ["A", "B", "C"] {
            let id = add_check(&pool, new_check(device, name, listener.silent_tls(), 2)).await;
            due_at(&pool, id, now).await;
        }
        let mut scheduler = limited(&pool, Limits { probes: 32, due_per_tick: 2 });

        scheduler.tick(now, &mut rng()).await;
        listener.wait_for(2).await;
        settle_briefly().await;

        assert_eq!(listener.accepted(), 2);
        scheduler.finish_running(now).await;
    }
}
