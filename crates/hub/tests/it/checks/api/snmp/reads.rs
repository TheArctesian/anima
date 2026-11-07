//! An SNMP check created, read and updated through the API, with every place a secret could
//! leak searched for the planted ones (SEC-003, LOG-007).

use std::fmt::Write as _;

use anima_hub::{checks, events::Signal};
use serde_json::json;
use sqlx::PgPool;

use super::{AUTH, PRIVACY, READ, SECRETS, v3};
use crate::checks::api::{
    harness::{
        CREATE, PVE_01, UPDATE, assert_absent, captured, create, data_with, input,
        schema_with_bus, signed_in, vault, with_variables,
    },
    writes::rows,
};

macro_rules! fixture_test {
    ($(async fn $name:ident($pool:ident: PgPool) $body:block)+) => {$(
        #[sqlx::test(
            migrations = "./migrations",
            fixtures(path = "../../../../fixtures", scripts("devices"))
        )]
        async fn $name($pool: PgPool) $body
    )+};
}

fixture_test! {
    async fn d_5_a_v2c_check_is_created_with_the_contract_defaults(pool: PgPool) {
        let (schema, _) = schema_with_bus(&pool);
        let config = json!({ "snmp": { "oid": "1.3.6.1.2.1.1.3.0", "community": super::COMMUNITY } });
        let id = create(&schema, input(PVE_01, "uptime", config)).await;
        let read = data_with(&schema, READ, json!({ "id": id })).await;
        assert_eq!(read["check"]["type"], "SNMP");
        assert_eq!(read["check"]["config"], json!({
            "__typename": "SnmpConfig", "port": 161, "version": "V2C",
            "oid": "1.3.6.1.2.1.1.3.0", "expected": null, "username": null, "timeoutMs": 5000,
        }));
        let (kind, stored): (String, serde_json::Value) =
            sqlx::query_as("SELECT type, config FROM checks.checks WHERE id = $1::uuid")
                .bind(&id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(kind, "snmp");
        assert_eq!(stored, json!({ "port": 161, "version": "V2C", "oid": "1.3.6.1.2.1.1.3.0", "timeoutMs": 5000 }));
    }

    async fn log_007_no_snmp_secret_reaches_a_response_row_event_broadcast_or_log(pool: PgPool) {
        let (schema, bus) = schema_with_bus(&pool);
        let mut receiver = bus.subscribe();
        let vars = json!({ "input": input(PVE_01, "snmp", v3("monitor")) });
        let (create_log, created) = captured(|| schema.execute(signed_in(CREATE).variables(
            async_graphql::Variables::from_json(vars),
        ))).await;
        assert_eq!(created.errors.len(), 0, "{:?}", created.errors);
        let id = created.data.clone().into_json().unwrap()["createCheck"]["id"].as_str().unwrap().to_owned();
        let mut changed = v3("monitor");
        changed["snmp"]["expected"] = json!("nas-01");
        let update = json!({ "id": id, "input": { "config": changed } });
        let (update_log, updated) = captured(|| with_variables(&schema, UPDATE, update)).await;
        assert!(updated.errors.is_empty(), "{:?}", updated.errors);
        let read = data_with(&schema, READ, json!({ "id": id })).await;
        assert_eq!(read["check"]["config"]["username"], "monitor");
        assert_eq!(read["check"]["config"]["expected"], "nas-01");

        let mut broadcast = String::new();
        while let Ok(signal) = receiver.try_recv() {
            if let Signal::Event(event) = signal {
                write!(broadcast, "{event:?}").unwrap();
            }
        }
        let row: String = sqlx::query_scalar("SELECT row_to_json(c)::text FROM checks.checks c")
            .fetch_one(&pool)
            .await
            .unwrap();
        let check = checks::get(&pool, id.parse().unwrap()).await.unwrap();
        let places = [
            serde_json::to_string(&created).unwrap(),
            serde_json::to_string(&updated).unwrap(),
            read.to_string(),
            serde_json::to_string(&rows(&pool, "audit_log").await).unwrap(),
            serde_json::to_string(&rows(&pool, "event_log").await).unwrap(),
            row,
            format!("{check:?}"),
            broadcast,
            create_log,
            update_log,
        ];
        let places: Vec<&str> = places.iter().map(String::as_str).collect();
        for secret in SECRETS {
            assert_absent(secret, &places);
        }
        let audits = rows(&pool, "audit_log").await;
        assert_eq!(audits[0]["change"]["sealed"], "[set]");
        assert_eq!(audits[0]["change"]["config"]["username"], "monitor");
    }

    async fn sec_005_an_update_without_secrets_keeps_them_and_audits_none(pool: PgPool) {
        let (schema, _) = schema_with_bus(&pool);
        let id = create(&schema, input(PVE_01, "snmp", v3("monitor"))).await;
        let mut blank = v3("monitor");
        blank["snmp"]["authPassword"] = json!(null);
        blank["snmp"].as_object_mut().unwrap().remove("privacyPassword");
        let updated = with_variables(&schema, UPDATE, json!({ "id": id, "input": { "config": blank } })).await;
        assert!(updated.errors.is_empty(), "{:?}", updated.errors);
        let setup = checks::load_snmp_secrets(&pool, &vault(), id.parse().unwrap())
            .await
            .unwrap()
            .unwrap();
        let Some(checks::config::SnmpSecrets::User { auth, privacy }) = setup.secrets else {
            panic!("no v3 secrets");
        };
        assert_eq!((auth.expose(), privacy.expose()), (AUTH.as_bytes(), PRIVACY.as_bytes()));
        let audits = rows(&pool, "audit_log").await;
        assert!(audits[1]["change"].get("sealed").is_none(), "{}", audits[1]["change"]);
    }
}
