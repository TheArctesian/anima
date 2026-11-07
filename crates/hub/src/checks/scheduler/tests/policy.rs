//! The probe policy the scheduler is given always refuses loopback, link-local and
//! multicast addresses, whatever the deny list holds. A Docker or systemd check whose SSH link
//! cannot open records a Down beat with a fixed message that holds no host (D-3, LOG-007). A
//! Tailscale ping check with no `LocalAPI` proxy records an Unknown beat (D-4).

use std::net::IpAddr;

use ipnet::IpNet;
use sqlx::PgPool;

use super::support::{
    DOCKER_HOST, Listener, add, add_check, database_now, device, due_at, new_check, on_loopback,
    rng, scheduler, scheduler_with, tcp,
};
use crate::{
    checks::{Definition, config::CheckConfig, scheduler::Probing},
    events::Bus,
    ids::{CheckId, DeviceId},
    targets::AddressPolicy,
};

fn nets(texts: &[&str]) -> Vec<IpNet> {
    texts.iter().map(|text| text.parse().unwrap()).collect()
}

#[test]
fn r_030_any_deny_list_still_refuses_loopback_link_local_and_multicast() {
    let lists = [
        nets(&[]),
        nets(&["10.0.0.0/8"]),
        nets(&["172.18.0.0/16", "203.0.113.7/32", "fd00::/8"]),
    ];
    let refused: Vec<IpAddr> = [
        "127.0.0.1",
        "169.254.169.254",
        "::1",
        "fe80::1",
        "224.0.0.1",
    ]
    .iter()
    .map(|text| text.parse().unwrap())
    .collect();
    for list in lists {
        let probing = Probing {
            vault: super::support::vault(),
            policy: AddressPolicy::new(list.clone()),
            localapi: None,
        };
        for ip in &refused {
            assert!(!probing.policy.permits(*ip), "{ip} with {list:?}");
        }
    }
}

tick_test! {
    async fn r_030_a_scheduler_with_an_empty_deny_list_never_connects_to_loopback(pool: PgPool) {
        let listener = Listener::start().await;
        let device = on_loopback(&pool, DOCKER_HOST).await;
        let id = add(&pool, device, tcp(listener.port), 0).await;
        let now = database_now(&pool).await;
        due_at(&pool, id, now).await;
        let mut scheduler = scheduler_with(&pool, &Bus::new(16), AddressPolicy::new(Vec::new()));

        scheduler.tick(now, &mut rng()).await;
        scheduler.finish_running(now).await;

        assert_eq!(listener.accepted(), 0);
        let beat: (String, Option<String>) = sqlx::query_as(
            "SELECT status, message FROM checks.heartbeats WHERE check_id = $1",
        )
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(beat, ("down".to_owned(), Some("the address is not permitted".to_owned())));
    }
}

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

fn tailscale_ping() -> Definition {
    Definition::Config(CheckConfig::TailscalePing {
        target: Some("100.64.0.7".to_owned()),
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

    async fn d_4_a_tailscale_ping_without_a_localapi_proxy_gives_an_unknown_beat_with_the_message(pool: PgPool) {
        let (status, message) = attempt(&pool, device(DOCKER_HOST), tailscale_ping()).await;
        assert_eq!(status, "unknown");
        assert_eq!(message, "Tailscale ping needs the LocalAPI proxy (ANIMA_TAILSCALE_LOCALAPI)");
        let state: (String, i32) =
            sqlx::query_as("SELECT status, failures FROM checks.checks WHERE type = 'tailscale_ping'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(state, ("unknown".to_owned(), 0));
    }
}
