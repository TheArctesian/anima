use std::collections::BTreeSet;

pub(crate) mod batching;
mod concurrency;
mod credential;
mod face;
mod fields;
pub(crate) mod graphql_read;
mod guard;
mod harness;
mod leak;
mod password_credential;
mod trail;
mod writes;

use serde_json::Value;
use sqlx::PgPool;

const PVE_01: &str = "0190a000-0000-7000-8002-000000000001";
const JELLYFIN: &str = "0190a000-0000-7000-8002-000000000006";
const DOCKER_HOST: &str = "0190a000-0000-7000-8002-000000000002";

fn phase1() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../contracts/fixtures/phase1.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn text(value: &Value) -> &str {
    value.as_str().unwrap()
}

fn constraint_of(error: &sqlx::Error) -> Option<&str> {
    error.as_database_error().and_then(|e| e.constraint())
}

async fn row_count(pool: &PgPool, table: &str) -> usize {
    let total: i64 = sqlx::query_scalar(&format!("SELECT count(*) FROM devices.{table}"))
        .fetch_one(pool)
        .await
        .unwrap();
    usize::try_from(total).unwrap()
}

async fn count(pool: &PgPool, sql: &str, device_id: &str) -> i64 {
    sqlx::query_scalar(sql)
        .bind(device_id.parse::<uuid::Uuid>().unwrap())
        .fetch_one(pool)
        .await
        .unwrap()
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../fixtures", scripts("devices"))
)]
async fn k6_devices_fixture_matches_phase1_json(pool: PgPool) {
    let json = phase1();

    for site in json["sites"].as_array().unwrap() {
        let row: (String, String) = sqlx::query_as(
            "SELECT name, subnet::text FROM devices.sites WHERE id = $1::text::uuid",
        )
        .bind(text(&site["id"]))
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(
            row,
            (text(&site["name"]).into(), text(&site["subnet"]).into())
        );
    }

    let devices = json["devices"].as_array().unwrap();
    let tag_total: usize = devices
        .iter()
        .map(|d| d["tags"].as_array().unwrap().len())
        .sum();
    assert_eq!(
        row_count(&pool, "sites").await,
        json["sites"].as_array().unwrap().len()
    );
    assert_eq!(row_count(&pool, "devices").await, devices.len());
    assert_eq!(row_count(&pool, "device_tags").await, tag_total);
    assert_eq!(row_count(&pool, "ssh_settings").await, devices.len());

    for device in devices {
        let id = text(&device["id"]);
        let row: (String, String, Option<String>, String, String, bool, String) = sqlx::query_as(
            "SELECT name, host, host(ip), kind, os, maintenance, site_id::text \
             FROM devices.devices WHERE id = $1::text::uuid",
        )
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();
        let expected = (
            text(&device["name"]).to_owned(),
            text(&device["host"]).to_owned(),
            device["ip"].as_str().map(str::to_owned),
            text(&device["kind"]).to_owned(),
            text(&device["os"]).to_lowercase(),
            device["maintenance"].as_bool().unwrap(),
            text(&device["siteId"]).to_owned(),
        );
        assert_eq!(row, expected, "device {id}");

        let tags: Vec<String> = sqlx::query_scalar(
            "SELECT tag FROM devices.device_tags WHERE device_id = $1::text::uuid",
        )
        .bind(id)
        .fetch_all(&pool)
        .await
        .unwrap();
        let expected_tags: BTreeSet<&str> = device["tags"]
            .as_array()
            .unwrap()
            .iter()
            .map(text)
            .collect();
        assert_eq!(
            tags.iter().map(String::as_str).collect::<BTreeSet<_>>(),
            expected_tags
        );

        let ssh: (bool, i32, String, String) = sqlx::query_as(
            "SELECT enabled, port, username, auth FROM devices.ssh_settings \
             WHERE device_id = $1::text::uuid",
        )
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();
        let expected_ssh = &device["ssh"];
        assert_eq!(
            ssh,
            (
                expected_ssh["enabled"].as_bool().unwrap(),
                i32::try_from(expected_ssh["port"].as_i64().unwrap()).unwrap(),
                text(&expected_ssh["username"]).to_owned(),
                text(&expected_ssh["auth"]).to_lowercase(),
            ),
            "ssh settings of {id}"
        );
    }
}

