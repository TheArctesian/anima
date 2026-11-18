//! The operational keys through K1 (R-147, K-E5 Section 2.2): a value goes in and never comes
//! out, an API key reads the metadata and changes nothing, and a change says "recorded".

use chrono::TimeDelta;
use serde_json::{Value, json};
use sqlx::PgPool;

use crate::hub::{Hub, error, set_operator, start};

const VALUE: &str = "sk-live-k1-0perational-Value";

const FIELDS: &str = "id name provider purpose instructions providerLink rotationDays expiresAt \
    due activeVersion lastRotatedAt lastRotatedBy createdAt createdBy \
    versions { number state createdAt createdBy endedAt rotatedAtProvider } \
    consumers { deviceId workload grantedAt grantedBy heldVersion heldSeenAt fetchedVersion fetchedAt }";

const PVE_01: &str = "0190a000-0000-7000-8002-000000000001";

fn create_document() -> String {
    format!(
        "mutation($input: CreateOperationalKeyInput!) {{ createOperationalKey(input: $input) {{ {FIELDS} }} }}"
    )
}

fn input(name: &str) -> Value {
    json!({ "name": name, "provider": "Cloudflare", "purpose": "DNS updates", "value": VALUE,
            "providerLink": "https://dash.cloudflare.com", "rotationDays": 90 })
}

/// An API key of `scope` made by the operator signed in with `login`.
async fn api_key(hub: &Hub, login: &str, scope: &str) -> String {
    let query = "mutation($input: ApiKeyInput!) { createApiKey(input: $input) { secret } }";
    let input = json!({ "name": format!("script-{scope}"), "scope": scope, "madeFor": "USER",
                        "expiresAt": start() + TimeDelta::days(30) });
    let made = hub
        .post(Some(login), query, json!({ "input": input }))
        .await;
    made["data"]["createApiKey"]["secret"]
        .as_str()
        .unwrap_or_else(|| panic!("{made}"))
        .to_owned()
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn r_147_a_value_goes_in_and_no_answer_holds_it(pool: PgPool) {
    set_operator(&pool).await;
    let hub = Hub::new(&pool, start());
    let login = hub.token().await;
    let made = hub
        .post(
            Some(&login),
            &create_document(),
            json!({ "input": input("cloudflare.dns") }),
        )
        .await;
    let key = &made["data"]["createOperationalKey"];
    assert_eq!(key["activeVersion"], 1, "{made}");
    assert_eq!(key["due"], "CURRENT");
    let grant = "mutation { grantOperationalKey(name: \"cloudflare.dns\", device: \"0190a000-0000-7000-8002-000000000001\", workload: \"ddns\") { name } }";
    hub.post(Some(&login), grant, json!({})).await;
    let rotate = format!(
        "mutation {{ rotateOperationalKey(name: \"cloudflare.dns\", value: \"{VALUE}-2\", rotatedAtProvider: false) {{ endedVersion stillHeldBy {{ deviceId }} key {{ {FIELDS} }} }} }}"
    );
    let rotated = hub.post(Some(&login), &rotate, json!({})).await;
    assert_eq!(
        rotated["data"]["rotateOperationalKey"]["endedVersion"], 1,
        "{rotated}"
    );

    let list = format!("{{ operationalKeys {{ {FIELDS} }} }}");
    let listed = hub.post(Some(&login), &list, json!({})).await;
    let consumers = &listed["data"]["operationalKeys"][0]["consumers"];
    assert_eq!(consumers[0]["deviceId"], PVE_01, "{listed}");
    for answer in [&made, &rotated, &listed] {
        assert!(!answer.to_string().contains(VALUE), "{answer}");
    }
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn r_147_an_api_key_reads_the_metadata_and_changes_nothing(pool: PgPool) {
    set_operator(&pool).await;
    let hub = Hub::new(&pool, start());
    let login = hub.token().await;
    hub.post(
        Some(&login),
        &create_document(),
        json!({ "input": input("cloudflare.dns") }),
    )
    .await;
    let writer = api_key(&hub, &login, "WRITE").await;
    let reader = api_key(&hub, &login, "READ").await;

    let read = hub
        .post(
            Some(&reader),
            "{ operationalKeys { name activeVersion } }",
            json!({}),
        )
        .await;
    assert_eq!(
        read["data"]["operationalKeys"][0]["name"], "cloudflare.dns",
        "{read}"
    );

    let changes = [
        "mutation { rotateOperationalKey(name: \"cloudflare.dns\", value: \"x\", rotatedAtProvider: true) { endedVersion } }",
        "mutation { grantOperationalKey(name: \"cloudflare.dns\", device: \"0190a000-0000-7000-8002-000000000001\", workload: \"ddns\") { name } }",
        "mutation { revokeOperationalKey(name: \"cloudflare.dns\") { endedVersion } }",
        "mutation { removeOperationalKey(name: \"cloudflare.dns\") { endedVersion } }",
    ];
    for change in changes {
        let refused = hub.post(Some(&writer), change, json!({})).await;
        assert_eq!(error(&refused).1, "FORBIDDEN", "{change}: {refused}");
    }
    let made = hub
        .post(
            Some(&writer),
            &create_document(),
            json!({ "input": input("other") }),
        )
        .await;
    assert_eq!(error(&made).1, "FORBIDDEN", "{made}");
    let versions: i64 =
        sqlx::query_scalar("SELECT count(*) FROM credentials.operational_key_versions")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(versions, 1, "nothing changed");
}

#[sqlx::test(migrations = "./migrations")]
async fn r_147_a_rotation_the_provider_did_not_see_is_recorded_not_rotated(pool: PgPool) {
    set_operator(&pool).await;
    let hub = Hub::new(&pool, start());
    let login = hub.token().await;
    hub.post(
        Some(&login),
        &create_document(),
        json!({ "input": input("cloudflare.dns") }),
    )
    .await;
    let rotate = "mutation { rotateOperationalKey(name: \"cloudflare.dns\", value: \"next\", rotatedAtProvider: false) { endedVersion } }";
    hub.post(Some(&login), rotate, json!({})).await;

    let message: String =
        sqlx::query_scalar("SELECT message FROM events.event_log WHERE message LIKE '%version 2%'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(message.contains("recorded"), "{message}");
    assert!(message.contains("not rotated at the provider"), "{message}");
}
