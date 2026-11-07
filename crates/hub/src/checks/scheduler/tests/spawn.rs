//! The spawned scheduler: it ticks at the clock's time until its handle is aborted
//! (HUB-003). Real time, because paused tokio time would also jump the database waits.

use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use chrono::{DateTime, TimeDelta, Utc};

use super::support::{
    DOCKER_HOST, Listener, add, beats, database_now, due_at, on_loopback, tcp, vault,
};
use crate::{
    checks::{Probing, spawn_scheduler},
    clock::Clock,
    events::Bus,
    targets::AddressPolicy,
};

/// A clock the test moves forward.
struct SteppedClock(Mutex<DateTime<Utc>>);

impl Clock for SteppedClock {
    fn now(&self) -> DateTime<Utc> {
        *self.0.lock().unwrap()
    }
}

impl SteppedClock {
    fn advance(&self, by: TimeDelta) {
        *self.0.lock().unwrap() += by;
    }
}

tick_test! {
    async fn hub_003_the_spawned_scheduler_runs_checks_until_aborted(pool: PgPool) {
        let listener = Listener::start().await;
        let device = on_loopback(&pool, DOCKER_HOST).await;
        let id = add(&pool, device, tcp(listener.port), 2).await;
        let now = database_now(&pool).await;
        due_at(&pool, id, now).await;
        let clock = Arc::new(SteppedClock(Mutex::new(now)));
        let probing = Probing {
            vault: vault(),
            policy: AddressPolicy::default().allowing_loopback(),
            localapi: None,
        };

        let handle = spawn_scheduler(pool.clone(), Bus::new(16), clock.clone(), probing);
        let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
        while beats(&pool, id).await.is_empty() {
            assert!(tokio::time::Instant::now() < deadline, "no beat from the scheduler");
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        handle.abort();
        assert!(handle.await.unwrap_err().is_cancelled());
        clock.advance(TimeDelta::seconds(10));
        tokio::time::sleep(Duration::from_millis(2500)).await;

        assert_eq!(beats(&pool, id).await, ["up"]);
    }
}
