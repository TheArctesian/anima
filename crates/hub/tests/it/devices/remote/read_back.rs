//! The remote access fields of K1 `Device` read back through GraphQL, the access warnings
//! (RA-020), the batched RDP and VNC `credentialStored` (API-005) and the RDP server
//! certificate accepted for a device (SEC-007).

use std::sync::{Arc, atomic::Ordering};

use anima_hub::{
    clock::{Clock, FixedClock},
    credentials::PasswordKind,
    devices::graphql::{Mutation, Query, rdp_credential_loader, vnc_credential_loader},
    trust::graphql::server_certificate_loader,
};
use async_graphql::{EmptySubscription, Schema};
use chrono::{DateTime, Utc};
use serde_json::{Value, json};
use sqlx::PgPool;

use super::{CREATE, HOME, PVE_01, full_rdp, passwords::store_pw};
use crate::devices::{
    DOCKER_HOST,
    batching::{BATCH_WINDOW, SITES, counting_pool},
    graphql_read::add_bulk,
    harness::{data, now, schema_for, with_variables},
};

macro_rules! fixture_test {
    ($(async fn $name:ident($pool:ident: PgPool) $body:block)+) => {$(
        #[sqlx::test(
            migrations = "./migrations",
            fixtures(path = "../../../fixtures", scripts("devices"))
        )]
        async fn $name($pool: PgPool) $body
    )+};
}

const FIELDS: &str = "rdp { enabled port username domain security ignoreCertificate
    resizeMethod audioOut microphone sharedDrive credentialStored }
    vnc { enabled port username credentialStored }
    macAddress metricsSource transfer { upload download copy paste } accessWarnings";

async fn mutate(pool: &PgPool, query: &str, variables: Value) -> Value {
    let response = with_variables(&schema_for(pool), query, variables).await;
    assert!(response.errors.is_empty(), "{:?}", response.errors);
    response.data.into_json().unwrap()
}

async fn read(pool: &PgPool, id: &str) -> Value {
    let query = format!("query($id: ID!) {{ device(id: $id) {{ {FIELDS} }} }}");
    mutate(pool, &query, json!({ "id": id })).await["device"].take()
}

async fn update(pool: &PgPool, id: &str, input: Value) -> Value {
    let query = format!(
        "mutation($id: ID!, $input: UpdateDeviceInput!) {{ updateDevice(id: $id, input: $input) {{ {FIELDS} }} }}"
    );
    mutate(pool, &query, json!({ "id": id, "input": input })).await["updateDevice"].take()
}

fn warnings(device: &Value) -> Vec<&str> {
    device["accessWarnings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|warning| warning.as_str().unwrap())
        .collect()
}

fixture_test! {
    async fn ra_030_a_new_device_reads_back_the_k1_defaults(pool: PgPool) {
        let query = CREATE.replace("{ id }", &format!("{{ id {FIELDS} }}"));
        let input = json!({ "siteId": HOME, "name": "win-01", "host": "10.10.0.60" });
        let created = mutate(&pool, &query, json!({ "input": input })).await["createDevice"].take();
        let expected = json!({
            "rdp": { "enabled": false, "port": 3389, "username": "", "domain": null,
                "security": "NLA", "ignoreCertificate": false, "resizeMethod": "DISPLAY_UPDATE",
                "audioOut": true, "microphone": false, "sharedDrive": false,
                "credentialStored": false },
            "vnc": { "enabled": false, "port": 5900, "username": null, "credentialStored": false },
            "macAddress": null,
            "metricsSource": "NONE",
            "transfer": { "upload": true, "download": true, "copy": true, "paste": true },
            "accessWarnings": [],
        });
        let id = created["id"].as_str().unwrap().to_owned();
        let mut shown = created;
        shown.as_object_mut().unwrap().remove("id");
        assert_eq!(shown, expected);
        assert_eq!(read(&pool, &id).await, expected);
    }

    async fn ra_030_every_setting_sent_reads_back_from_the_update_and_a_later_query(pool: PgPool) {
        let changed = update(&pool, PVE_01, json!({
            "rdp": full_rdp(),
            "vnc": { "enabled": true, "port": 5901, "username": "viewer" },
            "macAddress": "AA-BB-0C-DD-EE-F2",
            "metricsSource": "SSH",
            "transfer": { "upload": false, "copy": false },
        })).await;
        let expected = json!({
            "rdp": { "enabled": true, "port": 3390, "username": "admin", "domain": "CORP",
                "security": "TLS", "ignoreCertificate": true, "resizeMethod": "RECONNECT",
                "audioOut": false, "microphone": true, "sharedDrive": true,
                "credentialStored": false },
            "vnc": { "enabled": true, "port": 5901, "username": "viewer", "credentialStored": false },
            "macAddress": "aa:bb:0c:dd:ee:f2",
            "metricsSource": "SSH",
            "transfer": { "upload": false, "download": true, "copy": false, "paste": true },
            "accessWarnings": ["IGNORED_CERTIFICATE", "UNENCRYPTED_VNC"],
        });
        assert_eq!(changed, expected);
        assert_eq!(read(&pool, PVE_01).await, expected);
    }

    async fn ra_020_warnings_follow_the_saved_settings_and_a_protocol_that_is_off_earns_none(pool: PgPool) {
        let rdp = |enabled: bool, ignore: bool, security: &str| json!({ "enabled": enabled,
            "username": "admin", "ignoreCertificate": ignore, "security": security });
        let vnc = |enabled: bool| json!({ "enabled": enabled });
        let cases = [
            (rdp(true, true, "RDP"), vnc(true),
             vec!["IGNORED_CERTIFICATE", "RDP_SECURITY", "UNENCRYPTED_VNC"]),
            (rdp(true, false, "RDP"), vnc(false), vec!["RDP_SECURITY"]),
            (rdp(true, true, "ANY"), vnc(false), vec!["IGNORED_CERTIFICATE"]),
            (rdp(true, false, "TLS"), vnc(true), vec!["UNENCRYPTED_VNC"]),
            (rdp(true, false, "NLA"), vnc(false), vec![]),
            (rdp(false, true, "RDP"), vnc(false), vec![]),
        ];
        for (rdp, vnc, expected) in cases {
            let sent = format!("{rdp} {vnc}");
            update(&pool, PVE_01, json!({ "rdp": rdp, "vnc": vnc })).await;
            assert_eq!(warnings(&read(&pool, PVE_01).await), expected, "{sent}");
        }
    }

    async fn api_005_rdp_and_vnc_credential_stored_cost_a_constant_few_statements_for_13_or_163_devices(pool: PgPool) {
        let (counted, statements) = counting_pool(&pool).await;
        let schema = Schema::build(Query, Mutation, EmptySubscription)
            .data(counted.clone())
            .data(Arc::new(FixedClock(now())) as Arc<dyn Clock>)
            .data(rdp_credential_loader(counted.clone()).delay(BATCH_WINDOW))
            .data(vnc_credential_loader(counted.clone()).delay(BATCH_WINDOW))
            .finish();
        store_pw(&pool, PVE_01, PasswordKind::Rdp, "an-rdp-password").await;
        store_pw(&pool, DOCKER_HOST, PasswordKind::Vnc, "a-vnc-password").await;
        let query = "{ devices(first: 200) { edges { node { name
            rdp { credentialStored } vnc { credentialStored } } } } }";

        let few = data(&schema, query).await;
        let few_statements = statements.swap(0, Ordering::SeqCst);
        add_bulk(&pool, SITES, 150).await;
        let many = data(&schema, query).await;
        let many_statements = statements.load(Ordering::SeqCst);

        assert_eq!(stored(&few), (vec!["pve-01"], vec!["docker-host"], 13));
        assert_eq!(stored(&many), (vec!["pve-01"], vec!["docker-host"], 163));
        // The page, its count, one batch for RDP and one for VNC. One lookup per device costs
        // at least 26 statements on this data.
        assert!(
            few_statements <= 12 && many_statements <= 12,
            "{few_statements} statements for 13 devices, {many_statements} for 163"
        );
    }
}

