//! The domain expiry check through the API and the schema (spec 6.4, CHK-009, D-29, K1
//! `DomainExpiryConfig`, `Check.domainDaysLeft`): create, read, the hourly interval floor, the
//! expiry date's column and its days left.

use chrono::TimeDelta;
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

use super::{
    PVE_01, PVE_01_PING,
    api::harness::{
        CREATE, UPDATE, create, data_with, error_json, input, now, schema_for, validation_fields,
        with_variables,
    },
    constraint_of, id,
};

macro_rules! fixture_test {
    ($(async fn $name:ident($pool:ident: PgPool) $body:block)+) => {$(
        #[sqlx::test(
            migrations = "./migrations",
            fixtures(path = "../../fixtures", scripts("devices"))
        )]
        async fn $name($pool: PgPool) $body
    )+};
}

const READ: &str = "query($id: ID!) { check(id: $id) { type intervalSeconds domainDaysLeft \
    certificateDaysLeft config { __typename ... on DomainExpiryConfig { domain warnDays timeoutMs } } } }";

/// A `createCheck` input for a domain expiry check every 12 hours, as the client proposes.
fn domain_input(name: &str, config: &Value) -> Value {
    let mut input = input(PVE_01, name, json!({ "domainExpiry": config }));
    input["intervalSeconds"] = json!(43_200);
    input
}

async fn set_not_after(pool: &PgPool, check: &str, offset: TimeDelta) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE checks.checks SET domain_not_after = $2 WHERE id = $1::text::uuid")
        .bind(check)
        .bind(now() + offset)
        .execute(pool)
        .await
        .map(|_| ())
}

async fn stored_not_after(pool: &PgPool, check: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    sqlx::query_scalar("SELECT domain_not_after FROM checks.checks WHERE id = $1::text::uuid")
        .bind(check)
        .fetch_one(pool)
        .await
        .unwrap()
}

