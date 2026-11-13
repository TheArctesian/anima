//! `Query.sites`, `device`, `devices` and `tags` against the K6 rows (API-006, VPN-002, K7).

use std::sync::Arc;

use anima_hub::clock::{Clock, FixedClock};
use anima_hub::fleet::devices::{
    self,
    graphql::{Mutation, Query},
};
use async_graphql::{EmptySubscription, Schema};
use serde_json::{Value, json};
use sqlx::PgPool;

use super::harness::{data, now, only_error, schema_for, signed_in, validation_fields};

const HOME_SITE: &str = "0190a000-0000-7000-8001-000000000001";
const OFFICE_1_SITE: &str = "0190a000-0000-7000-8001-000000000002";

fn names(connection: &Value) -> Vec<&str> {
    connection["edges"]
        .as_array()
        .unwrap()
        .iter()
        .map(|edge| edge["node"]["name"].as_str().unwrap())
        .collect()
}

/// Adds `sites` empty sites and `devices` devices spread over every site, with SSH settings
/// and two checks each.
pub(crate) async fn add_bulk(pool: &PgPool, sites: i32, devices: i32) {
    sqlx::query(
        "INSERT INTO devices.sites (id, name, subnet) \
         SELECT gen_random_uuid(), 'bulk-site-' || g, ('10.100.' || g || '.0/24')::cidr \
         FROM generate_series(1, $1) g",
    )
    .bind(sites)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO devices.devices (id, site_id, name, host, kind, os, created_at, updated_at) \
         SELECT gen_random_uuid(), \
                (SELECT id FROM devices.sites ORDER BY name \
                 OFFSET g % (SELECT count(*) FROM devices.sites) LIMIT 1), \
                'bulk-' || g, 'bulk-' || g || '.lan', 'server', 'linux', now(), now() \
         FROM generate_series(1, $1) g",
    )
    .bind(devices)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO devices.ssh_settings (device_id, enabled, port, username, auth) \
         SELECT id, false, 22, '', 'private_key' FROM devices.devices WHERE name LIKE 'bulk-%'",
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO checks.checks (id, device_id, type, name, config) \
         SELECT gen_random_uuid(), id, 'icmp', 'Ping', '{\"timeoutMs\":2000,\"ipVersion\":\"V4\"}'::jsonb \
         FROM devices.devices WHERE name LIKE 'bulk-%' \
         UNION ALL \
         SELECT gen_random_uuid(), id, 'tcp', 'SSH port', '{\"port\":22,\"timeoutMs\":5000}'::jsonb \
         FROM devices.devices WHERE name LIKE 'bulk-%'",
    )
    .execute(pool)
    .await
    .unwrap();
}

macro_rules! fixture_test {
    ($(async fn $name:ident($pool:ident: PgPool) $body:block)+) => {$(
        #[sqlx::test(
            migrations = "./migrations",
            fixtures(path = "../../fixtures", scripts("devices"))
        )]
        async fn $name($pool: PgPool) $body
    )+};
}

const ALL_FIELDS: &str = "{ devices { totalCount pageInfo { hasNextPage endCursor }
    edges { cursor node { id name host ip kind os tags maintenance createdAt updatedAt
    ssh { enabled port username auth } site { id name } } } } }";

