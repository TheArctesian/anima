//! `Device.status`, `uptime`, `checks` and `DeviceFilter.status` (CHK-004, CHK-011, API-005).

use std::collections::HashMap;

use anima_hub::{fleet::checks, ids::DeviceId, status::Status};
use chrono::TimeDelta;
use serde_json::{Value, json};
use sqlx::PgPool;

use super::{PVE_01, names, ping_uptime, seed_standing};
use crate::devices::harness::{data, now, schema_for};

macro_rules! fixture_test {
    ($(async fn $name:ident($pool:ident: PgPool) $body:block)+) => {$(
        #[sqlx::test(
            migrations = "./migrations",
            fixtures(path = "../../../fixtures", scripts("devices", "checks"))
        )]
        async fn $name($pool: PgPool) $body
    )+};
}

fn status_name(status: Status) -> &'static str {
    match status {
        Status::Up => "UP",
        Status::Down => "DOWN",
        Status::Degraded => "DEGRADED",
        Status::Maintenance => "MAINTENANCE",
        Status::Unknown => "UNKNOWN",
    }
}

fn by_name(connection: &Value) -> HashMap<&str, &Value> {
    connection["edges"]
        .as_array()
        .unwrap()
        .iter()
        .map(|edge| (edge["node"]["name"].as_str().unwrap(), &edge["node"]))
        .collect()
}

fn all(filter: &str) -> String {
    format!(
        "{{ devices(first: 200, filter: {filter}) {{ totalCount edges {{ node {{ name }} }} }} }}"
    )
}