fixture_test! {
    async fn d_29_a_domain_expiry_check_is_created_with_the_contract_defaults_and_read_back(pool: PgPool) {
        let schema = schema_for(&pool);
        let id = create(&schema, domain_input("example.net", &json!({ "domain": "Example.NET" }))).await;

        let read = data_with(&schema, READ, json!({ "id": id })).await;

        let check = &read["check"];
        assert_eq!(check["type"], "DOMAIN_EXPIRY");
        assert_eq!(check["intervalSeconds"], 43_200);
        assert_eq!(
            check["config"],
            json!({ "__typename": "DomainExpiryConfig", "domain": "example.net", "warnDays": 30, "timeoutMs": 10000 })
        );
        assert_eq!(check["domainDaysLeft"], Value::Null, "no RDAP reply yet");
        let (kind, stored): (String, Value) =
            sqlx::query_as("SELECT type, config FROM checks.checks WHERE id = $1::text::uuid")
                .bind(&id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(kind, "domain_expiry");
        assert_eq!(stored, json!({ "domain": "example.net", "warnDays": 30, "timeoutMs": 10000 }));
    }

    async fn d_29_domain_days_left_floors_and_goes_negative_after_expiry(pool: PgPool) {
        let schema = schema_for(&pool);
        let config = json!({ "domain": "example.net", "warnDays": 14, "timeoutMs": 5000 });
        let id = create(&schema, domain_input("example.net", &config)).await;
        let cases = [
            (TimeDelta::days(12) + TimeDelta::hours(1), 12),
            (TimeDelta::hours(23), 0),
            (-TimeDelta::hours(1), -1),
            (-TimeDelta::days(40), -40),
        ];
        for (offset, expected) in cases {
            set_not_after(&pool, &id, offset).await.unwrap();
            let read = data_with(&schema, READ, json!({ "id": id })).await;
            assert_eq!(read["check"]["domainDaysLeft"], json!(expected), "{offset}");
            assert_eq!(read["check"]["certificateDaysLeft"], Value::Null);
        }
        let read = data_with(&schema, READ, json!({ "id": PVE_01_PING })).await;
        assert_eq!(read["check"]["domainDaysLeft"], Value::Null, "an ICMP check has none");
    }

    async fn d_29_a_domain_expiry_check_runs_at_most_once_an_hour(pool: PgPool) {
        let schema = schema_for(&pool);
        let mut hourly = domain_input("hourly", &json!({ "domain": "example.net" }));
        hourly["intervalSeconds"] = json!(3600);
        let id = create(&schema, hourly).await;
        let mut too_often = domain_input("often", &json!({ "domain": "example.org" }));
        too_often["intervalSeconds"] = json!(3599);
        let defaulted = input(PVE_01, "default", json!({ "domainExpiry": { "domain": "example.com" } }));

        for refused in [too_often, defaulted] {
            let response = with_variables(&schema, CREATE, json!({ "input": refused })).await;
            assert_eq!(validation_fields(&error_json(&response)), ["intervalSeconds"]);
        }
        let update = json!({ "id": id, "input": { "intervalSeconds": 600 } });
        let response = with_variables(&schema, UPDATE, update).await;
        assert_eq!(validation_fields(&error_json(&response)), ["intervalSeconds"]);
    }

    async fn d_29_a_domain_that_is_not_a_registrable_name_is_a_field_error(pool: PgPool) {
        let schema = schema_for(&pool);
        for domain in ["https://example.net", "example.net/x", "example.net:443", "localhost", "10.0.0.1", "bücher.example"] {
            let refused = domain_input("bad", &json!({ "domain": domain }));
            let response = with_variables(&schema, CREATE, json!({ "input": refused })).await;
            let error = error_json(&response);
            assert_eq!(validation_fields(&error), ["domain"], "{domain}");
        }
        let refused = domain_input("bad", &json!({ "domain": "example.net", "warnDays": 0 }));
        let response = with_variables(&schema, CREATE, json!({ "input": refused })).await;
        assert_eq!(validation_fields(&error_json(&response)), ["warnDays"]);
    }

    async fn d_29_a_new_domain_drops_the_old_expiry_and_other_changes_keep_it(pool: PgPool) {
        let schema = schema_for(&pool);
        let id = create(&schema, domain_input("site", &json!({ "domain": "example.net" }))).await;
        set_not_after(&pool, &id, TimeDelta::days(12)).await.unwrap();

        let same = json!({ "id": id, "input": { "config": { "domainExpiry": { "domain": "example.net", "warnDays": 7 } } } });
        data_with(&schema, UPDATE, same).await;
        assert_eq!(stored_not_after(&pool, &id).await, Some(now() + TimeDelta::days(12)));
        data_with(&schema, UPDATE, json!({ "id": id, "input": { "name": "renamed" } })).await;
        assert_eq!(stored_not_after(&pool, &id).await, Some(now() + TimeDelta::days(12)));

        let other = json!({ "id": id, "input": { "config": { "domainExpiry": { "domain": "example.org" } } } });
        data_with(&schema, UPDATE, other).await;
        assert_eq!(stored_not_after(&pool, &id).await, None);
    }

    async fn chk_009_only_a_domain_expiry_check_holds_a_domain_expiry_date(pool: PgPool) {
        let refused = set_not_after(&pool, PVE_01_PING, TimeDelta::days(3)).await.unwrap_err();
        assert_eq!(constraint_of(&refused), Some("checks_domain_not_after_check"));

        let check = Uuid::now_v7();
        sqlx::query(
            "INSERT INTO checks.checks (id, device_id, type, name, interval_seconds, config) \
             VALUES ($1, $2, 'domain_expiry', 'raw', 3600, '{\"domain\": \"example.net\"}')",
        )
        .bind(check)
        .bind(id(PVE_01))
        .execute(&pool)
        .await
        .unwrap();
        set_not_after(&pool, &check.to_string(), TimeDelta::days(3)).await.unwrap();
        let too_often = sqlx::query("UPDATE checks.checks SET interval_seconds = 3599 WHERE id = $1")
            .bind(check)
            .execute(&pool)
            .await
            .unwrap_err();
        assert_eq!(constraint_of(&too_often), Some("checks_domain_expiry_interval_check"));
    }
}
