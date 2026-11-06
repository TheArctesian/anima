//! `setRdpCredential`, `clearRdpCredential`, `setVncCredential` and `clearVncCredential`
//! (DEV-011, SEC-003, SEC-005, LOG-005, LOG-007).

use anima_hub::{
    credentials::{PasswordKind, Secret, load_password, store_password},
    events::Signal,
    ids::DeviceId,
};
use async_graphql::Request;
use serde_json::{Value, json};
use sqlx::PgPool;

use super::PVE_01;
use crate::devices::{
    DOCKER_HOST,
    credential::store_password as store_ssh_password,
    harness::{
        error_json, hardened_schema, schema_for, schema_with_bus, signed_in, validation_fields,
        vault, with_variables,
    },
    leak::{assert_absent, captured},
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

const UNKNOWN: &str = "0190a000-0000-7000-8002-0000000000ff";
const PASSWORD: &str = "rdp-or-vnc-hunter2-battery";

/// One protocol's mutations and what its audit entries and messages call it.
struct Protocol {
    set: &'static str,
    clear: &'static str,
    audit: &'static str,
    label: &'static str,
    kind: PasswordKind,
}

const RDP: Protocol = Protocol {
    set: "setRdpCredential",
    clear: "clearRdpCredential",
    audit: "rdp",
    label: "RDP",
    kind: PasswordKind::Rdp,
};

const VNC: Protocol = Protocol {
    set: "setVncCredential",
    clear: "clearVncCredential",
    audit: "vnc",
    label: "VNC",
    kind: PasswordKind::Vnc,
};

impl Protocol {
    fn set_query(&self) -> String {
        format!(
            "mutation($id: ID!, $credential: PasswordCredentialInput!) {{
                {}(deviceId: $id, credential: $credential) {{ enabled port credentialStored }} }}",
            self.set
        )
    }

    fn clear_query(&self) -> String {
        format!(
            "mutation($id: ID!) {{ {}(deviceId: $id) {{ enabled port credentialStored }} }}",
            self.clear
        )
    }
}

fn device(text: &str) -> DeviceId {
    text.parse().unwrap()
}

/// Stores a password for the device directly through `credentials`.
pub async fn store_pw(pool: &PgPool, id: &str, kind: PasswordKind, password: &str) {
    let mut conn = pool.acquire().await.unwrap();
    let secret = Secret::from(password.to_owned());
    store_password(&mut conn, &vault(), device(id), kind, secret)
        .await
        .unwrap();
}

async fn set(
    pool: &PgPool,
    protocol: &Protocol,
    id: &str,
    password: &str,
) -> async_graphql::Response {
    let variables = json!({ "id": id, "credential": { "password": password } });
    with_variables(&schema_for(pool), &protocol.set_query(), variables).await
}

async fn clear(pool: &PgPool, protocol: &Protocol, id: &str) -> async_graphql::Response {
    with_variables(
        &schema_for(pool),
        &protocol.clear_query(),
        json!({ "id": id }),
    )
    .await
}

fn ok(response: async_graphql::Response) -> Value {
    assert!(response.errors.is_empty(), "{:?}", response.errors);
    response.data.into_json().unwrap()
}

/// Whether the SSH, RDP and VNC secrets are stored, read through the loaders.
async fn stored(pool: &PgPool, id: &str) -> (bool, bool, bool) {
    let query = "query($id: ID!) { device(id: $id) {
        ssh { credentialStored } rdp { credentialStored } vnc { credentialStored } } }";
    let data = ok(with_variables(&schema_for(pool), query, json!({ "id": id })).await);
    let flag = |field: &str| data["device"][field]["credentialStored"].as_bool().unwrap();
    (flag("ssh"), flag("rdp"), flag("vnc"))
}

async fn audits(pool: &PgPool) -> Vec<Value> {
    sqlx::query_scalar("SELECT to_jsonb(a) FROM events.audit_log a ORDER BY at, id")
        .fetch_all(pool)
        .await
        .unwrap()
}

