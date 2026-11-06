//! Repairs driven by real scheduler ticks: the ICMP check goes Down, then the SSH port
//! answers or does not (CHK-013, R-064, LOG-005).

use chrono::TimeDelta;
use serde_json::json;

use super::support::{
    Audited, DOCKER_HOST, HOME_ASSISTANT, JELLYFIN, Listener, add_ping, audits_of, beats,
    closed_port, database_now, on_loopback, repair_events, scheduler, set_ssh_port, settle_briefly,
    silent_ping, stored, tick_at, tried_at,
};
use crate::{checks::config::CheckConfig, events::Bus, status::Status};

fn event(severity: &str, message: &str) -> (String, String, String) {
    (severity.to_owned(), "check".to_owned(), message.to_owned())
}

const SILENT: &str = "Ping never answered on docker-host, and no connection port answers either.";

repair_test! {
    async fn chk_013_a_ping_that_never_answered_moves_to_the_ssh_port_that_does(pool: PgPool) {
        let listener = Listener::start().await;
        let device = on_loopback(&pool, DOCKER_HOST, listener.port).await;
        let now = database_now(&pool).await;
        let id = add_ping(&pool, device, 0, now).await;
        let before = stored(&pool, id).await;
        let mut scheduler = scheduler(&pool, &Bus::new(64));

        tick_at(&mut scheduler, now).await;

        let check = stored(&pool, id).await;
        let port = listener.port;
        let tcp = CheckConfig::Tcp { port, timeout_ms: 5000 };
        assert_eq!(check.config, tcp);
        assert_eq!(check.name, format!("SSH port {port}"));
        let state = check.state;
        assert_eq!((state.status, state.failures, state.next_due_at), (Status::Unknown, 0, Some(now)));
        assert_eq!(check.settings, before.settings);
        assert_eq!(tried_at(&pool, id).await, Some(now));
        let text = format!(
            "Ping never answered on docker-host; SSH port {port} does, so its check now uses TCP {port}."
        );
        assert_eq!(repair_events(&pool, device).await, [event("info", &text)]);
        let audited = Audited {
            operator_id: uuid::Uuid::nil(),
            operator_name: "Anima".to_owned(),
            action: "update".to_owned(),
            target_type: "check".to_owned(),
            client: "127.0.0.1".to_owned(),
            change: json!({
                "repairedBy": "hub",
                "before": {
                    "type": "icmp",
                    "config": {"type": "icmp", "timeoutMs": 1000, "ipVersion": "V6"},
                    "name": "Ping",
                },
                "after": {
                    "type": "tcp",
                    "config": {"type": "tcp", "port": port, "timeoutMs": 5000},
                    "name": format!("SSH port {port}"),
                },
            }),
        };
        assert_eq!(audits_of(&pool, id).await, [audited]);

        tick_at(&mut scheduler, now + TimeDelta::seconds(1)).await;

        assert_eq!(beats(&pool, id).await, ["down", "up"]);
        assert_eq!(stored(&pool, id).await.state.status, Status::Up);
        listener.wait_for(2).await;
    }

    async fn chk_013_the_repair_waits_until_the_retries_are_used(pool: PgPool) {
        let listener = Listener::start().await;
        let device = on_loopback(&pool, DOCKER_HOST, listener.port).await;
        let now = database_now(&pool).await;
        let id = add_ping(&pool, device, 1, now).await;
        let mut scheduler = scheduler(&pool, &Bus::new(64));

        tick_at(&mut scheduler, now).await;
        settle_briefly().await;
        assert_eq!(beats(&pool, id).await, ["unknown"]);
        assert_eq!((listener.accepted(), tried_at(&pool, id).await), (0, None));
        tick_at(&mut scheduler, now + TimeDelta::seconds(5)).await;

        assert_eq!(beats(&pool, id).await, ["unknown", "down"]);
        assert_eq!(stored(&pool, id).await.name, format!("SSH port {}", listener.port));
    }

    async fn chk_013_no_answering_port_changes_nothing_and_warns_once(pool: PgPool) {
        let device = on_loopback(&pool, DOCKER_HOST, closed_port()).await;
        let now = database_now(&pool).await;
        let id = add_ping(&pool, device, 0, now).await;
        let mut scheduler = scheduler(&pool, &Bus::new(64));

        tick_at(&mut scheduler, now).await;

        let check = stored(&pool, id).await;
        assert_eq!((check.config, check.name.as_str()), (silent_ping(), "Ping"));
        assert_eq!(check.state.status, Status::Down);
        assert_eq!(tried_at(&pool, id).await, Some(now));
        assert_eq!(repair_events(&pool, device).await, [event("warning", SILENT)]);
        assert_eq!(audits_of(&pool, id).await, []);

        // The SSH port answers now, but the check was tried: the next Down beat leaves it.
        let listener = Listener::start().await;
        set_ssh_port(&pool, device, listener.port).await;
        tick_at(&mut scheduler, now + TimeDelta::seconds(10)).await;
        settle_briefly().await;

        assert_eq!(beats(&pool, id).await, ["down", "down"]);
        assert_eq!(listener.accepted(), 0);
        assert_eq!(stored(&pool, id).await.config, silent_ping());
        assert_eq!(repair_events(&pool, device).await, [event("warning", SILENT)]);
    }

    async fn chk_013_a_device_with_ssh_off_warns_once(pool: PgPool) {
        let device = on_loopback(&pool, HOME_ASSISTANT, 22).await;
        let now = database_now(&pool).await;
        let id = add_ping(&pool, device, 0, now).await;
        let mut scheduler = scheduler(&pool, &Bus::new(64));

        tick_at(&mut scheduler, now).await;
        tick_at(&mut scheduler, now + TimeDelta::seconds(10)).await;

        assert_eq!(stored(&pool, id).await.config, silent_ping());
        assert_eq!(tried_at(&pool, id).await, Some(now));
        let text = "Ping never answered on home-assistant, and no connection port answers either.";
        assert_eq!(repair_events(&pool, device).await, [event("warning", text)]);
    }

    async fn chk_013_a_device_in_maintenance_is_not_repaired(pool: PgPool) {
        let listener = Listener::start().await;
        let device = on_loopback(&pool, JELLYFIN, listener.port).await;
        let now = database_now(&pool).await;
        let id = add_ping(&pool, device, 0, now).await;
        let mut scheduler = scheduler(&pool, &Bus::new(64));

        tick_at(&mut scheduler, now).await;
        settle_briefly().await;

        assert_eq!(beats(&pool, id).await, ["maintenance"]);
        assert_eq!(stored(&pool, id).await.config, silent_ping());
        assert_eq!((listener.accepted(), tried_at(&pool, id).await), (0, None));
        assert_eq!(repair_events(&pool, device).await, []);
    }
}
