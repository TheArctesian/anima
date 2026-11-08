//! Writes that land while a probe runs: the attempt is evaluated on the state they left, not
//! on the state read at tick time (CHK-002, CHK-005), and HTTP secrets go only where the
//! configuration read with them points (SEC-005).

use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use chrono::{DateTime, TimeDelta, Utc};
use sqlx::PgPool;
use tokio::{
    io::{AsyncReadExt as _, AsyncWriteExt as _},
    task::JoinSet,
};

use super::support::{
    DOCKER_HOST, Listener, add, add_check, beats, context, database_now, device, due_at, new_check,
    on_loopback, rng, scheduler, stored, tcp, vault,
};
use crate::{
    checks::{
        self, Attempt, Beat, CheckChange, CheckState, Definition,
        config::{HttpCredentials, HttpInput, HttpMethod, StatusRange},
        probe::{Failure, ProbeResult},
        scheduler::attempt,
    },
    credentials::Secret,
    events::{Bus, Signal},
    ids::CheckId,
    status::Status,
};

const OLD_TOKEN: &str = "tok-OLD-SECRET-1111";
const NEW_TOKEN: &str = "tok-NEW-SECRET-2222";

/// An HTTP origin on 127.0.0.1 that answers 200 and keeps the head of every request.
struct Origin {
    port: u16,
    heads: Arc<Mutex<Vec<String>>>,
}

impl Origin {
    async fn start() -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let heads = Arc::new(Mutex::new(Vec::new()));
        let seen = Arc::clone(&heads);
        tokio::spawn(async move {
            while let Ok((mut tcp, _)) = listener.accept().await {
                let head = read_head(&mut tcp).await;
                seen.lock().unwrap().push(head);
                let reply = "HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
                let _ = tcp.write_all(reply.as_bytes()).await;
            }
        });
        Self { port, heads }
    }

    fn url(&self) -> String {
        format!("http://127.0.0.1:{}/", self.port)
    }

    fn heads(&self) -> Vec<String> {
        self.heads.lock().unwrap().clone()
    }
}

/// The request head, up to the blank line or 16 KiB.
async fn read_head(tcp: &mut tokio::net::TcpStream) -> String {
    let mut head = Vec::new();
    let mut byte = [0u8; 1];
    while !head.ends_with(b"\r\n\r\n") && head.len() < 16 * 1024 {
        if tcp.read_exact(&mut byte).await.is_err() {
            break;
        }
        head.push(byte[0]);
    }
    String::from_utf8_lossy(&head).into_owned()
}

fn bearer_http(url: String, token: &str) -> Definition {
    Definition::Http(HttpInput {
        url,
        method: HttpMethod::Get,
        headers: Vec::new(),
        body: None,
        max_redirects: 0,
        accepted_status: vec![StatusRange { from: 200, to: 299 }],
        keyword: None,
        json_query: None,
        json_expected: None,
        credentials: HttpCredentials::Bearer(Secret::from(token.to_owned())),
        verify_tls: true,
        timeout_ms: 2000,
    })
}