fixture_test! {
    async fn chk_004_device_status_is_what_the_checks_face_reports_for_every_device(pool: PgPool) {
        seed_standing(&pool).await;
        let result = data(&schema_for(&pool), "{ devices(first: 200) { edges { node { id name status } } } }").await;

        let edges = result["devices"]["edges"].as_array().unwrap();
        let ids: Vec<DeviceId> = edges.iter().map(|e| e["node"]["id"].as_str().unwrap().parse().unwrap()).collect();
        let expected = checks::statuses(&pool, &ids, now()).await.unwrap();
        assert_eq!(edges.len(), 13);
        for edge in edges {
            let id: DeviceId = edge["node"]["id"].as_str().unwrap().parse().unwrap();
            assert_eq!(edge["node"]["status"], status_name(expected[&id]), "{}", edge["node"]["name"]);
        }
        // The seed makes the check mean something: all five statuses show.
        let shown: std::collections::BTreeSet<_> = edges.iter().map(|e| e["node"]["status"].as_str().unwrap()).collect();
        assert_eq!(shown.len(), 5, "{shown:?}");
    }

    async fn chk_004_the_manual_maintenance_flag_turns_a_down_device_into_maintenance(pool: PgPool) {
        seed_standing(&pool).await;
        let result = data(&schema_for(&pool), "{ devices(first: 200) { edges { node { name maintenance status } } } }").await;
        let devices = by_name(&result["devices"]);
        assert_eq!(devices["jellyfin"]["status"], "MAINTENANCE");
        assert_eq!(devices["pve-01"]["status"], "DOWN");
    }

    async fn chk_011_device_uptime_is_what_the_checks_face_reports_per_window(pool: PgPool) {
        seed_standing(&pool).await;
        ping_uptime(&pool, PVE_01, ("uptime_hour", TimeDelta::days(20)), (9, 1)).await;
        ping_uptime(&pool, PVE_01, ("uptime_hour", TimeDelta::days(3)), (4, 1)).await;
        let result = data(
            &schema_for(&pool),
            "{ devices(first: 200) { edges { node { id name day: uptime(window: DAY) week: uptime(window: WEEK) month: uptime(window: MONTH) } } } }",
        )
        .await;

        let edges = result["devices"]["edges"].as_array().unwrap();
        let ids: Vec<DeviceId> = edges.iter().map(|e| e["node"]["id"].as_str().unwrap().parse().unwrap()).collect();
        for (field, window) in [("day", checks::UptimeWindow::Day), ("week", checks::UptimeWindow::Week), ("month", checks::UptimeWindow::Month)] {
            let expected = checks::uptimes(&pool, &ids, window, now()).await.unwrap();
            for edge in edges {
                let id: DeviceId = edge["node"]["id"].as_str().unwrap().parse().unwrap();
                let shown = edge["node"][field].as_f64();
                assert_eq!(shown, expected.get(&id).copied(), "{field} of {}", edge["node"]["name"]);
            }
        }
        let devices = by_name(&result["devices"]);
        assert_eq!(devices["pve-01"]["day"], 0.5);
        assert_eq!(devices["pve-01"]["week"], 0.8);
        assert_eq!(devices["pihole"]["day"], Value::Null);
    }

    async fn api_005_device_checks_are_the_checks_of_the_face_in_name_order(pool: PgPool) {
        let result = data(&schema_for(&pool), "{ devices(first: 200) { edges { node { id checks { name type } } } } }").await;
        let edges = result["devices"]["edges"].as_array().unwrap();
        let mut counted = 0;
        for edge in edges {
            let id: DeviceId = edge["node"]["id"].as_str().unwrap().parse().unwrap();
            let expected: Vec<Value> = checks::for_device(&pool, id)
                .await
                .unwrap()
                .iter()
                .map(|check| json!({ "name": check.name, "type": format!("{:?}", check.kind()).to_uppercase() }))
                .collect();
            assert_eq!(edge["node"]["checks"], json!(expected));
            counted += expected.len();
        }
        assert_eq!(counted, 28);
        let pve = data(&schema_for(&pool), &format!(r#"{{ device(id: "{PVE_01}") {{ checks {{ name }} }} }}"#)).await;
        assert_eq!(pve["device"]["checks"], json!([{ "name": "Ping" }, { "name": "SSH port" }, { "name": "Web UI" }]));
    }

    async fn chk_004_a_device_with_no_checks_is_unknown_with_no_uptime_and_no_checks(pool: PgPool) {
        sqlx::query("DELETE FROM checks.checks").execute(&pool).await.unwrap();
        let result = data(&schema_for(&pool), "{ devices(first: 1) { edges { node { status uptime(window: YEAR) checks { name } } } } }").await;
        assert_eq!(result["devices"]["edges"][0]["node"], json!({ "status": "UNKNOWN", "uptime": null, "checks": [] }));
    }

    async fn chk_004_status_filter_keeps_only_the_listed_statuses_and_total_count_follows(pool: PgPool) {
        seed_standing(&pool).await;
        let schema = schema_for(&pool);

        let down = data(&schema, &all("{ status: [DOWN] }")).await;
        assert_eq!(names(&down["devices"]), ["pve-01", "truenas"]);
        assert_eq!(down["devices"]["totalCount"], 2);

        let two = data(&schema, &all("{ status: [DEGRADED, UP, MAINTENANCE] }")).await;
        assert_eq!(names(&two["devices"]), ["backup-02", "docker-host", "jellyfin", "ollama", "pihole"]);
        assert_eq!(two["devices"]["totalCount"], 5);
    }

    async fn chk_004_status_filter_totals_count_the_filtered_set_not_the_page(pool: PgPool) {
        seed_standing(&pool).await;
        let result = data(
            &schema_for(&pool),
            "{ devices(first: 1, filter: { status: [UNKNOWN] }) { totalCount pageInfo { hasNextPage } edges { node { name } } } }",
        )
        .await;
        assert_eq!(result["devices"]["totalCount"], 6);
        assert_eq!(result["devices"]["pageInfo"]["hasNextPage"], true);
        assert_eq!(names(&result["devices"]), ["home-assistant"]);
    }

    async fn chk_004_status_filter_combines_with_the_name_and_tag_filters(pool: PgPool) {
        seed_standing(&pool).await;
        let schema = schema_for(&pool);
        let named = data(&schema, &all(r#"{ status: [UNKNOWN], name: "MAC" }"#)).await;
        assert_eq!(names(&named["devices"]), ["mac-office-1", "mac-office-2"]);
        assert_eq!(named["devices"]["totalCount"], 2);

        let tagged = data(&schema, &all(r#"{ status: [UP], tags: ["infra"] }"#)).await;
        assert_eq!(names(&tagged["devices"]), ["pihole"]);
    }

    async fn chk_004_an_empty_status_filter_lists_every_device(pool: PgPool) {
        seed_standing(&pool).await;
        let result = data(&schema_for(&pool), &all("{ status: [] }")).await;
        assert_eq!(result["devices"]["totalCount"], 13);
        let none = data(&schema_for(&pool), &all("{ status: null }")).await;
        assert_eq!(none["devices"]["totalCount"], 13);
    }

    async fn chk_004_status_filter_with_name_sort_pages_by_name(pool: PgPool) {
        seed_standing(&pool).await;
        let schema = schema_for(&pool);
        let filter = "{ status: [UNKNOWN, UP] }";
        let first = data(&schema, &format!("{{ devices(first: 3, filter: {filter}) {{ pageInfo {{ endCursor hasNextPage }} edges {{ node {{ name }} }} }} }}")).await;
        assert_eq!(names(&first["devices"]), ["backup-02", "home-assistant", "linux-pc-home"]);
        let cursor = first["devices"]["pageInfo"]["endCursor"].as_str().unwrap();

        let second = data(&schema, &format!("{{ devices(first: 3, filter: {filter}, after: \"{cursor}\") {{ edges {{ node {{ name }} }} }} }}")).await;
        assert_eq!(names(&second["devices"]), ["linux-pc-office-2", "mac-office-1", "mac-office-2"]);
        // A cursor of this path is a plain name cursor: the SQL path continues after it.
        let plain = data(&schema, &format!("{{ devices(first: 2, after: \"{cursor}\") {{ edges {{ node {{ name }} }} }} }}")).await;
        assert_eq!(names(&plain["devices"]), ["linux-pc-office-2", "mac-office-1"]);
    }
}
