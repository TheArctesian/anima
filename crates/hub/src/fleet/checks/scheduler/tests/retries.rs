//! Failed attempts over several ticks: retries, missed pushes, unreadable secrets,
//! maintenance and how soon a stopped device shows Down (CHK-002, CHK-006, SEC-003).

use chrono::{DateTime, TimeDelta, Utc};
use sqlx::PgPool;
use tokio::{sync::broadcast::Receiver, task::JoinHandle};

use super::support::{
    DOCKER_HOST, JELLYFIN, add, add_check, beats, closed_port, database_now, due_at, events_of,
    new_check, on_loopback, rng, scheduler, sealer, stored, tcp, vault_with,
};
use crate::{
    fleet::{
        checks::{
            self, CheckSettings, Definition, NewCheck, PushToken,
            config::{CheckConfig, HttpCredentials, HttpInput, HttpMethod, StatusRange},
            scheduler::tick::Scheduler,
        },
        credentials::Secret,
        events::{Bus, Signal},
    },
    ids::{CheckId, DeviceId},
    status::Status,
};

async fn tick_at(scheduler: &mut Scheduler, at: DateTime<Utc>) {
    scheduler.tick(at, &mut rng()).await;
    scheduler.finish_running(at).await;
}

/// An active manual window on one target.
async fn window_on(pool: &PgPool, target_type: &str, target: String) {
    let window = uuid::Uuid::now_v7();
    sqlx::query(
        "INSERT INTO maintenance.windows (id, name, kind, timezone, rule, active_manual) \
         VALUES ($1, 'Rack work', 'manual', 'UTC', '{\"kind\": \"manual\"}', true)",
    )
    .bind(window)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO maintenance.window_targets (window_id, target_type, target) \
         VALUES ($1, $2, $3)",
    )
    .bind(window)
    .bind(target_type)
    .bind(target)
    .execute(pool)
    .await
    .unwrap();
}

async fn last_message(pool: &PgPool, id: CheckId) -> Option<String> {
    sqlx::query_scalar(
        "SELECT message FROM checks.heartbeats WHERE check_id = $1 ORDER BY at DESC LIMIT 1",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .unwrap()
}

/// An HTTP check to a closed local port, its password sealed with another key.
fn http_with_password() -> Definition {
    Definition::Http(HttpInput {
        url: format!("http://127.0.0.1:{}/", closed_port()),
        method: HttpMethod::Get,
        headers: Vec::new(),
        body: None,
        max_redirects: 0,
        accepted_status: vec![StatusRange { from: 200, to: 299 }],
        keyword: None,
        json_query: None,
        json_expected: None,
        credentials: HttpCredentials::Basic {
            username: "operator".to_owned(),
            password: Secret::from("pw-SECRET-4711".to_owned()),
        },
        verify_tls: true,
        timeout_ms: 1000,
    })
}

/// A closed-port check on the device in the group, with no retries.
async fn child_of(pool: &PgPool, text: &str) -> (CheckId, CheckId) {
    let device = on_loopback(pool, text).await;
    let group = Definition::Config(CheckConfig::Group);
    let group = add_check(pool, new_check(device, "Group", group, 0)).await;
    let mut child = new_check(device, "Child", tcp(closed_port()), 0);
    child.parent = Some(group);
    (group, add_check(pool, child).await)
}

/// A service port on 127.0.0.1 that accepts until the device stops.
struct Host {
    port: u16,
    accepting: JoinHandle<()>,
}

impl Host {
    async fn start() -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let accepting = tokio::spawn(async move {
            while let Ok((tcp, _)) = listener.accept().await {
                drop(tcp);
            }
        });
        Self { port, accepting }
    }

    /// Stops the device: the listener closes, so every connect is refused.
    async fn stop(self) {
        self.accepting.abort();
        assert!(self.accepting.await.unwrap_err().is_cancelled());
    }
}

/// What one tick showed: seconds after the last Up beat, the beat recorded, the check's
/// status, the device's status (the card, CHK-004) and the device statuses published.
type Step = (i64, String, Status, Status, Vec<Status>);