fixture_test! {
    async fn api_006_devices_lists_all_in_name_byte_order_with_every_field(pool: PgPool) {
        let result = data(&schema_for(&pool), ALL_FIELDS).await;
        let connection = &result["devices"];
        let listed = names(connection);
        let mut expected: Vec<_> = devices::list(&pool).await.unwrap().into_iter().map(|d| d.name).collect();
        expected.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
        assert_eq!(listed, expected);
        assert_eq!(listed.len(), 13);
        assert_eq!(connection["totalCount"], 13);
        assert_eq!(connection["pageInfo"]["hasNextPage"], false);
        let edges = connection["edges"].as_array().unwrap();
        assert_eq!(connection["pageInfo"]["endCursor"], edges[12]["cursor"]);

        let pve = edges.iter().find(|e| e["node"]["name"] == "pve-01").unwrap();
        let node = &pve["node"];
        assert_eq!(node["id"], "0190a000-0000-7000-8002-000000000001");
        assert_eq!(node["host"], "pve-01.home.lan");
        assert_eq!(node["ip"], "10.10.0.2");
        assert_eq!(node["kind"], "Proxmox VE node");
        assert_eq!(node["os"], "LINUX");
        assert_eq!(node["tags"], json!(["infra", "proxmox"]));
        assert_eq!(node["maintenance"], false);
        // R-033: compare instants, not text.
        let instant = |value: &Value| value.as_str().unwrap().parse::<chrono::DateTime<chrono::Utc>>().unwrap();
        assert_eq!(instant(&node["createdAt"]), "2026-11-01T00:00:00Z".parse::<chrono::DateTime<chrono::Utc>>().unwrap());
        assert_eq!(instant(&node["updatedAt"]), instant(&node["createdAt"]));
        assert_eq!(node["ssh"], json!({ "enabled": true, "port": 22, "username": "operator", "auth": "PRIVATE_KEY" }));
        assert_eq!(node["site"], json!({ "id": HOME_SITE, "name": "home" }));

        let docker = edges.iter().find(|e| e["node"]["name"] == "docker-host").unwrap();
        assert_eq!(docker["node"]["ip"], Value::Null);
        let jellyfin = edges.iter().find(|e| e["node"]["name"] == "jellyfin").unwrap();
        assert_eq!(jellyfin["node"]["maintenance"], true);
        let win = edges.iter().find(|e| e["node"]["name"] == "win-desktop").unwrap();
        assert_eq!(win["node"]["os"], "WINDOWS");
        assert_eq!(win["node"]["ssh"]["enabled"], false);
        assert_eq!(win["node"]["site"]["name"], "office-2");
    }

    async fn api_006_paging_by_two_follows_end_cursor_and_visits_each_device_once(pool: PgPool) {
        let schema = schema_for(&pool);
        let mut seen: Vec<String> = Vec::new();
        let mut after = String::new();
        loop {
            let query = format!(
                "{{ devices(first: 2{after}) {{ totalCount pageInfo {{ hasNextPage endCursor }} edges {{ cursor node {{ name }} }} }} }}"
            );
            let page = data(&schema, &query).await["devices"].clone();
            assert_eq!(page["totalCount"], 13);
            seen.extend(names(&page).into_iter().map(str::to_owned));
            let edges = page["edges"].as_array().unwrap();
            assert_eq!(page["pageInfo"]["endCursor"], edges.last().unwrap()["cursor"]);
            if page["pageInfo"]["hasNextPage"] == false {
                break;
            }
            assert_eq!(edges.len(), 2);
            after = format!(", after: {}", page["pageInfo"]["endCursor"]);
        }
        let all = data(&schema, "{ devices { edges { node { name } } } }").await;
        assert_eq!(seen, names(&all["devices"]));
    }

    async fn api_006_bad_paging_arguments_are_validation_errors_naming_the_field(pool: PgPool) {
        let schema = schema_for(&pool);
        for (query, field) in [
            (r#"{ devices(after: "garbage") { totalCount } }"#, "after"),
            ("{ devices(first: 0) { totalCount } }", "first"),
            ("{ devices(first: 201) { totalCount } }", "first"),
        ] {
            let error = only_error(&schema, query).await;
            assert_eq!(validation_fields(&error), [field], "{query}");
        }
        // One error per site, each naming the same field.
        let nested = schema.execute(signed_in("{ sites { devices(first: 0) { totalCount } } }")).await;
        assert_eq!(nested.errors.len(), 3);
        for error in &nested.errors {
            let error = serde_json::to_value(error).unwrap();
            assert_eq!(validation_fields(&error), ["first"]);
        }
        let both = only_error(&schema, r#"{ devices(first: 0, after: "x") { totalCount } }"#).await;
        assert_eq!(validation_fields(&both), ["first", "after"]);
    }

    async fn vpn_002_devices_can_be_filtered_by_site(pool: PgPool) {
        let schema = schema_for(&pool);
        let query = format!(r#"{{ devices(filter: {{ siteId: "{OFFICE_1_SITE}" }}) {{ totalCount edges {{ node {{ name }} }} }} }}"#);
        let result = data(&schema, &query).await;
        assert_eq!(names(&result["devices"]), ["mac-office-1"]);
        assert_eq!(result["devices"]["totalCount"], 1);
        let home = format!(r#"{{ devices(filter: {{ siteId: "{HOME_SITE}" }}) {{ totalCount }} }}"#);
        assert_eq!(data(&schema, &home).await["devices"]["totalCount"], 9);
    }

    async fn vpn_002_a_malformed_site_id_is_a_validation_error_on_site_id(pool: PgPool) {
        let error = only_error(&schema_for(&pool), r#"{ devices(filter: { siteId: "nope" }) { totalCount } }"#).await;
        assert_eq!(validation_fields(&error), ["siteId"]);
    }

    async fn dev_010_a_filter_over_its_limits_is_validation_on_each_field(pool: PgPool) {
        let schema = schema_for(&pool);
        let name = "n".repeat(65);
        let tags = vec!["t"; 33].join(r#"", ""#);
        let query = format!(r#"{{ devices(filter: {{ name: "{name}", tags: ["{tags}"] }}) {{ totalCount }} }}"#);
        let error = only_error(&schema, &query).await;
        assert_eq!(validation_fields(&error), ["name", "tags"]);
        let at_limit = format!(r#"{{ devices(filter: {{ name: "{}" }}) {{ totalCount }} }}"#, "n".repeat(64));
        assert_eq!(data(&schema, &at_limit).await["devices"]["totalCount"], 0);
    }

    async fn api_006_name_filter_is_a_case_insensitive_substring_and_tags_are_all_of(pool: PgPool) {
        let schema = schema_for(&pool);
        let by_name = data(&schema, r#"{ devices(filter: { name: "MAC-OFF" }) { totalCount edges { node { name } } } }"#).await;
        assert_eq!(names(&by_name["devices"]), ["mac-office-1", "mac-office-2"]);
        assert_eq!(by_name["devices"]["totalCount"], 2);
        let by_tags = data(&schema, r#"{ devices(filter: { tags: ["desktop", "build"] }) { edges { node { name } } } }"#).await;
        assert_eq!(names(&by_tags["devices"]), ["linux-pc-office-2", "mac-office-1"]);
        let combined = data(&schema, r#"{ devices(filter: { tags: ["desktop"], name: "linux" }, sort: NAME) { totalCount } }"#).await;
        assert_eq!(combined["devices"]["totalCount"], 2);
    }

    async fn api_006_device_by_id_returns_it_or_null_or_a_validation_error(pool: PgPool) {
        let schema = schema_for(&pool);
        let found = data(&schema, r#"{ device(id: "0190a000-0000-7000-8002-000000000006") { name maintenance site { name } } }"#).await;
        assert_eq!(found["device"], json!({ "name": "jellyfin", "maintenance": true, "site": { "name": "home" } }));
        let unknown = data(&schema, r#"{ device(id: "0190a000-0000-7000-8002-0000000000f1") { name } }"#).await;
        assert_eq!(unknown["device"], Value::Null);
        let bad = only_error(&schema, r#"{ device(id: "not-a-uuid") { name } }"#).await;
        assert_eq!(validation_fields(&bad), ["id"]);
    }

    async fn api_005_sites_group_their_devices_and_page_by_name(pool: PgPool) {
        let schema = schema_for(&pool);
        let result = data(&schema, "{ sites { name subnet devices { totalCount edges { node { name site { name } } } } } }").await;
        let sites = result["sites"].as_array().unwrap();
        let summary: Vec<_> = sites.iter().map(|s| (s["name"].as_str().unwrap(), s["subnet"].as_str().unwrap(), s["devices"]["totalCount"].as_i64().unwrap())).collect();
        assert_eq!(summary, [("home", "10.10.0.0/24", 9), ("office-1", "10.20.0.0/24", 1), ("office-2", "10.30.0.0/24", 3)]);
        assert_eq!(names(&sites[2]["devices"]), ["linux-pc-office-2", "mac-office-2", "win-desktop"]);
        for site in sites {
            for edge in site["devices"]["edges"].as_array().unwrap() {
                assert_eq!(edge["node"]["site"]["name"], site["name"]);
            }
        }
    }

    async fn api_006_site_devices_follow_end_cursor_with_a_stable_total(pool: PgPool) {
        let schema = schema_for(&pool);
        let first = data(&schema, "{ sites { devices(first: 1) { totalCount pageInfo { hasNextPage endCursor } edges { node { name } } } } }").await;
        let office_2 = &first["sites"][2]["devices"];
        assert_eq!(names(office_2), ["linux-pc-office-2"]);
        assert_eq!(office_2["totalCount"], 3);
        assert_eq!(office_2["pageInfo"]["hasNextPage"], true);
        assert_eq!(first["sites"][1]["devices"]["pageInfo"]["hasNextPage"], false);

        let after = office_2["pageInfo"]["endCursor"].clone();
        let query = format!(r"{{ sites {{ name devices(first: 1, after: {after}) {{ totalCount pageInfo {{ hasNextPage }} edges {{ node {{ name }} }} }} }} }}");
        let second = data(&schema, &query).await;
        let office_2 = second["sites"].as_array().unwrap().iter().find(|s| s["name"] == "office-2").unwrap();
        assert_eq!(names(&office_2["devices"]), ["mac-office-2"]);
        assert_eq!(office_2["devices"]["totalCount"], 3);
        assert_eq!(office_2["devices"]["pageInfo"]["hasNextPage"], true);
    }

    async fn api_006_tags_are_distinct_and_in_byte_order(pool: PgPool) {
        let result = data(&schema_for(&pool), "{ tags }").await;
        assert_eq!(
            result["tags"],
            json!(["ai", "build", "desktop", "dns", "docker", "home", "infra", "lxc", "media", "proxmox", "storage", "vm"])
        );
    }

    async fn k7_a_database_failure_is_internal_and_leaks_no_sql(pool: PgPool) {
        let schema = schema_for(&pool);
        sqlx::query("DROP SCHEMA devices CASCADE").execute(&pool).await.unwrap();
        for query in ["{ devices { totalCount } }", "{ sites { name } }", "{ tags }"] {
            let error = only_error(&schema, query).await;
            assert_eq!(error["extensions"]["code"], "INTERNAL", "{query}");
            let text = error.to_string();
            for leak in ["devices.", "SELECT", "relation", "schema"] {
                assert!(!text.contains(leak), "{query}: {text}");
            }
        }
    }

    async fn api_006_a_null_first_pages_by_the_default_of_50(pool: PgPool) {
        add_bulk(&pool, 0, 40).await;
        let result = data(&schema_for(&pool), "{ devices(first: null) { totalCount pageInfo { hasNextPage } edges { cursor } } }").await;
        assert_eq!(result["devices"]["edges"].as_array().unwrap().len(), 50);
        assert_eq!(result["devices"]["totalCount"], 53);
        assert_eq!(result["devices"]["pageInfo"]["hasNextPage"], true);
    }

    async fn k7_a_loader_failure_is_internal_and_leaks_no_sql(pool: PgPool) {
        let schema = schema_for(&pool);
        // The sites still load; the batched device query joins the dropped table.
        sqlx::query("DROP TABLE devices.ssh_settings").execute(&pool).await.unwrap();
        let response = schema.execute(signed_in("{ sites { name devices { totalCount } } }")).await;
        assert_eq!(response.errors.len(), 3);
        for error in &response.errors {
            let error = serde_json::to_value(error).unwrap();
            assert_eq!(error["extensions"]["code"], "INTERNAL", "{error}");
            let text = error.to_string();
            for leak in ["ssh_settings", "SELECT", "relation", "devices."] {
                assert!(!text.contains(leak), "{text}");
            }
        }
    }
}

#[tokio::test]
async fn k7_missing_schema_data_is_internal_without_rust_type_names() {
    let schema = Schema::build(Query, Mutation, EmptySubscription).finish();
    let error = only_error(&schema, "{ tags }").await;
    assert_eq!(error["extensions"]["code"], "INTERNAL");
    let text = error.to_string();
    for leak in ["PgPool", "sqlx", "Data `", "does not exist"] {
        assert!(!text.contains(leak), "{text}");
    }
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn api_010_a_missing_site_devices_loader_is_internal_without_rust_type_names(pool: PgPool) {
    let clock: Arc<dyn Clock> = Arc::new(FixedClock(now()));
    let schema = Schema::build(Query, Mutation, EmptySubscription)
        .data(pool)
        .data(clock)
        .finish();
    let response = schema
        .execute(signed_in("{ sites { devices { totalCount } } }"))
        .await;
    assert_eq!(response.errors.len(), 3);
    for error in &response.errors {
        let error = serde_json::to_value(error).unwrap();
        assert_eq!(error["extensions"]["code"], "INTERNAL", "{error}");
        let text = error.to_string();
        for leak in ["DataLoader", "DevicesBySite", "Data `", "does not exist"] {
            assert!(!text.contains(leak), "{text}");
        }
    }
}