async fn insert_site(pool: &PgPool, name: &str, subnet: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO devices.sites (id, name, subnet) VALUES (gen_random_uuid(), $1, $2::cidr)",
    )
    .bind(name)
    .bind(subnet)
    .execute(pool)
    .await
    .map(|_| ())
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../fixtures", scripts("devices"))
)]
async fn vpn_001_overlapping_site_subnet_is_rejected(pool: PgPool) {
    let error = insert_site(&pool, "overlap", "10.10.0.128/25")
        .await
        .unwrap_err();
    assert_eq!(constraint_of(&error), Some("sites_subnet_no_overlap"));

    insert_site(&pool, "office-3", "10.40.0.0/24")
        .await
        .unwrap();
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../fixtures", scripts("devices"))
)]
async fn dev_015_deleting_a_device_removes_its_tags_and_ssh_settings(pool: PgPool) {
    let tags = "SELECT count(*) FROM devices.device_tags WHERE device_id = $1";
    let ssh = "SELECT count(*) FROM devices.ssh_settings WHERE device_id = $1";
    assert_eq!(count(&pool, tags, PVE_01).await, 2);
    assert_eq!(count(&pool, ssh, PVE_01).await, 1);

    sqlx::query("DELETE FROM devices.devices WHERE id = $1::text::uuid")
        .bind(PVE_01)
        .execute(&pool)
        .await
        .unwrap();

    assert_eq!(count(&pool, tags, PVE_01).await, 0);
    assert_eq!(count(&pool, ssh, PVE_01).await, 0);
    assert_eq!(count(&pool, tags, DOCKER_HOST).await, 2);
    assert_eq!(count(&pool, ssh, DOCKER_HOST).await, 1);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../fixtures", scripts("devices"))
)]
async fn k2_duplicate_device_name_fails_on_the_named_constraint(pool: PgPool) {
    let error = sqlx::query(
        "INSERT INTO devices.devices (id, site_id, name, host, kind, os, created_at, updated_at) \
         VALUES (gen_random_uuid(), $1::text::uuid, 'pve-01', 'other.lan', 'Server', 'linux', now(), now())",
    )
    .bind("0190a000-0000-7000-8001-000000000002")
    .execute(&pool)
    .await
    .unwrap_err();
    assert_eq!(constraint_of(&error), Some("devices_name_key"));
}

const HOME_SITE: &str = "0190a000-0000-7000-8001-000000000001";
const NEW_DEVICE: &str = "0190a000-0000-7000-8002-0000000000f1";

async fn insert_device(pool: &PgPool, site_id: Option<&str>, os: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO devices.devices (id, site_id, name, host, kind, os, created_at, updated_at) \
         VALUES ($1::text::uuid, $2::text::uuid, 'fresh', 'fresh.lan', 'Server', $3, now(), now())",
    )
    .bind(NEW_DEVICE)
    .bind(site_id)
    .bind(os)
    .execute(pool)
    .await
    .map(|_| ())
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../fixtures", scripts("devices"))
)]
async fn vpn_002_deleting_a_site_that_still_has_devices_is_rejected(pool: PgPool) {
    let error = sqlx::query("DELETE FROM devices.sites WHERE id = $1::text::uuid")
        .bind(HOME_SITE)
        .execute(&pool)
        .await
        .unwrap_err();
    assert_eq!(constraint_of(&error), Some("devices_site_id_fkey"));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../fixtures", scripts("devices"))
)]
async fn vpn_002_a_device_without_a_site_is_rejected(pool: PgPool) {
    let error = insert_device(&pool, None, "linux").await.unwrap_err();
    let database = error.as_database_error().unwrap();
    assert_eq!(database.code().as_deref(), Some("23502"));
    assert!(database.message().contains("site_id"));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../fixtures", scripts("devices"))
)]
async fn k2_bad_os_port_and_auth_values_are_rejected_by_named_checks(pool: PgPool) {
    let bad_os = insert_device(&pool, Some(HOME_SITE), "beos")
        .await
        .unwrap_err();
    assert_eq!(constraint_of(&bad_os), Some("devices_os_check"));

    let cases = [
        (0, "password", "ssh_settings_port_check"),
        (65536, "password", "ssh_settings_port_check"),
        (22, "kerberos", "ssh_settings_auth_check"),
    ];
    for (port, auth, expected) in cases {
        let error = sqlx::query(
            "UPDATE devices.ssh_settings SET port = $1, auth = $2 \
             WHERE device_id = $3::text::uuid",
        )
        .bind(port)
        .bind(auth)
        .bind(PVE_01)
        .execute(&pool)
        .await
        .unwrap_err();
        assert_eq!(
            constraint_of(&error),
            Some(expected),
            "port {port} auth {auth}"
        );
    }
}
