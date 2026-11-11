//! The VPN mutations through the hub and a fake broker: D-23 refusals, the audit entries, the
//! error codes of broker refusals, and that no key reaches a log, an event or an audit entry
//! (R-055, VP-7, VP-10, D-23, LOG-005, LOG-007).

use std::time::Duration;

use anima_hub::{events::process_log, vpn::Feed};
use chrono::TimeDelta;
use serde_json::{Value, json};
use sqlx::PgPool;

use super::{
    broker::{FakeBroker, nodes_value, ok, read_with, refusal, start},
    graphql::{create_site, data, hub_reading},
};
use crate::{
    hub::{Hub, start as clock_start},
    logs::Capture,
};

const KEY: &str = "REDACTED-PREAUTH-KEY";
const AUTH_ID: &str = "hskey-authreq-AAAAAAAAAAAA-_BBBBBBBBBB";

const ROUTER_KEY: &str =
    "mutation($s: ID!) { createRouterKey(siteId: $s) { key expiresAt steps } }";
const INVITE: &str = "mutation { createVpnInvite { key expiresAt steps } }";
const REGISTER: &str =
    "mutation($a: String!) { registerVpnNode(authId: $a) { name address keyExpiry } }";
const RENEW: &str = "mutation($n: String!) { renewVpnKey(name: $n) { name address keyExpiry } }";

/// The broker's answer to one request line: `nodes` as `nodes_value`, the rest by `others`.
fn answering(others: impl Fn(&Value) -> String + Send + Sync + 'static) -> FakeBroker {
    start(Duration::ZERO, move |line| {
        let request: Value = serde_json::from_str(line).unwrap();
        if request["verb"] == "nodes" {
            Some(ok(&nodes_value()))
        } else {
            Some(others(&request))
        }
    })
}

fn new_key() -> String {
    ok(&json!({ "key": KEY, "expiresAt": "2030-05-05T13:00:00.000Z", "keyId": 7 }))
}

/// A signed-in hub over `fake`, after one read, with the sites home and office1; returns the
/// hub, its token and home's id.
async fn hub_over(
    pool: &PgPool,
    fake: &FakeBroker,
) -> (Hub, String, String, tokio::task::JoinHandle<()>) {
    let feed = Feed::new();
    let (hub, token) = hub_reading(pool, &feed, Some(fake.broker())).await;
    let home = create_site(&hub, &token, "home", "10.10.0.0/24").await;
    create_site(&hub, &token, "office1", "10.20.0.0/24").await;
    let reader = read_with(pool, &feed, fake.broker()).await;
    (hub, token, home, reader)
}

/// The requests the fake saw, other than `nodes`.
fn asked(fake: &FakeBroker) -> Vec<Value> {
    fake.seen()
        .iter()
        .map(|line| serde_json::from_str::<Value>(line).unwrap())
        .filter(|request| request["verb"] != "nodes")
        .collect()
}

fn code(response: &Value) -> &Value {
    &response["errors"][0]["extensions"]["code"]
}

