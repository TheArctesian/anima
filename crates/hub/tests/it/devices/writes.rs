//! `createSite`, `createDevice`, `updateDevice` and `deleteDevice` (DEV-010, DEV-015, VPN-001).

use serde_json::{Value, json};
use sqlx::PgPool;

use super::{
    harness::{error_json, now, only_error, schema_for, validation_fields, with_variables},
    row_count,
};

const HOME_SITE: &str = "0190a000-0000-7000-8001-000000000001";
const PVE_01: &str = "0190a000-0000-7000-8002-000000000001";
const CREATE_DEVICE: &str = "mutation($input: CreateDeviceInput!) { createDevice(input: $input) {
    id name host kind os tags maintenance createdAt updatedAt
    ssh { enabled port username auth } site { name } } }";
const UPDATE_DEVICE: &str = "mutation($id: ID!, $input: UpdateDeviceInput!) {
    updateDevice(id: $id, input: $input) { id name host kind os tags maintenance updatedAt
    ssh { enabled port username auth } site { name } } }";
const CREATE_SITE: &str = "mutation($name: String!, $subnet: Cidr!) {
    createSite(name: $name, subnet: $subnet) { id name subnet devices { totalCount } } }";

macro_rules! fixture_test {
    ($(async fn $name:ident($pool:ident: PgPool) $body:block)+) => {$(
        #[sqlx::test(
            migrations = "./migrations",
            fixtures(path = "../../fixtures", scripts("devices"))
        )]
        async fn $name($pool: PgPool) $body
    )+};
}

async fn created(pool: &PgPool, input: Value) -> Value {
    let response =
        with_variables(&schema_for(pool), CREATE_DEVICE, json!({ "input": input })).await;
    assert!(response.errors.is_empty(), "{:?}", response.errors);
    response.data.into_json().unwrap()["createDevice"].clone()
}

async fn failed(pool: &PgPool, query: &str, variables: Value) -> Value {
    error_json(&with_variables(&schema_for(pool), query, variables).await)
}

async fn logged(pool: &PgPool) -> (i64, i64) {
    let events = sqlx::query_scalar("SELECT count(*) FROM events.event_log");
    let audits = sqlx::query_scalar("SELECT count(*) FROM events.audit_log");
    (
        events.fetch_one(pool).await.unwrap(),
        audits.fetch_one(pool).await.unwrap(),
    )
}

fn pve_02() -> Value {
    json!({ "siteId": HOME_SITE, "name": "pve-02", "host": "pve-02.home.lan" })
}