async fn event_messages(pool: &PgPool) -> Vec<String> {
    sqlx::query_scalar("SELECT message FROM events.event_log ORDER BY at, id")
        .fetch_all(pool)
        .await
        .unwrap()
}

async fn secret_rows(pool: &PgPool) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM credentials.secrets")
        .fetch_one(pool)
        .await
        .unwrap()
}

fixture_test! {
    async fn dev_011_set_stores_the_password_for_its_protocol_only(pool: PgPool) {
        for (protocol, expected) in [(&RDP, (false, true, false)), (&VNC, (false, true, true))] {
            let data = ok(set(&pool, protocol, PVE_01, PASSWORD).await);
            assert_eq!(data[protocol.set]["credentialStored"], true);
            assert_eq!(stored(&pool, PVE_01).await, expected, "{}", protocol.set);
            let opened = load_password(&pool, &vault(), device(PVE_01), protocol.kind)
                .await
                .unwrap();
            assert!(opened.is_some_and(|p| p.expose() == PASSWORD.as_bytes()));
        }
        assert_eq!(stored(&pool, DOCKER_HOST).await, (false, false, false));
    }

    async fn dev_011_clear_removes_only_its_protocols_password(pool: PgPool) {
        store_ssh_password(&pool, PVE_01, "an-ssh-password").await;
        store_pw(&pool, PVE_01, PasswordKind::Rdp, PASSWORD).await;
        store_pw(&pool, PVE_01, PasswordKind::Vnc, PASSWORD).await;
        let data = ok(clear(&pool, &RDP, PVE_01).await);
        assert_eq!(data["clearRdpCredential"]["credentialStored"], false);
        assert_eq!(stored(&pool, PVE_01).await, (true, false, true));
        let data = ok(clear(&pool, &VNC, PVE_01).await);
        assert_eq!(data["clearVncCredential"]["credentialStored"], false);
        assert_eq!(stored(&pool, PVE_01).await, (true, false, false));
    }

    async fn dev_011_ssh_set_and_clear_leave_the_rdp_and_vnc_passwords(pool: PgPool) {
        ok(set(&pool, &RDP, PVE_01, PASSWORD).await);
        ok(set(&pool, &VNC, PVE_01, PASSWORD).await);
        store_ssh_password(&pool, PVE_01, "an-ssh-password").await;
        let clear_ssh = "mutation($id: ID!) { clearSshCredential(deviceId: $id) { credentialStored } }";
        ok(with_variables(&schema_for(&pool), clear_ssh, json!({ "id": PVE_01 })).await);
        assert_eq!(stored(&pool, PVE_01).await, (false, true, true));
    }

    async fn log_005_set_and_clear_audit_the_protocol_and_a_second_clear_writes_nothing(pool: PgPool) {
        for protocol in [&RDP, &VNC] {
            let before = audits(&pool).await.len();
            ok(set(&pool, protocol, PVE_01, PASSWORD).await);
            ok(clear(&pool, protocol, PVE_01).await);
            ok(clear(&pool, protocol, PVE_01).await);
            let audits = audits(&pool).await;
            assert_eq!(audits.len(), before + 2, "{}", protocol.set);
            let (set_entry, clear_entry) = (&audits[before], &audits[before + 1]);
            assert_eq!(set_entry["change"], json!({ "protocol": protocol.audit, "kind": "password" }));
            assert_eq!(clear_entry["change"],
                json!({ "protocol": protocol.audit, "kind": "password", "cleared": true }));
            for (entry, action) in [(set_entry, "update"), (clear_entry, "delete")] {
                assert_eq!(entry["action"], action);
                assert_eq!(entry["target_type"], "credential");
                assert_eq!(entry["target_id"], PVE_01);
            }
        }
        assert_eq!(event_messages(&pool).await, [
            "Device pve-01 RDP password set.",
            "Device pve-01 RDP password cleared.",
            "Device pve-01 VNC password set.",
            "Device pve-01 VNC password cleared.",
        ]);
    }

    async fn log_007_a_set_password_is_in_no_response_audit_event_bus_message_or_log(pool: PgPool) {
        for protocol in [&RDP, &VNC] {
            let secret = format!("{}-leak-check-secret", protocol.audit);
            let (schema, bus) = schema_with_bus(&pool);
            let mut receiver = bus.subscribe();
            let variables = json!({ "id": PVE_01, "credential": { "password": secret } });
            let query = protocol.set_query();
            let (log, response) = captured(|| with_variables(&schema, &query, variables)).await;
            let response = serde_json::to_string(&ok(response)).unwrap();
            let Signal::Event(event) = receiver.try_recv().unwrap() else { panic!("an event") };
            assert_eq!(event.message, format!("Device pve-01 {} password set.", protocol.label));
            let audits = serde_json::to_string(&audits(&pool).await).unwrap();
            let events = event_messages(&pool).await.join("\n");
            assert_absent(&secret, &[&response, &format!("{event:?}"), &audits, &events, &log]);
        }
    }

    async fn dev_011_an_unknown_device_is_not_found_and_a_bad_id_is_a_field_error(pool: PgPool) {
        for protocol in [&RDP, &VNC] {
            for response in [set(&pool, protocol, UNKNOWN, PASSWORD).await, clear(&pool, protocol, UNKNOWN).await] {
                let error = error_json(&response);
                assert_eq!(error["extensions"]["code"], "NOT_FOUND", "{error}");
            }
            for response in [set(&pool, protocol, "pve-01", PASSWORD).await, clear(&pool, protocol, "pve-01").await] {
                assert_eq!(validation_fields(&error_json(&response)), ["deviceId"]);
            }
            let both = error_json(&set(&pool, protocol, "pve-01", " ").await);
            assert_eq!(validation_fields(&both), ["deviceId", "credential.password"]);
        }
        assert_eq!((secret_rows(&pool).await, audits(&pool).await.len()), (0, 0));
    }

    async fn log_007_a_blank_or_oversized_password_is_a_field_error_that_quotes_nothing(pool: PgPool) {
        let oversized = "q".repeat(1025);
        for protocol in [&RDP, &VNC] {
            for password in ["", " \t ", oversized.as_str()] {
                let error = error_json(&set(&pool, protocol, PVE_01, password).await);
                assert_eq!(validation_fields(&error), ["credential.password"]);
                let text = error.to_string();
                assert!(!text.contains("qqqqqqqq") && !text.contains("1025"), "{text}");
            }
        }
        assert_eq!((secret_rows(&pool).await, audits(&pool).await.len()), (0, 0));
    }

    async fn log_007_an_argument_of_the_wrong_shape_is_refused_without_quoting_it(pool: PgPool) {
        let schema = hardened_schema(&pool);
        for protocol in [&RDP, &VNC] {
            let shapes = [
                r#"{ password: "shape-extra-secret", extra: "x" }"#,
                r#"{ passwrd: "shape-misspelt-secret" }"#,
                r#""shape-bare-secret""#,
                r#"["shape-list-secret"]"#,
                "{ password: 918273645 }",
            ];
            for credential in shapes {
                let query = format!(
                    r#"mutation {{ {}(deviceId: "{PVE_01}", credential: {credential}) {{ credentialStored }} }}"#,
                    protocol.set
                );
                let (log, response) = captured(|| schema.execute(signed_in(&query))).await;
                assert!(!response.errors.is_empty(), "{credential} was accepted");
                let body = serde_json::to_string(&response).unwrap();
                let secret = credential.split('"').nth(1).unwrap_or("918273645");
                assert_absent(secret, &[&body, &format!("{:?}", response.errors), &log]);
            }
            let bare = json!({ "id": PVE_01, "credential": "variable-bare-secret" });
            let request = Request::new(protocol.set_query()).variables(async_graphql::Variables::from_json(bare));
            let (log, response) = captured(|| schema.execute(request)).await;
            let error = error_json(&response);
            assert_eq!(error["extensions"]["fields"][0]["field"], "credential", "{error}");
            let body = serde_json::to_string(&response).unwrap();
            assert_absent("variable-bare-secret", &[&body, &log]);
        }
        assert_eq!(secret_rows(&pool).await, 0);
    }
}
