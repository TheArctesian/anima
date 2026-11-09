//! Remote access settings, the MAC address and the wake relay of `devices` (RA-030..033, D-2,
//! D-7, DB-011), read back through the K3 face.

mod passwords;
mod read_back;
mod relay;
mod session_controls;

use anima_hub::{
    devices::{
        self, Error, MacAddress, MetricsSource, Missing, RdpSecurity, RdpSettings, ResizeMethod,
        TransferPolicy, VncSettings,
    },
    ids::{DeviceId, SiteId},
};
use serde_json::{Value, json};
use sqlx::PgPool;

use super::harness::{error_json, schema_for, validation_fields, with_variables};

const HOME: &str = "0190a000-0000-7000-8001-000000000001";
const OFFICE_1: &str = "0190a000-0000-7000-8001-000000000002";
const OFFICE_2: &str = "0190a000-0000-7000-8001-000000000003";
const PVE_01: &str = "0190a000-0000-7000-8002-000000000001";
const MAC_OFFICE_1: &str = "0190a000-0000-7000-8002-00000000000a";
const CREATE: &str = "mutation($input: CreateDeviceInput!) { createDevice(input: $input) { id } }";
const UPDATE: &str = "mutation($id: ID!, $input: UpdateDeviceInput!) {
    updateDevice(id: $id, input: $input) { id } }";

macro_rules! fixture_test {
    ($(async fn $name:ident($pool:ident: PgPool) $body:block)+) => {$(
        #[sqlx::test(
            migrations = "./migrations",
            fixtures(path = "../../fixtures", scripts("devices"))
        )]
        async fn $name($pool: PgPool) $body
    )+};
}

fn device(text: &str) -> DeviceId {
    text.parse().unwrap()
}

fn site(text: &str) -> SiteId {
    text.parse().unwrap()
}

async fn create(pool: &PgPool, extra: Value) -> devices::Device {
    let mut input = json!({ "siteId": HOME, "name": "win-01", "host": "10.10.0.60" });
    input
        .as_object_mut()
        .unwrap()
        .extend(extra.as_object().unwrap().clone());
    let response = with_variables(&schema_for(pool), CREATE, json!({ "input": input })).await;
    assert!(response.errors.is_empty(), "{:?}", response.errors);
    let data = response.data.into_json().unwrap();
    devices::get(pool, device(data["createDevice"]["id"].as_str().unwrap()))
        .await
        .unwrap()
}

async fn update(pool: &PgPool, id: &str, input: Value) -> devices::Device {
    let vars = json!({ "id": id, "input": input });
    let response = with_variables(&schema_for(pool), UPDATE, vars).await;
    assert!(response.errors.is_empty(), "{:?}", response.errors);
    devices::get(pool, device(id)).await.unwrap()
}

async fn set_relay(pool: &PgPool, site_id: &str, device_id: &str) {
    sqlx::query(
        "UPDATE devices.sites SET wake_relay_device_id = $2::text::uuid WHERE id = $1::text::uuid",
    )
    .bind(site_id)
    .bind(device_id)
    .execute(pool)
    .await
    .unwrap();
}

fn mac(text: &str) -> MacAddress {
    text.parse().unwrap()
}

fn full_rdp() -> Value {
    json!({ "enabled": true, "port": 3390, "username": "admin", "domain": "CORP",
        "security": "TLS", "ignoreCertificate": true, "resizeMethod": "RECONNECT",
        "audioOut": false, "microphone": true, "sharedDrive": true })
}

