//! The probe policy the scheduler is given always refuses loopback, link-local and
//! multicast addresses, whatever the deny list holds (R-030, D-8).

use std::net::IpAddr;

use ipnet::IpNet;

use super::support::{
    DOCKER_HOST, Listener, add, database_now, due_at, on_loopback, rng, scheduler_with, tcp,
};
use crate::{
    checks::{probe::AddressPolicy, scheduler::Probing},
    events::Bus,
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
