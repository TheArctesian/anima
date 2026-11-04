use sqlx::Connection;

use super::*;
use crate::devices::store;

const NEW_ID: &str = "0190a000-0000-7000-8002-0000000000f1";
const NEW_SITE: &str = "0190a000-0000-7000-8001-0000000000f1";
const NOW: &str = "2026-11-02T10:00:00Z";

fn field_of(error: Error) -> &'static str {
    match error {
        Error::Validation(errors) => errors[0].field,
        other => panic!("expected a validation error, got {other:?}"),
    }
}

fn new_site(name: &str, subnet: &str) -> NewSite {
    NewSite {
        name: name.into(),
        subnet: subnet.parse().unwrap(),
    }
}

async fn create(pool: &sqlx::PgPool, device: &NewDevice) -> Result<Device, Error> {
    let mut conn = pool.acquire().await.unwrap();
    store::create_device(&mut conn, device_id(NEW_ID), instant(NOW), device).await
}

async fn rows(pool: &sqlx::PgPool, table: &str, id: &str) -> i64 {
    let column = if table == "devices" {
        "id"
    } else {
        "device_id"
    };
    sqlx::query_scalar(&format!(
        "SELECT count(*) FROM devices.{table} WHERE {column} = $1::text::uuid"
    ))
    .bind(id)
    .fetch_one(pool)
    .await
    .unwrap()
}