/// The last heartbeat message of the check.
async fn last_message(pool: &PgPool, id: CheckId) -> Option<String> {
    sqlx::query_scalar(
        "SELECT message FROM checks.heartbeats WHERE check_id = $1 ORDER BY at DESC LIMIT 1",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .unwrap()
}

/// Records one failed attempt at `at` in its own transaction; returns whether it was written.
async fn failed_attempt(pool: PgPool, id: CheckId, at: DateTime<Utc>) -> bool {
    let result = ProbeResult::failed(&Failure::NoPush);
    let attempt = Attempt {
        result: &result,
        in_maintenance: false,
        at,
        domain: None,
    };
    let mut tx = pool.begin().await.unwrap();
    let beat = checks::record_attempt(&mut tx, id, &attempt).await.unwrap();
    tx.commit().await.unwrap();
    beat.is_some()
}

tick_test! {
    async fn chk_002_concurrent_failed_attempts_count_every_written_failure(pool: PgPool) {
        for round in 0..30 {
            let new = new_check(device(DOCKER_HOST), &format!("F{round}"), tcp(22), 10);
            let id = add_check(&pool, new).await;
            let base = database_now(&pool).await - TimeDelta::seconds(30);
            let mut attempts = JoinSet::new();
            for i in 0..10 {
                let at = base + TimeDelta::milliseconds(i);
                attempts.spawn(failed_attempt(pool.clone(), id, at));
            }
            let all = tokio::time::timeout(Duration::from_secs(30), attempts.join_all()).await;
            let written = all.expect("the attempts finish").into_iter().filter(|w| *w).count();
            let failures = stored(&pool, id).await.state.failures;
            assert_eq!(usize::try_from(failures).unwrap(), written, "round {round}");
        }
    }

    async fn chk_002_a_settings_change_during_a_probe_applies_to_its_attempt(pool: PgPool) {
        let listener = Listener::start().await;
        let device = on_loopback(&pool, DOCKER_HOST).await;
        let id = add(&pool, device, listener.silent_tls(), 2).await;
        let now = database_now(&pool).await;
        due_at(&pool, id, now).await;
        let mut scheduler = scheduler(&pool, &Bus::new(64));

        scheduler.tick(now, &mut rng()).await;
        listener.wait_for(1).await;
        let change = CheckChange {
            retries: Some(0),
            ..CheckChange::default()
        };
        let mut conn = pool.acquire().await.unwrap();
        checks::update_check(&mut conn, &vault(), id, change).await.unwrap();
        scheduler.finish_running(now).await;

        assert_eq!(beats(&pool, id).await, ["down"]);
    }

    async fn chk_005_a_pause_during_a_probe_wins(pool: PgPool) {
        let listener = Listener::start().await;
        let device = on_loopback(&pool, DOCKER_HOST).await;
        let id = add(&pool, device, listener.silent_tls(), 2).await;
        let now = database_now(&pool).await;
        sqlx::query("UPDATE checks.checks SET next_due_at = $2 WHERE id = $1")
            .bind(id)
            .bind(now)
            .execute(&pool)
            .await
            .unwrap();
        let bus = Bus::new(64);
        let mut receiver = bus.subscribe();
        let mut scheduler = scheduler(&pool, &bus);

        scheduler.tick(now, &mut rng()).await;
        listener.wait_for(1).await;
        let mut conn = pool.acquire().await.unwrap();
        checks::set_paused(&mut conn, id, true).await.unwrap();
        scheduler.finish_running(now).await;

        assert_eq!(beats(&pool, id).await.len(), 0);
        let signals: Vec<Signal> = std::iter::from_fn(|| receiver.try_recv().ok()).collect();
        assert!(!signals.iter().any(|s| matches!(s, Signal::Heartbeat { .. })), "{signals:?}");
        let check = stored(&pool, id).await;
        assert!(check.paused);
        assert_eq!(check.state.failures, 0);
    }

    async fn chk_002_an_attempt_is_evaluated_on_a_push_made_during_its_probe(pool: PgPool) {
        let listener = Listener::start().await;
        let device = on_loopback(&pool, DOCKER_HOST).await;
        let id = add(&pool, device, listener.silent_tls(), 1).await;
        let now = database_now(&pool).await;
        sqlx::query(
            "UPDATE checks.checks SET status = 'up', failures = 1, next_due_at = $2 WHERE id = $1",
        )
        .bind(id)
        .bind(now)
        .execute(&pool)
        .await
        .unwrap();
        let mut scheduler = scheduler(&pool, &Bus::new(64));

        scheduler.tick(now, &mut rng()).await;
        listener.wait_for(1).await;
        let pushed_at = now - TimeDelta::seconds(1);
        let push = Beat {
            at: pushed_at,
            status: Status::Up,
            response_time_ms: None,
            message: None,
            state: CheckState {
                status: Status::Up,
                failures: 0,
                next_due_at: Some(pushed_at + TimeDelta::seconds(10)),
            },
            resolved_ip: None,
            cert_not_after: None,
            domain_not_after: None,
        };
        let mut conn = pool.acquire().await.unwrap();
        let written = checks::record_beat(&mut conn, id, &push).await.unwrap();
        assert_eq!(written, checks::Recorded::Written);
        scheduler.finish_running(now).await;

        assert_eq!(beats(&pool, id).await, ["up", "unknown"]);
        let state = stored(&pool, id).await.state;
        assert_eq!((state.status, state.failures), (Status::Up, 1));
    }

    async fn sec_005_a_new_secret_never_reaches_the_origin_read_at_tick_time(pool: PgPool) {
        let (old, new) = (Origin::start().await, Origin::start().await);
        let device = on_loopback(&pool, DOCKER_HOST).await;
        let id = add(&pool, device, bearer_http(old.url(), OLD_TOKEN), 0).await;
        let now = database_now(&pool).await;
        due_at(&pool, id, now).await;
        let due = checks::due_checks(&pool, now, 10).await.unwrap().remove(0);

        let change = CheckChange {
            definition: Some(bearer_http(new.url(), NEW_TOKEN)),
            ..CheckChange::default()
        };
        let mut conn = pool.acquire().await.unwrap();
        checks::update_check(&mut conn, &vault(), id, change).await.unwrap();
        let finished = attempt::run(&context(&pool), due, now).await;

        // The attempt probes the configuration stored with the secrets, so the old origin
        // gets nothing and the new one gets its own token.
        let leaked = old.heads().iter().any(|head| head.contains(NEW_TOKEN));
        assert!(!leaked, "the new token reached the old origin");
        assert_eq!(old.heads().len(), 0);
        let sent = new.heads();
        assert_eq!(sent.len(), 1);
        assert!(sent[0].contains(&format!("Bearer {NEW_TOKEN}")), "{}", sent[0]);
        assert!(finished.recorded.is_some());
        assert_eq!(beats(&pool, id).await, ["up"]);
    }

    async fn chk_001_secrets_bound_to_another_destination_fail_the_check(pool: PgPool) {
        let origin = Origin::start().await;
        let device = on_loopback(&pool, DOCKER_HOST).await;
        let id = add(&pool, device, bearer_http(origin.url(), OLD_TOKEN), 1).await;
        let elsewhere = format!("\"http://127.0.0.1:{}/\"", origin.port + 1);
        sqlx::query(
            "UPDATE checks.checks SET status = 'up', \
                    config = jsonb_set(config, '{url}', $2::jsonb) WHERE id = $1",
        )
        .bind(id)
        .bind(elsewhere)
        .execute(&pool)
        .await
        .unwrap();
        let mut scheduler = scheduler(&pool, &Bus::new(16));

        let first = database_now(&pool).await - TimeDelta::seconds(20);
        for at in [first, first + TimeDelta::seconds(10)] {
            due_at(&pool, id, at).await;
            scheduler.tick(at, &mut rng()).await;
            scheduler.finish_running(at).await;
        }

        assert_eq!(beats(&pool, id).await, ["unknown", "down"]);
        assert_eq!(stored(&pool, id).await.state.status, Status::Down);
        let message = last_message(&pool, id).await;
        assert_eq!(message.as_deref(), Some("the check secrets do not match its configuration"));
        assert_eq!(origin.heads().len(), 0, "a request went out");
    }

    async fn chk_001_a_configuration_unreadable_after_the_tick_fails_the_check(pool: PgPool) {
        let origin = Origin::start().await;
        let device = on_loopback(&pool, DOCKER_HOST).await;
        let id = add(&pool, device, bearer_http(origin.url(), OLD_TOKEN), 0).await;
        let now = database_now(&pool).await;
        due_at(&pool, id, now).await;
        let due = checks::due_checks(&pool, now, 10).await.unwrap().remove(0);
        sqlx::query("UPDATE checks.checks SET config = '{}' WHERE id = $1")
            .bind(id)
            .execute(&pool)
            .await
            .unwrap();

        attempt::run(&context(&pool), due, now).await;

        assert_eq!(beats(&pool, id).await, ["down"]);
        let message = last_message(&pool, id).await;
        assert_eq!(message.as_deref(), Some("the configuration is not valid"));
        assert_eq!(origin.heads().len(), 0, "a request went out");
    }
}