fn published(receiver: &mut Receiver<Signal>) -> Vec<Status> {
    std::iter::from_fn(|| receiver.try_recv().ok())
        .filter_map(|signal| match signal {
            Signal::DeviceStatus { status, .. } => Some(status),
            _ => None,
        })
        .collect()
}

/// Ticks the scheduler at each due time it stored, never earlier, until the check is Down.
/// A tick one second before each due time must do nothing.
async fn follow_until_down(
    pool: &PgPool,
    scheduler: &mut Scheduler,
    receiver: &mut Receiver<Signal>,
    (id, device, last_up): (CheckId, DeviceId, DateTime<Utc>),
) -> Vec<Step> {
    let mut steps = Vec::new();
    while steps.len() < 8 {
        let due = stored(pool, id)
            .await
            .state
            .next_due_at
            .expect("a due time");
        let recorded = beats(pool, id).await;
        tick_at(scheduler, due - TimeDelta::seconds(1)).await;
        assert_eq!(
            beats(pool, id).await,
            recorded,
            "probed before its due time"
        );
        assert_eq!(published(receiver), [], "published before its due time");

        tick_at(scheduler, due).await;
        let beat = beats(pool, id).await.pop().unwrap();
        let check = stored(pool, id).await.state.status;
        let card = checks::device_status(pool, device, due).await.unwrap();
        steps.push((
            (due - last_up).num_seconds(),
            beat,
            check,
            card,
            published(receiver),
        ));
        if check == Status::Down {
            break;
        }
    }
    steps
}

