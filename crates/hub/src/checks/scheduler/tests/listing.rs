//! A Docker or systemd check whose SSH link cannot open records a Down beat with a fixed
//! message that holds no host (D-3, CHK-012, LOG-007).

use sqlx::PgPool;

use super::support::{
    DOCKER_HOST, add_check, database_now, device, due_at, new_check, rng, scheduler,
};
use crate::{
    checks::{Definition, config::CheckConfig},
    events::Bus,
    ids::{CheckId, DeviceId},
};

/// `desktop-pc` in the K6 fixtures: SSH is turned off.
const SSH_OFF: &str = "0190a000-0000-7000-8002-000000000004";

fn docker() -> Definition {
    Definition::Config(CheckConfig::Docker {
        container: "web".to_owned(),
        timeout_ms: 5000,
    })
}

fn systemd() -> Definition {
    Definition::Config(CheckConfig::Systemd {
        unit: "nginx".to_owned(),
        timeout_ms: 5000,
    })
}

async fn attempt(pool: &PgPool, device: DeviceId, definition: Definition) -> (String, String) {
    let name = definition.kind().as_sql().to_owned();
    let id: CheckId = add_check(pool, new_check(device, &name, definition, 0)).await;
    let now = database_now(pool).await;
    due_at(pool, id, now).await;
    let mut scheduler = scheduler(pool, &Bus::new(16));
    scheduler.tick(now, &mut rng()).await;
    scheduler.finish_running(now).await;
    sqlx::query_as::<_, (String, String)>(
        "SELECT status, message FROM checks.heartbeats WHERE check_id = $1",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .unwrap()
}

tick_test! {
    async fn d_3_a_device_with_ssh_off_gives_a_down_beat_with_a_fixed_message(pool: PgPool) {
        for definition in [docker(), systemd()] {
            let (status, message) = attempt(&pool, device(SSH_OFF), definition).await;
            assert_eq!(status, "down");
            assert_eq!(message, "the SSH link failed: SSH is turned off for the device");
        }
    }

    async fn d_3_a_device_without_an_accepted_host_key_gives_a_down_beat_without_its_host(pool: PgPool) {
        for definition in [docker(), systemd()] {
            let (status, message) = attempt(&pool, device(DOCKER_HOST), definition).await;
            assert_eq!(status, "down");
            assert_eq!(message, "the SSH link failed: no host key is accepted for the device");
            assert!(!message.contains("10.10.0.21") && !message.contains("docker-host"));
        }
    }
}
