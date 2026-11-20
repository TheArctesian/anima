//! The sealed records through the hub API (K-E5 Section 3; VLT-001, VLT-002, VLT-004, VLT-006;
//! PH-E-002): the hub stores what a client sealed and cannot read it.

use base64::{Engine as _, engine::general_purpose::STANDARD};
use chrono::TimeDelta;
use serde_json::{Value, json};
use sqlx::PgPool;

use anima_hub::fleet::events::process_log;

use crate::{
    hub::{Hub, error, set_operator, start},
    logs::Capture,
};

const RECORD: &str = "id owner extension sealed sealedIndex keyId createdAt updatedAt";
const STORE: &str = "mutation($i: SealedRecordInput!) { storeSealedRecord(input: $i) \
                     { id owner extension sealed sealedIndex keyId createdAt updatedAt } }";
const OWNER: &str = "passport:0190a000-0000-7000-8000-00000000aaaa";
/// A value that a client would seal. The test seals it as a client does, with a key that the
/// hub never gets, and then looks for it in each place that the hub writes.
const PLAIN: &str = "P4551-secret-passport-number";

async fn signed_in(pool: &PgPool) -> (Hub, String) {
    set_operator(pool).await;
    let hub = Hub::new(pool, start());
    let token = hub.token().await;
    (hub, token)
}

/// What a client does with its vault key: here, each byte of the value with one byte of a key
/// stream. The test needs only that the hub gets no plain byte sequence of the value.
fn sealed_by_a_client(plain: &str) -> Vec<u8> {
    plain
        .bytes()
        .enumerate()
        .map(|(index, byte)| byte ^ u8::try_from((index * 37 + 101) % 251).unwrap())
        .collect()
}

fn input(sealed: &[u8]) -> Value {
    json!({ "owner": OWNER, "extension": "documents", "sealed": STANDARD.encode(sealed),
            "sealedIndex": STANDARD.encode(b"index-bytes"), "keyId": STANDARD.encode([7_u8; 32]) })
}