fixture_test! {
    async fn vpn_001_create_site_returns_the_site_with_a_v7_id_and_no_devices(pool: PgPool) {
        let vars = json!({ "name": "office-3", "subnet": "10.40.0.0/24" });
        let response = with_variables(&schema_for(&pool), CREATE_SITE, vars).await;
        assert!(response.errors.is_empty(), "{:?}", response.errors);
        let site = &response.data.into_json().unwrap()["createSite"];
        assert_eq!(site["name"], "office-3");
        assert_eq!(site["subnet"], "10.40.0.0/24");
        assert_eq!(site["devices"]["totalCount"], 0);
        let id: uuid::Uuid = site["id"].as_str().unwrap().parse().unwrap();
        assert_eq!(id.get_version_num(), 7);
        assert_eq!(row_count(&pool, "sites").await, 4);
    }

    async fn vpn_001_a_subnet_that_overlaps_a_site_is_validation_on_subnet_and_stores_nothing(pool: PgPool) {
        let vars = json!({ "name": "overlap", "subnet": "10.10.0.128/25" });
        let error = failed(&pool, CREATE_SITE, vars).await;
        assert_eq!(validation_fields(&error), ["subnet"]);
        assert_eq!(row_count(&pool, "sites").await, 3);
        assert_eq!(logged(&pool).await, (0, 0));
    }

    async fn vpn_001_a_subnet_with_host_bits_is_refused_by_the_cidr_scalar(pool: PgPool) {
        let vars = json!({ "name": "bad", "subnet": "10.40.0.5/24" });
        let response = with_variables(&schema_for(&pool), CREATE_SITE, vars).await;
        assert_eq!(response.errors.len(), 1, "{:?}", response.errors);
        assert_eq!(row_count(&pool, "sites").await, 3);
        assert_eq!(logged(&pool).await, (0, 0));
    }

    async fn dev_010_create_device_applies_the_k1_defaults(pool: PgPool) {
        let device = created(&pool, pve_02()).await;
        assert_eq!(device["name"], "pve-02");
        assert_eq!(device["host"], "pve-02.home.lan");
        assert_eq!(device["kind"], "server");
        assert_eq!(device["os"], "OTHER");
        assert_eq!(device["tags"], json!([]));
        assert_eq!(device["maintenance"], false);
        assert_eq!(device["site"]["name"], "home");
        assert_eq!(device["ssh"], json!({ "enabled": false, "port": 22, "username": "", "auth": "PRIVATE_KEY" }));
        assert_eq!(device["createdAt"], device["updatedAt"]);
        assert!(device["createdAt"].as_str().unwrap().starts_with("2026-11-04T09:00:00"));
        let id: uuid::Uuid = device["id"].as_str().unwrap().parse().unwrap();
        assert_eq!(id.get_version_num(), 7);
        assert_eq!(now().to_rfc3339(), "2026-11-04T09:00:00+00:00");
    }

    async fn dev_010_create_device_with_tags_ssh_and_default_check_off_stores_them(pool: PgPool) {
        let mut input = pve_02();
        input["kind"] = json!("Proxmox VE node");
        input["os"] = json!("LINUX");
        input["tags"] = json!(["infra", " lab ", "infra"]);
        input["addDefaultCheck"] = json!(false);
        input["ssh"] = json!({ "enabled": true, "username": "operator", "auth": "PASSWORD" });
        let device = created(&pool, input).await;
        assert_eq!(device["kind"], "Proxmox VE node");
        assert_eq!(device["os"], "LINUX");
        assert_eq!(device["tags"], json!(["infra", "lab"]));
        assert_eq!(device["ssh"], json!({ "enabled": true, "port": 22, "username": "operator", "auth": "PASSWORD" }));
        assert_eq!(row_count(&pool, "devices").await, 14);
        assert_eq!(row_count(&pool, "ssh_settings").await, 14);
    }

    async fn dev_010_a_bad_host_is_validation_on_host_and_nothing_is_stored(pool: PgPool) {
        let mut input = pve_02();
        input["host"] = json!("http://pve-02");
        let error = failed(&pool, CREATE_DEVICE, json!({ "input": input })).await;
        assert_eq!(validation_fields(&error), ["host"]);
        assert_eq!(row_count(&pool, "devices").await, 13);
        assert_eq!(logged(&pool).await, (0, 0));
    }

    async fn log_004_control_and_direction_characters_in_a_name_are_validation_not_internal(pool: PgPool) {
        for bad in ["a\nb", "a\u{202E}b", "a\u{0000}b"] {
            let mut input = pve_02();
            input["name"] = json!(bad);
            let error = failed(&pool, CREATE_DEVICE, json!({ "input": input })).await;
            assert_eq!(validation_fields(&error), ["name"], "{bad:?}");
            let site = failed(&pool, CREATE_SITE, json!({ "name": bad, "subnet": "10.40.0.0/24" })).await;
            assert_eq!(validation_fields(&site), ["name"], "{bad:?}");
        }
        assert_eq!(logged(&pool).await, (0, 0));
    }

    async fn dev_010_every_bad_field_is_reported_at_once_including_a_malformed_site_id(pool: PgPool) {
        let input = json!({ "siteId": "nope", "name": "", "host": "bad host", "ssh": { "enabled": true, "username": "", "auth": "PASSWORD" } });
        let error = failed(&pool, CREATE_DEVICE, json!({ "input": input })).await;
        assert_eq!(validation_fields(&error), ["siteId", "name", "host", "ssh.username"]);
        assert_eq!(logged(&pool).await, (0, 0));
    }

    async fn vpn_002_an_unknown_site_id_is_validation_on_site_id(pool: PgPool) {
        let mut input = pve_02();
        input["siteId"] = json!("0190a000-0000-7000-8001-0000000000ff");
        let error = failed(&pool, CREATE_DEVICE, json!({ "input": input })).await;
        assert_eq!(validation_fields(&error), ["siteId"]);
        assert_eq!(logged(&pool).await, (0, 0));
    }

    async fn dev_010_a_duplicate_device_name_is_validation_on_name(pool: PgPool) {
        let mut input = pve_02();
        input["name"] = json!("pve-01");
        let error = failed(&pool, CREATE_DEVICE, json!({ "input": input })).await;
        assert_eq!(validation_fields(&error), ["name"]);
        assert_eq!(row_count(&pool, "devices").await, 13);
        assert_eq!(logged(&pool).await, (0, 0));
    }

    async fn dev_010_update_with_only_maintenance_changes_only_that_and_the_time(pool: PgPool) {
        let vars = json!({ "id": PVE_01, "input": { "maintenance": true } });
        let response = with_variables(&schema_for(&pool), UPDATE_DEVICE, vars).await;
        assert!(response.errors.is_empty(), "{:?}", response.errors);
        let device = &response.data.into_json().unwrap()["updateDevice"];
        assert_eq!(device["maintenance"], true);
        assert_eq!(device["name"], "pve-01");
        assert_eq!(device["host"], "pve-01.home.lan");
        assert_eq!(device["tags"], json!(["infra", "proxmox"]));
        assert_eq!(device["ssh"]["username"], "operator");
        assert!(device["updatedAt"].as_str().unwrap().starts_with("2026-11-04T09:00:00"));
    }

    async fn dev_010_update_replaces_tags_and_checks_the_fields_it_gets(pool: PgPool) {
        let vars = json!({ "id": PVE_01, "input": { "tags": ["edge"], "name": "pve-1" } });
        let response = with_variables(&schema_for(&pool), UPDATE_DEVICE, vars).await;
        assert!(response.errors.is_empty(), "{:?}", response.errors);
        let device = &response.data.into_json().unwrap()["updateDevice"];
        assert_eq!((device["tags"].clone(), device["name"].clone()), (json!(["edge"]), json!("pve-1")));

        let bad = json!({ "id": PVE_01, "input": { "host": "a b", "siteId": "x" } });
        let error = failed(&pool, UPDATE_DEVICE, bad).await;
        assert_eq!(validation_fields(&error), ["siteId", "host"]);
    }

    async fn dev_010_update_of_an_unknown_device_is_not_found_and_logs_nothing(pool: PgPool) {
        let vars = json!({ "id": "0190a000-0000-7000-8002-0000000000ff", "input": { "maintenance": true } });
        let error = failed(&pool, UPDATE_DEVICE, vars).await;
        assert_eq!(error["extensions"]["code"], "NOT_FOUND");
        let bad_id = json!({ "id": "nope", "input": {} });
        let error = failed(&pool, UPDATE_DEVICE, bad_id).await;
        assert_eq!(validation_fields(&error), ["id"]);
        assert_eq!(logged(&pool).await, (0, 0));
    }

    async fn dev_015_delete_returns_the_id_and_removes_the_device_with_tags_and_ssh(pool: PgPool) {
        let schema = schema_for(&pool);
        let query = "mutation($id: ID!) { deleteDevice(id: $id) }";
        let response = with_variables(&schema, query, json!({ "id": PVE_01 })).await;
        assert!(response.errors.is_empty(), "{:?}", response.errors);
        assert_eq!(response.data.into_json().unwrap()["deleteDevice"], PVE_01);
        for table in ["devices", "ssh_settings", "device_tags"] {
            let column = if table == "devices" { "id" } else { "device_id" };
            let left: i64 = sqlx::query_scalar(&format!("SELECT count(*) FROM devices.{table} WHERE {column} = $1::text::uuid"))
                .bind(PVE_01).fetch_one(&pool).await.unwrap();
            assert_eq!(left, 0, "{table}");
        }
        let again = error_json(&with_variables(&schema, query, json!({ "id": PVE_01 })).await);
        assert_eq!(again["extensions"]["code"], "NOT_FOUND");
        assert_eq!(logged(&pool).await, (1, 1));
        let malformed = only_error(&schema, r#"mutation { deleteDevice(id: "nope") }"#).await;
        assert_eq!(validation_fields(&malformed), ["id"]);
    }
}
