//! The probe policy the scheduler is given always refuses loopback, link-local and
//! multicast addresses, whatever the deny list holds. A Docker or systemd check whose SSH link
//! cannot open records a Down beat with a fixed message that holds no host (D-3, LOG-007). A
//! Tailscale ping check with no `LocalAPI` proxy records an Unknown beat (D-4). SNMP secrets go
//! only to the device host they were entered for: after a host change the attempt sends
//! nothing, removes them and writes one event (SEC-010).

use std::{net::IpAddr, sync::atomic::Ordering};

use chrono::{DateTime, TimeDelta, Utc};
use ipnet::IpNet;
use sqlx::PgPool;

use super::support::{
    DOCKER_HOST, Listener, add, add_check, context, database_now, device, due_at, events_of,
    new_check, on_loopback, rng, scheduler, scheduler_with, tcp, vault,
};
use crate::{
    checks::{
        self, CheckChange, Definition, Sealer,
        config::{CheckConfig, SnmpConfig, SnmpCredentials, SnmpInput, SnmpVersion},
        probe::Failure,
        scheduler::{Probing, attempt},
        tests::snmp::agent::{Agent, Setup, spawn},
    },
    credentials::Secret,
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

/// An SNMP v2c check on `device` against the local agent, its community entered for `host`.
async fn snmp_check(pool: &PgPool, (device, host): (DeviceId, &str), agent: &Agent) -> CheckId {
    let mut conn = pool.acquire().await.unwrap();
    let sealer = Sealer {
        vault: &vault(),
        host,
    };
    let new = new_check(device, "SNMP", snmp(agent, Some(Setup::new().community)), 0);
    checks::create_check(&mut conn, sealer, new)
        .await
        .unwrap()
        .id
}

fn snmp(agent: &Agent, community: Option<&str>) -> Definition {
    Definition::Snmp(SnmpInput {
        config: SnmpConfig {
            port: agent.port,
            version: SnmpVersion::V2c,
            oid: "1.3.6.1.2.1.1.5.0".to_owned(),
            expected: None,
            username: None,
            timeout_ms: 1000,
        },
        credentials: SnmpCredentials {
            community: community.map(|text| Secret::from(text.to_owned())),
            ..SnmpCredentials::default()
        },
    })
}

/// The operator's update of the check's definition, with the device at `host`.
async fn enter(pool: &PgPool, (id, host): (CheckId, &str), definition: Definition) {
    let mut conn = pool.acquire().await.unwrap();
    let change = CheckChange {
        definition: Some(definition),
        ..CheckChange::default()
    };
    let sealer = Sealer {
        vault: &vault(),
        host,
    };
    checks::update_check(&mut conn, sealer, id, change)
        .await
        .unwrap();
}

/// Runs one attempt of the check at `at`; returns the beat's status and message (empty for
/// none).
async fn attempt_at(pool: &PgPool, id: CheckId, at: DateTime<Utc>) -> (String, String) {
    due_at(pool, id, at).await;
    let due = checks::due_checks(pool, at, 10).await.unwrap();
    let due = due.into_iter().find(|due| due.id == id).unwrap();
    attempt::run(&context(pool), due, at).await;
    let (status, message): (String, Option<String>) = sqlx::query_as(
        "SELECT status, message FROM checks.heartbeats WHERE check_id = $1 AND at = $2",
    )
    .bind(id)
    .bind(at)
    .fetch_one(pool)
    .await
    .unwrap();
    (status, message.unwrap_or_default())
}

fn requests(agent: &Agent) -> usize {
    agent.requests.load(Ordering::SeqCst)
}

async fn sealed(pool: &PgPool, id: CheckId) -> bool {
    sqlx::query_scalar("SELECT secrets_sealed IS NOT NULL FROM checks.checks WHERE id = $1")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap()
}

/// The device's events about SNMP secrets, as (severity, message).
async fn secret_events(pool: &PgPool, device: DeviceId) -> Vec<(String, String)> {
    let events = events_of(pool, device).await;
    events
        .into_iter()
        .filter(|(_, source, message)| source == "check" && message.contains("SNMP secrets"))
        .map(|(severity, _, message)| (severity, message))
        .collect()
}

tick_test! {
    async fn sec_010_a_host_that_did_not_change_keeps_its_snmp_secrets(pool: PgPool) {
        let agent = spawn(Setup::new()).await;
        let device = on_loopback(&pool, DOCKER_HOST).await;
        let id = snmp_check(&pool, (device, "127.0.0.1"), &agent).await;
        let now = database_now(&pool).await;
        assert_eq!(attempt_at(&pool, id, now).await.0, "up");
        // A definition that leaves the community out, same version and user, same host.
        enter(&pool, (id, "127.0.0.1"), snmp(&agent, None)).await;
        assert_eq!(attempt_at(&pool, id, now + TimeDelta::seconds(10)).await.0, "up");
        assert_eq!(requests(&agent), 2);
        assert_eq!(secret_events(&pool, device).await, Vec::new());
    }

    async fn sec_010_after_a_host_change_no_snmp_secret_goes_to_the_new_host(pool: PgPool) {
        // The agent listens on the new host. The community was entered for the old one.
        let agent = spawn(Setup::new()).await;
        let device = device(DOCKER_HOST);
        let id = snmp_check(&pool, (device, "10.10.0.21"), &agent).await;
        on_loopback(&pool, DOCKER_HOST).await;
        let now = database_now(&pool).await;

        let (status, message) = attempt_at(&pool, id, now).await;

        assert_eq!(requests(&agent), 0, "a request reached the new host");
        assert_eq!(status, "down");
        assert_eq!(message, Failure::SecretsForOtherHost.to_string());
        assert!(!message.contains("127.0.0.1") && !message.contains(Setup::new().community));
        assert!(!sealed(&pool, id).await, "the secrets were kept");
        let events = secret_events(&pool, device).await;
        assert_eq!(events.len(), 1, "{events:?}");
        assert_eq!(events[0].0, "warning");
        assert!(events[0].1.contains("Enter the community"), "{}", events[0].1);
        assert!(!events[0].1.contains("127.0.0.1") && !events[0].1.contains("10.10.0.21"));
        // The hub's own scheduler found the moved host, so the event is Hub (LOG-013).
        let kinds: Vec<Option<String>> = sqlx::query_scalar(
            "SELECT actor_kind FROM events.event_log WHERE device_id = $1 AND source = 'check'",
        )
        .bind(device)
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(kinds, [Some("hub".to_owned())]);

        // The next attempt still sends nothing and writes no second event.
        let (_, message) = attempt_at(&pool, id, now + TimeDelta::seconds(10)).await;
        assert_eq!(message, Failure::SecretMissing.to_string());
        assert_eq!(requests(&agent), 0);
        assert_eq!(secret_events(&pool, device).await.len(), 1);

        // The operator enters the community again for the new host: the check probes it.
        enter(&pool, (id, "127.0.0.1"), snmp(&agent, Some(Setup::new().community))).await;
        let (status, _) = attempt_at(&pool, id, now + TimeDelta::seconds(20)).await;
        assert_eq!(status, "up");
        assert_eq!(requests(&agent), 1);
    }
}
