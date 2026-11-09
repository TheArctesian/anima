//! `shareSession` and `Session.shares` (RA-035, D-7, D-9, S-3, S-4, S-6, LOG-007): the link
//! holds the token only after `#`, the hub keeps only its hash, K1's defaults, the audit
//! entry `share.create` and its event, and the guard.

use async_graphql::{Request, Variables};
use chrono::{DateTime, TimeDelta, Utc};
use serde_json::{Value, json};
use sqlx::PgPool;

use super::{CLIENT, ORIGIN, SHARE, bench, error_of, events, share_audits};
use crate::sessions::{
    tests::fixture::{now, stored_text},
    tickets::hash_of,
};

fn at(value: &Value) -> DateTime<Utc> {
    value.as_str().unwrap().parse().unwrap()
}

/// The token of a share link: what follows `#`.
fn token_of(url: &str) -> &str {
    url.split_once('#').unwrap().1
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn d_7_ra_035_the_link_holds_the_token_only_after_the_fragment_and_the_hub_its_hash(
    pool: PgPool,
) {
    let bench = bench(&pool).await;
    let link = bench.shared().await;
    let url = link["url"].as_str().unwrap();

    let (before, token) = url.split_once('#').unwrap();
    assert_eq!(before, format!("{ORIGIN}/shared"));
    assert_eq!(token.len(), 43, "32 bytes, unpadded base64url");
    assert!(
        token
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_'),
        "{token}"
    );
    assert!(
        !url.contains('?'),
        "no query: the token never reaches a server"
    );
    let hash = hash_of(token).unwrap();
    assert!(
        bench.live.shares().hashes() == [hash],
        "only the token's SHA-256 is held"
    );

    let second = bench.shared().await;
    let second_token = token_of(second["url"].as_str().unwrap());
    assert_ne!(second_token, token, "each share has its own token");
    assert_eq!(bench.live.shares().hashes().len(), 2);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn d_9_share_input_takes_k1_s_defaults_for_absent_and_null_fields(pool: PgPool) {
    let bench = bench(&pool).await;
    let absent = bench.shared().await;
    let nulls = bench
        .share(json!({ "access": null, "expiresInMinutes": null, "maxViewers": null }))
        .await;
    assert!(nulls.errors.is_empty(), "{:?}", nulls.errors);
    let nulls = nulls.data.into_json().unwrap()["shareSession"].clone();

    for link in [absent, nulls] {
        let share = &link["share"];
        assert_eq!(share["sessionId"], bench.session.to_string());
        assert_eq!(share["access"], "VIEW");
        assert_eq!(share["maxViewers"], 3);
        assert_eq!(share["viewers"], 0);
        assert_eq!(at(&share["createdAt"]), now());
        assert_eq!(at(&share["expiresAt"]), now() + TimeDelta::minutes(60));
    }
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn d_9_share_input_is_taken_as_given_at_both_bounds(pool: PgPool) {
    let bench = bench(&pool).await;
    for (access, minutes, viewers) in [("CONTROL", 5, 10), ("VIEW", 1440, 1)] {
        let input = json!({ "access": access, "expiresInMinutes": minutes, "maxViewers": viewers });
        let response = bench.share(input).await;
        assert!(response.errors.is_empty(), "{:?}", response.errors);
        let share = response.data.into_json().unwrap()["shareSession"]["share"].clone();
        assert_eq!(share["access"], access);
        assert_eq!(share["maxViewers"], viewers);
        assert_eq!(at(&share["expiresAt"]), now() + TimeDelta::minutes(minutes));
    }
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn d_9_a_value_out_of_bounds_is_validation_on_its_field_and_shares_nothing(pool: PgPool) {
    let bench = bench(&pool).await;
    let response = bench
        .share(json!({ "expiresInMinutes": 1441, "maxViewers": 0 }))
        .await;
    let error = serde_json::to_value(&response.errors[0]).unwrap();
    assert_eq!(error["extensions"]["code"], "VALIDATION");
    let fields: Vec<&str> = error["extensions"]["fields"]
        .as_array()
        .unwrap()
        .iter()
        .map(|field| field["field"].as_str().unwrap())
        .collect();
    assert_eq!(fields, ["expiresInMinutes", "maxViewers"]);
    for input in [
        json!({ "expiresInMinutes": 4 }),
        json!({ "maxViewers": 11 }),
    ] {
        assert_eq!(error_of(&bench.share(input).await).0, "VALIDATION");
    }
    assert!(bench.live.shares().hashes().is_empty());
    assert_eq!(share_audits(&pool).await, []);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn ra_035_log_005_share_create_is_audited_with_an_info_event_and_no_token(pool: PgPool) {
    let bench = bench(&pool).await;
    let input = json!({ "access": "CONTROL", "expiresInMinutes": 5, "maxViewers": 10 });
    let response = bench.share(input).await;
    assert!(response.errors.is_empty(), "{:?}", response.errors);
    let link = response.data.into_json().unwrap()["shareSession"].clone();
    let id = link["share"]["id"].as_str().unwrap();

    let change = json!({
        "sessionId": bench.session.to_string(),
        "access": "control",
        "expiresAt": "2026-11-04T09:05:00Z",
        "maxViewers": 10,
    });
    let create = (
        "create".to_owned(),
        id.to_owned(),
        CLIENT.to_owned(),
        change,
    );
    assert_eq!(share_audits(&pool).await, [create]);
    let event = (
        "info".to_owned(),
        "session".to_owned(),
        Some("docker-host".to_owned()),
        "An operator shared a session.".to_owned(),
    );
    assert_eq!(events(&pool).await, [event]);
    let stored = stored_text(&pool).await;
    let url = link["url"].as_str().unwrap();
    assert!(!stored.contains(token_of(url)), "no token is stored");
    assert!(!stored.contains("/shared#"));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn ra_035_session_shares_lists_the_session_s_live_shares(pool: PgPool) {
    let bench = bench(&pool).await;
    assert_eq!(bench.listed().await, Vec::<String>::new());
    let first = bench.shared().await["share"]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let second = bench.shared().await["share"]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(bench.listed().await, [first, second]);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn api_003_share_session_and_end_share_need_a_signed_in_operator(pool: PgPool) {
    let bench = bench(&pool).await;
    let variables = json!({ "id": bench.session.to_string(), "input": {} });
    let end = "mutation { endShare(id: \"0190a000-0000-7000-8020-000000000001\") }";
    for (query, variables) in [(SHARE, variables), (end, json!({}))] {
        let request = Request::new(query).variables(Variables::from_json(variables));
        let response = bench.schema.execute(request).await;
        assert_eq!(error_of(&response).0, "UNAUTHENTICATED");
    }
    assert!(bench.live.shares().hashes().is_empty());
}