/// A throwaway SHA-256 fingerprint for PVE-01's accepted RDP server certificate.
const CERTIFICATE: &str = "0a:1b:2c:3d:4e:5f:60:71:82:93:a4:b5:c6:d7:e8:f9:\
                           0a:1b:2c:3d:4e:5f:60:71:82:93:a4:b5:c6:d7:e8:f9";

/// Stores `CERTIFICATE` as PVE-01's accepted server certificate, accepted by a new operator.
async fn accept_certificate(pool: &PgPool) {
    sqlx::query(
        "with operator as (insert into access.operators (id, username, password_hash, created_at) \
             values (gen_random_uuid(), 'operator', '$argon2id$x', now()) returning id) \
         insert into trust.server_certificates (device_id, fingerprint_sha256, subject, \
             not_after, first_seen_at, accepted_by) \
         select $1::text::uuid, $2, 'CN=PVE-01', '2031-01-01T00:00:00Z', $3, id from operator",
    )
    .bind(PVE_01)
    .bind(CERTIFICATE)
    .bind(now())
    .execute(pool)
    .await
    .unwrap();
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn sec_007_rdp_settings_show_the_accepted_server_certificate_and_null_without_one(
    pool: PgPool,
) {
    let schema = Schema::build(Query, Mutation, EmptySubscription)
        .data(pool.clone())
        .data(Arc::new(FixedClock(now())) as Arc<dyn Clock>)
        .data(server_certificate_loader(pool.clone()))
        .finish();
    accept_certificate(&pool).await;
    let query = |id: &str| {
        format!(
            r#"{{ device(id: "{id}") {{ rdp {{ serverCertificate {{
                fingerprintSha256 subject notAfter firstSeenAt }} }} }} }}"#
        )
    };

    let accepted = data(&schema, &query(PVE_01)).await;
    let none = data(&schema, &query(DOCKER_HOST)).await;

    let shown = &accepted["device"]["rdp"]["serverCertificate"];
    let instant = |field: &str| {
        shown[field]
            .as_str()
            .unwrap()
            .parse::<DateTime<Utc>>()
            .unwrap()
    };
    assert_eq!(shown["fingerprintSha256"], CERTIFICATE);
    assert_eq!(shown["subject"], "CN=PVE-01");
    assert_eq!(
        instant("notAfter"),
        "2031-01-01T00:00:00Z".parse::<DateTime<Utc>>().unwrap()
    );
    assert_eq!(instant("firstSeenAt"), now());
    assert_eq!(none["device"]["rdp"]["serverCertificate"], Value::Null);
}

/// The devices with an RDP password, those with a VNC password, and how many there are.
fn stored(result: &Value) -> (Vec<&str>, Vec<&str>, usize) {
    let edges = result["devices"]["edges"].as_array().unwrap();
    let with = |protocol: &str| {
        edges
            .iter()
            .filter(|edge| edge["node"][protocol]["credentialStored"] == true)
            .map(|edge| edge["node"]["name"].as_str().unwrap())
            .collect()
    };
    (with("rdp"), with("vnc"), edges.len())
}