fixture_test! {
    async fn db_011_a_device_inserted_by_sql_reads_back_the_rust_defaults(pool: PgPool) {
        let found = devices::get(&pool, device(PVE_01)).await.unwrap();
        assert_eq!(found.rdp, RdpSettings::default());
        assert_eq!(found.vnc, VncSettings::default());
        assert_eq!(found.transfer, TransferPolicy::default());
        assert_eq!(found.metrics_source, MetricsSource::default());
        assert_eq!(found.mac_address, None);
    }

    async fn ra_030_create_without_settings_reads_back_the_k1_defaults(pool: PgPool) {
        let created = create(&pool, json!({})).await;
        assert_eq!(created.rdp, RdpSettings::default());
        assert_eq!(created.rdp.port, 3389);
        assert_eq!(created.vnc.port, 5900);
        assert_eq!(created.transfer, TransferPolicy::default());
        assert_eq!(created.metrics_source, MetricsSource::None);
        assert_eq!(created.mac_address, None);
    }

    async fn ra_030_create_stores_every_setting_the_operator_sent(pool: PgPool) {
        let created = create(&pool, json!({
            "rdp": full_rdp(),
            "vnc": { "enabled": true, "username": " viewer " },
            "macAddress": "AA-BB-0C-DD-EE-F2",
            "metricsSource": "SSH",
            "transfer": { "upload": false, "paste": false },
        })).await;
        assert_eq!(created.rdp, RdpSettings {
            enabled: true, port: 3390, username: "admin".into(), domain: Some("CORP".into()),
            security: RdpSecurity::Tls, ignore_certificate: true, resize: ResizeMethod::Reconnect,
            audio_out: false, microphone: true, shared_drive: true,
        });
        assert_eq!(created.vnc, VncSettings {
            enabled: true, port: 5900, username: Some("viewer".into()),
        });
        assert_eq!(created.mac_address, Some(mac("aa:bb:0c:dd:ee:f2")));
        assert_eq!(created.metrics_source, MetricsSource::Ssh);
        assert_eq!(created.transfer, TransferPolicy {
            upload: false, download: true, copy: true, paste: false,
        });
    }

    async fn ra_031_create_reads_a_blank_mac_address_as_none(pool: PgPool) {
        assert_eq!(create(&pool, json!({ "macAddress": "" })).await.mac_address, None);
    }

    async fn d2_update_replaces_rdp_wholly_and_leaves_vnc_and_transfer(pool: PgPool) {
        let created = create(&pool, json!({
            "rdp": full_rdp(),
            "vnc": { "enabled": true, "port": 5901, "username": "viewer" },
            "transfer": { "copy": false },
        })).await;
        let id = created.id.to_string();
        let changed = update(&pool, &id, json!({
            "rdp": { "enabled": true, "username": "other" },
        })).await;
        assert_eq!(changed.rdp, RdpSettings {
            enabled: true, username: "other".into(), ..RdpSettings::default()
        });
        assert_eq!(changed.vnc, created.vnc);
        assert_eq!(changed.transfer, created.transfer);
        assert_eq!(changed.mac_address, created.mac_address);
    }

    async fn d2_update_replaces_vnc_and_transfer_wholly_and_leaves_rdp(pool: PgPool) {
        let created = create(&pool, json!({
            "rdp": full_rdp(),
            "vnc": { "enabled": true, "port": 5901, "username": "viewer" },
            "transfer": { "copy": false },
            "metricsSource": "SSH",
        })).await;
        let changed = update(&pool, &created.id.to_string(), json!({
            "vnc": { "enabled": false },
            "transfer": { "download": false },
        })).await;
        assert_eq!(changed.vnc, VncSettings::default());
        assert_eq!(changed.transfer, TransferPolicy { download: false, ..TransferPolicy::default() });
        assert_eq!(changed.rdp, created.rdp);
        assert_eq!(changed.metrics_source, MetricsSource::Ssh);
        let again = update(&pool, &created.id.to_string(), json!({ "metricsSource": "NONE" })).await;
        assert_eq!(again.metrics_source, MetricsSource::None);
        assert_eq!(again.transfer, changed.transfer);
    }

    async fn d2_update_with_a_null_domain_or_vnc_username_clears_it(pool: PgPool) {
        let created = create(&pool, json!({
            "rdp": full_rdp(),
            "vnc": { "enabled": true, "username": "viewer" },
        })).await;
        let id = created.id.to_string();
        let cleared = update(&pool, &id, json!({
            "rdp": { "enabled": true, "username": "admin", "domain": null },
            "vnc": { "enabled": true, "username": "  " },
        })).await;
        assert_eq!(cleared.rdp.domain, None);
        assert_eq!(cleared.vnc.username, None);
    }

    async fn d2_update_of_mac_address_leaves_clears_or_sets_it(pool: PgPool) {
        let created = create(&pool, json!({ "macAddress": "02:00:5e:00:00:fb" })).await;
        let id = created.id.to_string();
        let kept = update(&pool, &id, json!({ "name": "win-02" })).await;
        assert_eq!(kept.mac_address, Some(mac("02:00:5e:00:00:fb")));
        let kept = update(&pool, &id, json!({ "macAddress": null })).await;
        assert_eq!(kept.mac_address, Some(mac("02:00:5e:00:00:fb")));
        let set = update(&pool, &id, json!({ "macAddress": "02-00-5E-00-00-FC" })).await;
        assert_eq!(set.mac_address, Some(mac("02:00:5e:00:00:fc")));
        let cleared = update(&pool, &id, json!({ "macAddress": "" })).await;
        assert_eq!(cleared.mac_address, None);
    }

    async fn ra_030_invalid_settings_are_reported_together_by_field_path(pool: PgPool) {
        let input = json!({
            "siteId": HOME, "name": "win-01", "host": "10.10.0.60",
            "rdp": { "enabled": true, "port": 0, "username": "", "domain": "a b" },
            "vnc": { "enabled": true, "port": 70000, "username": "x y" },
            "macAddress": "ff:ff:ff:ff:ff:ff",
        });
        let response = with_variables(&schema_for(&pool), CREATE, json!({ "input": input })).await;
        let error = error_json(&response);
        assert_eq!(
            validation_fields(&error),
            ["rdp.port", "rdp.username", "rdp.domain", "vnc.port", "vnc.username", "macAddress"]
        );
        let named: i64 = sqlx::query_scalar("SELECT count(*) FROM devices.devices WHERE name = 'win-01'")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(named, 0);
    }

    async fn ra_030_an_invalid_update_changes_nothing(pool: PgPool) {
        let vars = json!({ "id": PVE_01, "input": { "name": "renamed", "macAddress": "00:00:00:00:00:00",
            "rdp": { "enabled": true, "username": "" } } });
        let error = error_json(&with_variables(&schema_for(&pool), UPDATE, vars).await);
        assert_eq!(validation_fields(&error), ["rdp.username", "macAddress"]);
        assert_eq!(devices::get(&pool, device(PVE_01)).await.unwrap().name, "pve-01");
    }

    async fn ra_030_device_added_and_updated_audit_entries_hold_the_settings(pool: PgPool) {
        let created = create(&pool, json!({ "rdp": full_rdp(), "macAddress": "02:00:5e:00:00:fb" })).await;
        update(&pool, &created.id.to_string(), json!({
            "vnc": { "enabled": true }, "macAddress": "", "transfer": { "upload": false },
        })).await;
        let changes: Vec<Value> = sqlx::query_scalar(
            "SELECT change FROM events.audit_log WHERE action IN ('create', 'update') ORDER BY at, id",
        )
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(changes[0]["rdp"]["domain"], "CORP");
        assert_eq!(changes[0]["rdp"]["security"], "tls");
        assert_eq!(changes[0]["macAddress"], "02:00:5e:00:00:fb");
        assert_eq!(changes[0]["transfer"]["upload"], true);
        assert_eq!(changes[1]["vnc"]["port"], 5900);
        assert_eq!(changes[1]["macAddress"], Value::Null);
        assert_eq!(changes[1]["transfer"]["upload"], false);
        assert!(changes[1].get("rdp").is_none());
    }

    async fn d7_wake_relay_is_none_when_unset_and_the_device_when_set(pool: PgPool) {
        assert!(devices::wake_relay(&pool, site(HOME)).await.unwrap().is_none());
        set_relay(&pool, HOME, PVE_01).await;
        let relay = devices::wake_relay(&pool, site(HOME)).await.unwrap().unwrap();
        assert_eq!(relay.id, device(PVE_01));
        assert_eq!(relay.name, "pve-01");
    }

    async fn d7_wake_relay_of_an_unknown_site_is_not_found(pool: PgPool) {
        let missing = site("0190a000-0000-7000-8001-0000000000f1");
        let error = devices::wake_relay(&pool, missing).await.unwrap_err();
        assert!(matches!(error, Error::NotFound(Missing::Site(found)) if found == missing));
    }

    async fn d7_deleting_the_relay_device_clears_the_sites_relay(pool: PgPool) {
        set_relay(&pool, HOME, PVE_01).await;
        let query = "mutation($id: ID!) { deleteDevice(id: $id) }";
        let response = with_variables(&schema_for(&pool), query, json!({ "id": PVE_01 })).await;
        assert!(response.errors.is_empty(), "{:?}", response.errors);
        assert!(devices::wake_relay(&pool, site(HOME)).await.unwrap().is_none());
    }

    async fn d7_moving_the_relay_device_to_another_site_clears_the_old_relay(pool: PgPool) {
        set_relay(&pool, OFFICE_1, MAC_OFFICE_1).await;
        update(&pool, MAC_OFFICE_1, json!({ "siteId": OFFICE_2 })).await;
        assert!(devices::wake_relay(&pool, site(OFFICE_1)).await.unwrap().is_none());
        assert!(devices::wake_relay(&pool, site(OFFICE_2)).await.unwrap().is_none());
    }

    async fn d7_turning_the_relay_devices_ssh_off_clears_its_relay(pool: PgPool) {
        set_relay(&pool, HOME, PVE_01).await;
        let on = json!({ "ssh": { "enabled": true, "port": 22, "username": "root", "auth": "PASSWORD" } });
        update(&pool, PVE_01, on).await;
        assert!(devices::wake_relay(&pool, site(HOME)).await.unwrap().is_some());
        let off = json!({ "ssh": { "enabled": false, "port": 22, "username": "root", "auth": "PASSWORD" } });
        update(&pool, PVE_01, off).await;
        assert!(devices::wake_relay(&pool, site(HOME)).await.unwrap().is_none());
    }

    async fn d7_an_update_that_keeps_the_site_or_moves_another_device_keeps_the_relay(pool: PgPool) {
        set_relay(&pool, OFFICE_1, MAC_OFFICE_1).await;
        update(&pool, MAC_OFFICE_1, json!({ "siteId": OFFICE_1, "name": "mac-office-1b" })).await;
        update(&pool, PVE_01, json!({ "siteId": OFFICE_2 })).await;
        let relay = devices::wake_relay(&pool, site(OFFICE_1)).await.unwrap();
        assert_eq!(relay.map(|device| device.id), Some(device(MAC_OFFICE_1)));
    }
}