#[sqlx::test(migrations = "./migrations")]
async fn vlt_001_a_sealed_record_is_stored_replaced_read_and_deleted_as_opaque_bytes(pool: PgPool) {
    let (hub, token) = signed_in(&pool).await;
    let sealed = sealed_by_a_client(PLAIN);
    let made = hub
        .post(Some(&token), STORE, json!({ "i": input(&sealed) }))
        .await;
    let record = &made["data"]["storeSealedRecord"];
    assert_eq!(record["owner"], OWNER, "{made}");
    assert_eq!(record["sealed"], STANDARD.encode(&sealed));
    assert_eq!(record["keyId"], STANDARD.encode([7_u8; 32]));

    let again = hub
        .post(Some(&token), STORE, json!({ "i": input(b"new bytes") }))
        .await;
    assert_eq!(again["data"]["storeSealedRecord"]["id"], record["id"]);
    let read = hub
        .post(
            Some(&token),
            &format!("query($o: String!) {{ sealedRecord(owner: $o) {{ {RECORD} }} }}"),
            json!({ "o": OWNER }),
        )
        .await;
    assert_eq!(
        read["data"]["sealedRecord"]["sealed"],
        STANDARD.encode(b"new bytes"),
        "{read}"
    );
    let actions: Vec<String> = sqlx::query_scalar(
        "select action from events.audit_log where target_type = 'sealed_record' order by at, id",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(actions, ["create", "update"]);

    let delete = "mutation($o: String!) { deleteSealedRecord(owner: $o) }";
    let deleted = hub.post(Some(&token), delete, json!({ "o": OWNER })).await;
    assert_eq!(deleted["data"]["deleteSealedRecord"], true, "{deleted}");
    let twice = hub.post(Some(&token), delete, json!({ "o": OWNER })).await;
    assert_eq!(twice["data"]["deleteSealedRecord"], false);
    let gone = hub
        .post(
            Some(&token),
            "query($o: String!) { sealedRecord(owner: $o) { id } }",
            json!({ "o": OWNER }),
        )
        .await;
    assert!(gone["data"]["sealedRecord"].is_null(), "{gone}");
}

#[sqlx::test(migrations = "./migrations")]
async fn ph_e_002_the_hub_holds_no_plain_byte_of_a_value_that_a_client_sealed(pool: PgPool) {
    let capture = Capture::default();
    let writer = capture.clone();
    let subscriber = process_log::subscriber("trace", move || writer.clone()).unwrap();
    let _default = tracing::subscriber::set_default(subscriber);
    tracing::callsite::rebuild_interest_cache();
    let (hub, token) = signed_in(&pool).await;
    let sealed = sealed_by_a_client(PLAIN);
    let made = hub
        .post(Some(&token), STORE, json!({ "i": input(&sealed) }))
        .await;
    assert!(made["errors"].is_null(), "{made}");
    assert!(!capture.text().is_empty(), "the log output is read");

    // Each text, JSON and bytea column of each table of the hub.
    let columns: Vec<(String, String, String, String)> = sqlx::query_as(
        "select c.table_schema, c.table_name, c.column_name, c.data_type \
         from information_schema.columns c join pg_tables t \
         on t.schemaname = c.table_schema and t.tablename = c.table_name \
         where c.table_schema not in ('pg_catalog', 'information_schema') \
         and c.data_type in ('text', 'bytea', 'jsonb', 'json')",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert!(
        columns
            .iter()
            .any(|(schema, _, column, _)| schema == "sealed_records" && column == "sealed"),
        "the search reads the table of the sealed records"
    );
    assert!(columns.len() > 100, "{} columns", columns.len());
    for (schema, table, column, data_type) in columns {
        let place = format!("\"{schema}\".\"{table}\"");
        let query = if data_type == "bytea" {
            format!("select count(*) from {place} where position($1 in \"{column}\") > 0")
        } else {
            format!(
                "select count(*) from {place} \
                 where position(convert_from($1, 'UTF8') in \"{column}\"::text) > 0"
            )
        };
        let found: i64 = sqlx::query_scalar(&query)
            .bind(PLAIN.as_bytes())
            .fetch_one(&pool)
            .await
            .unwrap_or_else(|error| panic!("{schema}.{table}.{column}: {error}"));
        assert_eq!(found, 0, "{schema}.{table}.{column} holds the value");
    }
    // The stored bytes are the bytes of the client, not changed.
    let stored: Vec<u8> = sqlx::query_scalar("select sealed from sealed_records.record")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(stored, sealed);
    let logged = capture.text();
    assert!(!logged.contains(PLAIN), "the log output holds the value");
    assert!(
        !logged.contains(&STANDARD.encode(&sealed)),
        "the log output holds the sealed bytes"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn vlt_001_input_that_is_not_valid_names_each_field_and_quotes_nothing(pool: PgPool) {
    let (hub, token) = signed_in(&pool).await;
    let bad = json!({ "owner": "no colon", "extension": "Documents", "sealed": "not base64 !",
                      "sealedIndex": "", "keyId": STANDARD.encode([7_u8; 31]) });
    let refused = hub.post(Some(&token), STORE, json!({ "i": bad })).await;
    assert_eq!(error(&refused).1, "VALIDATION", "{refused}");
    let fields: Vec<&str> = refused["errors"][0]["extensions"]["fields"]
        .as_array()
        .unwrap()
        .iter()
        .map(|field| field["field"].as_str().unwrap())
        .collect();
    assert_eq!(
        fields,
        ["owner", "extension", "sealed", "sealedIndex", "keyId"]
    );
    assert!(!refused.to_string().contains("not base64"), "{refused}");
    let big = STANDARD.encode(vec![1_u8; 4097]);
    let offer = hub
        .post(
            Some(&token),
            "mutation($s: String!) { offerVaultKey(sealed: $s) }",
            json!({ "s": big }),
        )
        .await;
    assert_eq!(error(&offer).1, "VALIDATION");
    let signed_out = hub.post(None, STORE, json!({ "i": input(b"bytes") })).await;
    assert_eq!(error(&signed_out).1, "UNAUTHENTICATED");
}

#[sqlx::test(migrations = "./migrations")]
async fn vlt_002_a_vault_key_offer_is_read_one_time_and_not_after_5_minutes(pool: PgPool) {
    let (hub, token) = signed_in(&pool).await;
    let offer = "mutation($s: String!) { offerVaultKey(sealed: $s) }";
    let read = "query($id: ID!) { vaultKeyOffer(id: $id) }";
    let sealed = STANDARD.encode(b"a key sealed to an exchange");
    let made = hub.post(Some(&token), offer, json!({ "s": sealed })).await;
    let id = made["data"]["offerVaultKey"].as_str().unwrap().to_owned();

    let first = hub.post(Some(&token), read, json!({ "id": id })).await;
    assert_eq!(first["data"]["vaultKeyOffer"], sealed, "{first}");
    let second = hub.post(Some(&token), read, json!({ "id": id })).await;
    assert!(second["data"]["vaultKeyOffer"].is_null(), "one read");

    let old = hub.post(Some(&token), offer, json!({ "s": sealed })).await;
    let old_id = old["data"]["offerVaultKey"].as_str().unwrap().to_owned();
    sqlx::query("update sealed_records.key_offer set expires_at = $1")
        .bind(start() - TimeDelta::seconds(1))
        .execute(&pool)
        .await
        .unwrap();
    let late = hub.post(Some(&token), read, json!({ "id": old_id })).await;
    assert!(late["data"]["vaultKeyOffer"].is_null(), "{late}");
    let unknown = hub
        .post(Some(&token), read, json!({ "id": "not an id" }))
        .await;
    assert!(unknown["data"]["vaultKeyOffer"].is_null());
    let entry: (String, Value) = sqlx::query_as(
        "select action, change from events.audit_log where target_type = 'vault_key_offer' \
         order by at limit 1",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(entry.0, "create");
    assert!(!entry.1.to_string().contains(&sealed), "{}", entry.1);
}