tick_test! {
    // CHK-002 with the default settings (spec 6.4): the card stays
    // Up through the retries and changes to Down on the tick 2 intervals after the last Up
    // beat, through the signal the `deviceStatusChanged` subscription reads (CHK-001,
    // CHK-002, CHK-004, LOG-004, LOG-010, DEV-002).
    async fn chk_002_a_stopped_device_shows_down_within_2_intervals_with_the_defaults(pool: PgPool) {
        let host = Host::start().await;
        let device = on_loopback(&pool, DOCKER_HOST).await;
        let config = serde_json::json!({ "type": "tcp", "port": host.port });
        let service_port = Definition::Config(serde_json::from_value(config).unwrap());
        let new = NewCheck {
            settings: CheckSettings::default(),
            ..new_check(device, "Service port", service_port, 0)
        };
        let id = add_check(&pool, new).await;
        let last_up = database_now(&pool).await;
        due_at(&pool, id, last_up).await;
        let bus = Bus::new(64);
        let mut receiver = bus.subscribe();
        let mut scheduler = scheduler(&pool, &bus);

        tick_at(&mut scheduler, last_up).await;
        assert_eq!(beats(&pool, id).await, ["up"]);
        assert_eq!(published(&mut receiver), [Status::Up]);
        host.stop().await;
        let steps =
            follow_until_down(&pool, &mut scheduler, &mut receiver, (id, device, last_up)).await;

        let (up, down) = (Status::Up, Status::Down);
        let pending = |at: i64| (at, "unknown".to_owned(), up, up, Vec::new());
        let shown_down = (120, "down".to_owned(), down, down, vec![down]);
        assert_eq!(steps, [pending(60), pending(80), pending(100), shown_down]);
        let interval = i64::from(CheckSettings::default().interval_seconds);
        assert!(steps.last().unwrap().0 <= 2 * interval);
        assert_eq!(events_of(&pool, device).await.last().unwrap().2, "docker-host is Down");
    }

    async fn chk_002_a_closed_port_is_down_only_after_the_retries(pool: PgPool) {
        let device = on_loopback(&pool, DOCKER_HOST).await;
        let id = add(&pool, device, tcp(closed_port()), 2).await;
        let now = database_now(&pool).await;
        due_at(&pool, id, now).await;
        let mut scheduler = scheduler(&pool, &Bus::new(16));

        tick_at(&mut scheduler, now).await;
        let first = stored(&pool, id).await.state;
        assert_eq!((first.status, first.failures), (Status::Unknown, 1));
        assert_eq!(first.next_due_at, Some(now + TimeDelta::seconds(5)));
        tick_at(&mut scheduler, now + TimeDelta::seconds(4)).await;
        assert_eq!(beats(&pool, id).await, ["unknown"]);
        tick_at(&mut scheduler, now + TimeDelta::seconds(5)).await;
        assert_eq!(stored(&pool, id).await.state.status, Status::Unknown);
        tick_at(&mut scheduler, now + TimeDelta::seconds(10)).await;

        assert_eq!(beats(&pool, id).await, ["unknown", "unknown", "down"]);
        assert_eq!(stored(&pool, id).await.state.status, Status::Down);
    }

    async fn chk_002_a_push_check_with_no_push_is_down_after_its_retries(pool: PgPool) {
        let device = on_loopback(&pool, DOCKER_HOST).await;
        let id = add(&pool, device, Definition::Push(PushToken::from_random(&[3; 32])), 1).await;
        let now = database_now(&pool).await;
        due_at(&pool, id, now).await;
        let mut scheduler = scheduler(&pool, &Bus::new(16));

        tick_at(&mut scheduler, now).await;
        assert_eq!(stored(&pool, id).await.state.status, Status::Unknown);
        tick_at(&mut scheduler, now + TimeDelta::seconds(5)).await;

        assert_eq!(beats(&pool, id).await, ["unknown", "down"]);
        assert_eq!(last_message(&pool, id).await.as_deref(), Some("no push received"));
    }

    async fn sec_003_secrets_that_do_not_open_give_a_failed_beat(pool: PgPool) {
        let device = on_loopback(&pool, DOCKER_HOST).await;
        let new = new_check(device, "Web", http_with_password(), 0);
        let mut conn = pool.acquire().await.unwrap();
        let id = checks::create_check(&mut conn, sealer(&vault_with(8)), new).await.unwrap().id;
        let now = database_now(&pool).await;
        due_at(&pool, id, now).await;
        let mut scheduler = scheduler(&pool, &Bus::new(16));

        tick_at(&mut scheduler, now).await;

        assert_eq!(beats(&pool, id).await, ["down"]);
        let message = last_message(&pool, id).await;
        assert_eq!(message.as_deref(), Some("the check secrets cannot be opened"));
    }

    async fn chk_006_a_device_with_the_manual_flag_records_maintenance_not_down(pool: PgPool) {
        let device = on_loopback(&pool, JELLYFIN).await;
        let id = add(&pool, device, tcp(closed_port()), 0).await;
        let now = database_now(&pool).await;
        due_at(&pool, id, now).await;
        let mut scheduler = scheduler(&pool, &Bus::new(16));

        tick_at(&mut scheduler, now).await;

        assert_eq!(beats(&pool, id).await, ["maintenance"]);
        assert_eq!(stored(&pool, id).await.state.status, Status::Down);
    }

    async fn chk_006_a_window_on_the_device_records_maintenance_not_down(pool: PgPool) {
        let device = on_loopback(&pool, DOCKER_HOST).await;
        let id = add(&pool, device, tcp(closed_port()), 0).await;
        window_on(&pool, "device", device.to_string()).await;
        let now = database_now(&pool).await;
        due_at(&pool, id, now).await;
        let mut scheduler = scheduler(&pool, &Bus::new(16));

        tick_at(&mut scheduler, now).await;

        assert_eq!(beats(&pool, id).await, ["maintenance"]);
    }

    async fn chk_006_a_window_on_the_check_records_maintenance(pool: PgPool) {
        let (_, child) = child_of(&pool, DOCKER_HOST).await;
        window_on(&pool, "check", child.to_string()).await;
        let now = database_now(&pool).await;
        due_at(&pool, child, now).await;
        let mut scheduler = scheduler(&pool, &Bus::new(16));

        tick_at(&mut scheduler, now).await;

        assert_eq!(beats(&pool, child).await, ["maintenance"]);
    }

    async fn chk_006_a_window_on_the_group_records_maintenance_for_its_child(pool: PgPool) {
        let (group, child) = child_of(&pool, DOCKER_HOST).await;
        window_on(&pool, "check", group.to_string()).await;
        let now = database_now(&pool).await;
        due_at(&pool, child, now).await;
        let mut scheduler = scheduler(&pool, &Bus::new(16));

        tick_at(&mut scheduler, now).await;

        assert_eq!(beats(&pool, child).await, ["maintenance"]);
    }
}