/// Every audit change and event message the hub wrote, as one text.
async fn written(pool: &PgPool) -> String {
    sqlx::query_scalar(
        "select coalesce((select string_agg(change::text || ' ' || target_type || ' ' || action, ' ') \
         from events.audit_log), '') || ' ' || \
         coalesce((select string_agg(source || ' ' || message, ' ') from events.event_log), '')",
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn d_23_vpn_mutations_refuse_an_api_key(pool: PgPool) {
    let fake = answering(|_| new_key());
    let (hub, token, home, reader) = hub_over(&pool, &fake).await;
    let create = "mutation($input: ApiKeyInput!) { createApiKey(input: $input) { secret } }";
    let input = json!({ "name": "script", "scope": "WRITE",
                        "expiresAt": clock_start() + TimeDelta::days(30) });
    let made = data(&hub, &token, create, json!({ "input": input })).await;
    let secret = made["createApiKey"]["secret"].as_str().unwrap();

    for (query, variables) in [
        (INVITE, json!({})),
        (ROUTER_KEY, json!({ "s": home })),
        (REGISTER, json!({ "a": AUTH_ID })),
        (RENEW, json!({ "n": "mac" })),
        (APPROVE, json!({ "s": home })),
    ] {
        let response = hub.post(Some(secret), query, variables).await;
        assert_eq!(code(&response), "FORBIDDEN", "{query}: {response}");
    }
    reader.abort();

    assert_eq!(asked(&fake), Vec::<Value>::new());
}

#[sqlx::test(migrations = "./migrations")]
async fn vp_7_create_router_key_returns_the_key_once_and_writes_an_audit_entry_without_it(
    pool: PgPool,
) {
    let capture = Capture::default();
    let writer = capture.clone();
    let subscriber = process_log::subscriber("debug", move || writer.clone()).unwrap();
    let _default = tracing::subscriber::set_default(subscriber);
    tracing::callsite::rebuild_interest_cache();
    let fake = answering(|_| new_key());
    let (hub, token, home, reader) = hub_over(&pool, &fake).await;

    let made = data(&hub, &token, ROUTER_KEY, json!({ "s": home })).await;
    reader.abort();

    let key = &made["createRouterKey"];
    assert_eq!(key["key"], KEY);
    assert_eq!(key["expiresAt"], "2030-05-05T13:00:00+00:00");
    let steps: Vec<&str> = key["steps"]
        .as_array()
        .unwrap()
        .iter()
        .map(|step| step.as_str().unwrap())
        .collect();
    assert_eq!(steps.len(), 3);
    assert!(steps[1].ends_with(
        "--auth-key=file:/run/anima-preauth --advertise-routes=10.10.0.0/24 --hostname home-router"
    ));
    assert!(steps.iter().all(|step| !step.contains(KEY)));
    assert_eq!(
        asked(&fake),
        [json!({ "verb": "router-key", "site": "home" })]
    );
    let stored = written(&pool).await;
    assert!(stored.contains(r#"{"site": "home""#), "{stored}");
    assert!(stored.contains("vpn create"), "{stored}");
    assert!(stored.contains("vpn Made a one-hour VPN key for the router of home."));
    let logged = capture.text();
    for text in [&stored, &logged] {
        assert!(!text.contains(KEY), "{text}");
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn vpn_003_an_invite_gives_the_three_computer_steps_without_the_key(pool: PgPool) {
    let fake = answering(|_| new_key());
    let (hub, token, _, reader) = hub_over(&pool, &fake).await;

    let made = data(&hub, &token, INVITE, json!({})).await;
    reader.abort();

    assert_eq!(made["createVpnInvite"]["key"], KEY);
    assert_eq!(
        made["createVpnInvite"]["steps"],
        json!([
            r#"sudo bash -c 'umask 077; read -rsp "Pre-auth key: " K; echo; printf "%s" "$K" > /run/anima-preauth'"#,
            "sudo tailscale up --login-server https://hs.anima.ing \
             --auth-key=file:/run/anima-preauth --accept-routes",
            "sudo rm -f /run/anima-preauth",
        ])
    );
    assert_eq!(asked(&fake), [json!({ "verb": "personal-key" })]);
    assert!(written(&pool).await.contains("vpn create"));
}

#[sqlx::test(migrations = "./migrations")]
async fn vp_7_a_closed_connection_with_no_reply_is_unavailable(pool: PgPool) {
    // The broker drops a request that waits too long behind its audit: no line at all.
    let fake = start(Duration::ZERO, |line| {
        line.contains(r#""nodes""#).then(|| ok(&nodes_value()))
    });
    let (hub, token, _, reader) = hub_over(&pool, &fake).await;

    let response = hub.post(Some(&token), INVITE, json!({})).await;
    reader.abort();

    assert_eq!(code(&response), "UNAVAILABLE", "{response}");
    assert!(!written(&pool).await.contains("vpn create"));
}

#[sqlx::test(migrations = "./migrations")]
async fn vp_7_a_router_key_for_a_served_subnet_says_why_it_is_not_allowed(pool: PgPool) {
    let detail = "another router already serves this site's subnet";
    let fake = answering(move |_| refusal("not-allowed", detail));
    let (hub, token, home, reader) = hub_over(&pool, &fake).await;

    let response = hub
        .post(Some(&token), ROUTER_KEY, json!({ "s": home }))
        .await;
    reader.abort();

    assert_eq!(code(&response), "VALIDATION", "{response}");
    let field = &response["errors"][0]["extensions"]["fields"][0];
    assert_eq!(field["field"], "siteId");
    assert!(
        field["message"].as_str().unwrap().contains(detail),
        "{field}"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn vp_7_a_rate_limited_broker_gives_a_conflict_error(pool: PgPool) {
    let fake = answering(|_| {
        refusal(
            "rate-limited",
            "at most 3 keys and 3 registrations in 24 hours",
        )
    });
    let (hub, token, _, reader) = hub_over(&pool, &fake).await;

    let response = hub.post(Some(&token), INVITE, json!({})).await;
    reader.abort();

    assert_eq!(code(&response), "CONFLICT", "{response}");
    assert_eq!(asked(&fake), [json!({ "verb": "personal-key" })]);
    assert!(!written(&pool).await.contains("vpn create"));
}

#[sqlx::test(migrations = "./migrations")]
async fn vp_7_register_undone_gives_a_validation_error_that_says_the_node_had_a_tag(pool: PgPool) {
    let fake = answering(|_| refusal("undone", "the new node was deleted: the node had a tag"));
    let (hub, token, _, reader) = hub_over(&pool, &fake).await;

    let response = hub
        .post(Some(&token), REGISTER, json!({ "a": AUTH_ID }))
        .await;
    reader.abort();

    assert_eq!(code(&response), "VALIDATION", "{response}");
    let field = &response["errors"][0]["extensions"]["fields"][0];
    assert_eq!(field["field"], "authId");
    let message = field["message"].as_str().unwrap();
    assert!(message.contains("the node had a tag"), "{message}");
    assert_eq!(
        asked(&fake),
        [json!({ "verb": "register", "authId": AUTH_ID })]
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn vp_7_an_auth_id_of_another_form_is_refused_before_the_broker(pool: PgPool) {
    let fake = answering(|_| refusal("bad-request", "-"));
    let (hub, token, _, reader) = hub_over(&pool, &fake).await;

    for bad in ["hskey-authreq-short", "--auth-id=x", &format!("{AUTH_ID} ")] {
        let response = hub.post(Some(&token), REGISTER, json!({ "a": bad })).await;
        assert_eq!(code(&response), "VALIDATION", "{bad}: {response}");
    }
    reader.abort();

    assert_eq!(asked(&fake), Vec::<Value>::new());
}

#[sqlx::test(migrations = "./migrations")]
async fn vp_10_renew_sends_the_broker_id_of_the_named_operator_node(pool: PgPool) {
    let fake = answering(|_| {
        ok(&json!({ "name": "mac", "address": "100.64.0.2",
                    "keyExpiry": "2030-11-01T12:00:00.000Z" }))
    });
    let (hub, token, _, reader) = hub_over(&pool, &fake).await;

    let renewed = data(&hub, &token, RENEW, json!({ "n": "mac" })).await;
    let tagged = hub
        .post(Some(&token), RENEW, json!({ "n": "router-home" }))
        .await;
    reader.abort();

    assert_eq!(
        renewed["renewVpnKey"],
        json!({ "name": "mac", "address": "100.64.0.2", "keyExpiry": "2030-11-01T12:00:00+00:00" })
    );
    assert_eq!(code(&tagged), "VALIDATION", "{tagged}");
    assert_eq!(asked(&fake), [json!({ "verb": "renew", "node": 2 })]);
    assert!(written(&pool).await.contains("vpn update"));
}

const APPROVE: &str = "mutation($s: ID!) { approveSiteRoute(siteId: $s) }";

/// office1's id: the site whose router waits in `nodes_value`.
async fn office1(hub: &Hub, token: &str) -> String {
    let sites = data(hub, token, "{ sites { id name } }", json!({})).await;
    let found = sites["sites"]
        .as_array()
        .unwrap()
        .iter()
        .find(|site| site["name"] == "office1");
    found.unwrap()["id"].as_str().unwrap().to_owned()
}

#[sqlx::test(migrations = "./migrations")]
async fn vp_7_approve_route_writes_an_audit_entry(pool: PgPool) {
    let fake = answering(|_| ok(&json!({ "node": 9, "site": "office1" })));
    let (hub, token, _, reader) = hub_over(&pool, &fake).await;
    let site = office1(&hub, &token).await;

    let approved = data(&hub, &token, APPROVE, json!({ "s": site })).await;
    reader.abort();

    assert_eq!(approved["approveSiteRoute"], true);
    assert_eq!(
        asked(&fake),
        [json!({ "verb": "approve-route", "node": 9, "site": "office1" })]
    );
    let stored = written(&pool).await;
    assert!(
        stored
            .contains(r#"{"site": "office1", "route": "10.20.0.0/24", "router": "router-office"}"#),
        "{stored}"
    );
    assert!(stored.contains("vpn accept"), "{stored}");
}

#[sqlx::test(migrations = "./migrations")]
async fn r_055_approve_route_refuses_a_router_of_another_site(pool: PgPool) {
    let fake = answering(|_| refusal("not-allowed", "the node does not qualify for this verb"));
    let (hub, token, home, reader) = hub_over(&pool, &fake).await;
    let site = office1(&hub, &token).await;

    let refused = hub.post(Some(&token), APPROVE, json!({ "s": site })).await;
    let none_waits = hub.post(Some(&token), APPROVE, json!({ "s": home })).await;
    reader.abort();

    assert_eq!(code(&refused), "VALIDATION", "{refused}");
    assert_eq!(code(&none_waits), "VALIDATION", "{none_waits}");
    assert_eq!(
        asked(&fake).len(),
        1,
        "home has no waiting router, so no broker call"
    );
    assert!(!written(&pool).await.contains("vpn accept"));
}