store_test! {
    async fn vpn_001_site_with_an_overlapping_subnet_is_a_subnet_error(pool: PgPool) {
        let mut conn = pool.acquire().await.unwrap();
        let id = site_id(NEW_SITE);
        // Inside home 10.10.0.0/24, and identical to it: both are overlaps.
        for subnet in ["10.10.0.128/25", "10.10.0.0/24", "10.0.0.0/8"] {
            let error = store::create_site(&mut conn, id, &new_site("lab", subnet)).await;
            assert_eq!(field_of(error.unwrap_err()), "subnet", "{subnet}");
        }
        let created = store::create_site(&mut conn, id, &new_site("lab", "10.99.0.0/24"))
            .await
            .unwrap();
        assert_eq!(created.name, "lab");
        assert_eq!(created.subnet.to_string(), "10.99.0.0/24");
    }

    async fn k2_duplicate_site_name_is_a_name_error(pool: PgPool) {
        let mut conn = pool.acquire().await.unwrap();
        let error = store::create_site(&mut conn, site_id(NEW_SITE), &new_site("home", "10.99.0.0/24"))
            .await
            .unwrap_err();
        assert_eq!(field_of(error), "name");
    }

    async fn k3_create_device_without_ssh_gets_the_default_settings_row(pool: PgPool) {
        let created = create(&pool, &record("new-box")).await.unwrap();
        assert_eq!(created.id, device_id(NEW_ID));
        assert_eq!(created.ssh, SshSettings::default());
        assert!(!created.maintenance);
        assert_eq!(created.ip, None);
        assert_eq!((created.created_at, created.updated_at), (instant(NOW), instant(NOW)));
        assert_eq!(rows(&pool, "ssh_settings", NEW_ID).await, 1);
    }

    async fn k3_create_device_stores_tags_and_ssh_settings(pool: PgPool) {
        let device = NewDevice {
            tags: vec!["web".into(), "alpha".into()],
            ssh: SshSettings {
                enabled: true,
                port: 2222,
                username: "ops".into(),
                auth: SshAuth::Password,
            },
            ..record("new-box")
        };
        let created = create(&pool, &device).await.unwrap();
        assert_eq!(created.tags, ["alpha", "web"]);
        assert_eq!(created.ssh, device.ssh);
        assert_eq!(store::get(&pool, created.id).await.unwrap(), created);
    }

    async fn k3_create_device_in_an_unknown_site_is_a_site_id_error(pool: PgPool) {
        let device = NewDevice { site: site_id(NEW_SITE), ..record("new-box") };
        assert_eq!(field_of(create(&pool, &device).await.unwrap_err()), "siteId");
        assert_eq!(rows(&pool, "devices", NEW_ID).await, 0);
    }

    async fn k3_create_device_with_a_taken_name_is_a_name_error_and_leaves_nothing(pool: PgPool) {
        let error = create(&pool, &record("pve-01")).await.unwrap_err();
        assert_eq!(field_of(error), "name");
        assert_eq!(rows(&pool, "ssh_settings", NEW_ID).await, 0);
    }

    async fn k3_failed_create_inside_a_transaction_rolls_back_with_it(pool: PgPool) {
        let mut conn = pool.acquire().await.unwrap();
        let mut tx = conn.begin().await.unwrap();
        let device = NewDevice { tags: vec!["x".into()], ..record("rolled-back") };
        store::create_device(&mut tx, device_id(NEW_ID), instant(NOW), &device).await.unwrap();
        tx.rollback().await.unwrap();
        assert_eq!(rows(&pool, "devices", NEW_ID).await, 0);
        assert_eq!(rows(&pool, "device_tags", NEW_ID).await, 0);
    }

    async fn k3_update_of_only_maintenance_leaves_the_rest_unchanged(pool: PgPool) {
        let before = store::get(&pool, device_id(PVE_01)).await.unwrap();
        let change = DeviceChange { maintenance: Some(true), ..DeviceChange::default() };
        let mut conn = pool.acquire().await.unwrap();
        let after = store::update_device(&mut conn, before.id, instant(NOW), &change).await.unwrap();
        let expected = Device { maintenance: true, updated_at: instant(NOW), ..before };
        assert_eq!(after, expected);
    }

    async fn k3_update_replaces_tags_and_ssh_wholly(pool: PgPool) {
        let ssh = SshSettings { enabled: false, port: 2200, username: "ops".into(), auth: SshAuth::Password };
        let change = DeviceChange {
            tags: Some(vec!["only".into()]),
            ssh: Some(ssh.clone()),
            name: Some("pve-renamed".into()),
            site: Some(site_id(OFFICE_1)),
            os: Some(OperatingSystem::Other),
            ..DeviceChange::default()
        };
        let mut conn = pool.acquire().await.unwrap();
        let after = store::update_device(&mut conn, device_id(PVE_01), instant(NOW), &change).await.unwrap();
        assert_eq!(after.tags, ["only"]);
        assert_eq!(after.ssh, ssh);
        assert_eq!((after.name.as_str(), after.site, after.os), ("pve-renamed", site_id(OFFICE_1), OperatingSystem::Other));
        assert_eq!(rows(&pool, "device_tags", PVE_01).await, 1);
        let cleared = DeviceChange { tags: Some(vec![]), ..DeviceChange::default() };
        let after = store::update_device(&mut conn, device_id(PVE_01), instant(NOW), &cleared).await.unwrap();
        assert_eq!(after.tags.len(), 0);
    }

    async fn k3_update_to_a_taken_name_or_unknown_site_is_a_field_error(pool: PgPool) {
        let mut conn = pool.acquire().await.unwrap();
        let taken = DeviceChange { name: Some("ollama".into()), ..DeviceChange::default() };
        let error = store::update_device(&mut conn, device_id(PVE_01), instant(NOW), &taken).await;
        assert_eq!(field_of(error.unwrap_err()), "name");
        let nowhere = DeviceChange { site: Some(site_id(NEW_SITE)), ..DeviceChange::default() };
        let error = store::update_device(&mut conn, device_id(PVE_01), instant(NOW), &nowhere).await;
        assert_eq!(field_of(error.unwrap_err()), "siteId");
    }

    async fn k3_update_of_an_unknown_device_is_not_found(pool: PgPool) {
        let mut conn = pool.acquire().await.unwrap();
        let error = store::update_device(&mut conn, device_id(NEW_ID), instant(NOW), &DeviceChange::default()).await;
        assert!(matches!(error, Err(Error::NotFound(Missing::Device(_)))));
    }

    async fn dev_015_delete_returns_the_device_and_leaves_no_tag_or_ssh_rows(pool: PgPool) {
        let mut conn = pool.acquire().await.unwrap();
        let before = store::get(&pool, device_id(PVE_01)).await.unwrap();
        let deleted = store::delete_device(&mut conn, before.id).await.unwrap();
        assert_eq!(deleted, before);
        assert_eq!(rows(&pool, "devices", PVE_01).await, 0);
        assert_eq!(rows(&pool, "device_tags", PVE_01).await, 0);
        assert_eq!(rows(&pool, "ssh_settings", PVE_01).await, 0);
        let again = store::delete_device(&mut conn, before.id).await;
        assert!(matches!(again, Err(Error::NotFound(Missing::Device(_)))));
        assert_eq!(store::list_sites(&pool).await.unwrap().len(), 3);
    }
}
