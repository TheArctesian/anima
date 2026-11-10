//! K8 Sections 4 and 6, MET-015, MET-017, NET-002: a push is refused with the first refusal in
//! K8's order, each written as a warning with its code and address, the device when the token
//! was valid, and never the token; one with no valid token at most once a minute per address.
//! An accepted batch stores each mean at 10 seconds, its max in the aggregates, and nothing
//! twice.

use axum::http::StatusCode;
use chrono::{DurationRound as _, TimeDelta};
use serde_json::json;
use sqlx::PgPool;

use super::{
    batches::{aggregates, batch, cpu_batch, point, point_with_max, samples_of, warnings},
    harness::{Agents, DOCKER_HOST, NEIGHBOUR, OUTSIDE, PERMITTED, PVE_01, pushing_now, refusal},
};

fn refused(device: Option<&str>, address: &str, code: &str) -> (Option<String>, String) {
    let message = format!("Refused an agent push from {address}: {code}.");
    (device.map(str::to_owned), message)
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn k8_6_each_push_refusal_has_its_status_code_and_event_and_never_the_token(pool: PgPool) {
    let now = pushing_now(&pool).await;
    let agents = Agents::at(&pool, now).await;
    let token = agents.token_for(PVE_01).await;
    let bearer = format!("Bearer {token}");
    let good = cpu_batch(PVE_01, now, 5.0);
    let mut other = good.clone();
    other["device"] = json!(DOCKER_HOST);
    let mut shapeless = good.clone();
    shapeless["series"] = json!([]);
    let old = cpu_batch(PVE_01, now - TimeDelta::hours(2), 5.0);
    let large = format!("{{\"pad\":\"{}\"}}", "a".repeat(512 * 1024));

    let cases = [
        (None, String::from("{}"), PERMITTED, 401, "TOKEN_REFUSED"),
        (
            Some("Bearer anima_agent_x"),
            good.to_string(),
            NEIGHBOUR,
            401,
            "TOKEN_REFUSED",
        ),
        (Some(bearer.as_str()), large, PERMITTED, 413, "TOO_LARGE"),
        (
            Some(bearer.as_str()),
            "{not json".to_owned(),
            PERMITTED,
            400,
            "MALFORMED",
        ),
        (
            Some(bearer.as_str()),
            other.to_string(),
            PERMITTED,
            403,
            "WRONG_DEVICE",
        ),
        (
            Some(bearer.as_str()),
            shapeless.to_string(),
            PERMITTED,
            400,
            "MALFORMED",
        ),
        (
            Some(bearer.as_str()),
            old.to_string(),
            PERMITTED,
            422,
            "OUT_OF_RANGE",
        ),
    ];
    for (authorization, body, from, status, code) in &cases {
        let answer = agents.push_raw(*authorization, body.clone(), from).await;
        assert_eq!(answer.0.as_u16(), *status, "{code}");
        assert_eq!(answer.1, refusal(code));
        // A minute apart, so neither throttle holds an event back (CR-15, CR-21).
        agents.clock.advance(TimeDelta::minutes(1));
    }
    let expected: Vec<_> = cases
        .iter()
        .map(|(authorization, _, from, _, code)| {
            let device = (*authorization == Some(bearer.as_str())).then_some("pve-01");
            refused(device, from, code)
        })
        .collect();
    let mut written = warnings(&pool).await;
    written.sort();
    let mut expected = expected;
    expected.sort();
    assert_eq!(written, expected);
    let everything: String = sqlx::query_scalar(
        "SELECT concat_ws(' ', (SELECT string_agg(l::text, ' ') FROM events.event_log l), \
             (SELECT string_agg(l::text, ' ') FROM events.audit_log l))",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(
        !everything.contains(&token[12..]),
        "a token reached an event"
    );
    assert!(!everything.contains("anima_agent_"), "{everything}");
    assert_eq!(samples_of(&pool, PVE_01).await, 0);
    assert_eq!(samples_of(&pool, DOCKER_HOST).await, 0);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn met_017_one_devices_token_cannot_push_another_devices_batch(pool: PgPool) {
    let now = pushing_now(&pool).await;
    let agents = Agents::at(&pool, now).await;
    let pve = agents.token_for(PVE_01).await;
    agents.token_for(DOCKER_HOST).await;
    let answer = agents.push(&pve, &cpu_batch(DOCKER_HOST, now, 5.0)).await;
    assert_eq!(answer, (StatusCode::FORBIDDEN, refusal("WRONG_DEVICE")));
    assert_eq!(samples_of(&pool, DOCKER_HOST).await, 0);
    assert_eq!(samples_of(&pool, PVE_01).await, 0);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn cr_15_push_refusals_without_a_valid_token_are_written_once_a_minute_per_address(
    pool: PgPool,
) {
    let now = pushing_now(&pool).await;
    let agents = Agents::at(&pool, now).await;
    let token = agents.token_for(PVE_01).await;
    let bad = Some("Bearer anima_agent_nothing");
    for _ in 0..3 {
        agents.push_raw(bad, "{}", PERMITTED).await;
        agents.push_raw(None, "{}", NEIGHBOUR).await;
    }
    // A valid token's refusals are held back per device, not per address (CR-21).
    agents
        .push_raw(Some(&format!("Bearer {token}")), "[]", NEIGHBOUR)
        .await;
    let mut expected = vec![
        refused(None, PERMITTED, "TOKEN_REFUSED"),
        refused(None, NEIGHBOUR, "TOKEN_REFUSED"),
        refused(Some("pve-01"), NEIGHBOUR, "MALFORMED"),
    ];
    expected.sort();
    let mut written = warnings(&pool).await;
    written.sort();
    assert_eq!(written, expected);

    agents.clock.advance(TimeDelta::minutes(1));
    agents.push_raw(bad, "{}", PERMITTED).await;
    assert_eq!(warnings(&pool).await.len(), 4);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn net_002_a_push_from_outside_the_permitted_networks_never_reaches_the_route(pool: PgPool) {
    let now = pushing_now(&pool).await;
    let agents = Agents::at(&pool, now).await;
    let token = agents.token_for(PVE_01).await;
    let bearer = format!("Bearer {token}");
    let body = cpu_batch(PVE_01, now, 5.0).to_string();
    let answer = agents.push_raw(Some(&bearer), body, OUTSIDE).await;
    assert_eq!(answer.0, StatusCode::FORBIDDEN);
    assert_eq!(samples_of(&pool, PVE_01).await, 0);
    let refusals: Vec<String> =
        sqlx::query_scalar("SELECT message FROM events.event_log WHERE source = 'access'")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert!(
        refusals.iter().any(|message| message.contains(OUTSIDE)),
        "{refusals:?}"
    );
    assert!(warnings(&pool).await.is_empty(), "the route saw nothing");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn met_015_each_mean_is_a_10_second_sample_and_a_repeated_batch_adds_nothing(pool: PgPool) {
    let now = pushing_now(&pool).await;
    let agents = Agents::at(&pool, now).await;
    let token = agents.token_for(PVE_01).await;
    let earlier = now - TimeDelta::seconds(10);
    let pushed = batch(
        PVE_01,
        json!([{ "kind": "cpu_percent" }, { "kind": "disk_percent", "label": "/" }]),
        vec![
            point(earlier, json!([10.0, 40.0])),
            point(now, json!([20.0, null])),
        ],
    );
    for _ in 0..2 {
        assert_eq!(
            agents.push(&token, &pushed).await,
            (StatusCode::NO_CONTENT, String::new())
        );
    }
    let stored: Vec<(String, f64)> = sqlx::query_as(
        "SELECT s.kind || ' ' || s.label || ' ' || to_char(m.at, 'SS'), m.value \
         FROM metrics.samples m JOIN metrics.series s ON s.id = m.series_id \
         ORDER BY s.kind, s.label, m.at",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    let second = |at: chrono::DateTime<chrono::Utc>| at.format("%S").to_string();
    assert_eq!(
        stored,
        [
            (format!("cpu_percent  {}", second(earlier)), 10.0),
            (format!("cpu_percent  {}", second(now)), 20.0),
            (format!("disk_percent / {}", second(earlier)), 40.0),
        ]
    );
    let counted: i32 = aggregates(&pool, "day", PVE_01, "cpu_percent")
        .await
        .iter()
        .map(|row| row.3)
        .sum();
    assert_eq!(
        counted, 2,
        "the repeated batch added nothing to the aggregates"
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn met_015_the_max_lands_in_the_minute_hour_and_day_rows_and_the_mean_in_the_avg(
    pool: PgPool,
) {
    let now = pushing_now(&pool).await;
    let agents = Agents::at(&pool, now).await;
    let token = agents.token_for(PVE_01).await;
    // Two batches, each with a point of one minute five minutes ago, so they share a bucket.
    let minute = (now - TimeDelta::minutes(5))
        .duration_trunc(TimeDelta::minutes(1))
        .unwrap();
    let at = |seconds| minute + TimeDelta::seconds(seconds);
    let series = json!([{ "kind": "cpu_percent" }]);
    let first = batch(
        PVE_01,
        series.clone(),
        vec![point_with_max(at(10), json!([10.0]), json!([50.0]))],
    );
    let second = batch(PVE_01, series, vec![point(at(20), json!([20.0]))]);
    assert_eq!(agents.push(&token, &first).await.0, StatusCode::NO_CONTENT);
    assert_eq!(agents.push(&token, &second).await.0, StatusCode::NO_CONTENT);

    for table in ["minute", "hour", "day"] {
        let rows = aggregates(&pool, table, PVE_01, "cpu_percent").await;
        let (avg, min, max, count) = *rows.last().unwrap();
        assert_eq!(max, 50.0, "{table}: the window's max, not the mean");
        assert_eq!(min, 10.0, "{table}");
        assert_eq!(count, 2, "{table}");
        assert!((avg - 15.0).abs() < 1e-9, "{table}: {avg}");
    }
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn met_011_a_push_after_the_operator_switched_the_source_away_is_token_refused(pool: PgPool) {
    let now = pushing_now(&pool).await;
    let agents = Agents::at(&pool, now).await;
    let token = agents.token_for(PVE_01).await;
    let switch =
        "mutation($id: ID!) { updateDevice(id: $id, input: { metricsSource: SSH }) { id } }";
    let switched = agents.operate(switch, json!({ "id": PVE_01 })).await;
    assert_eq!(switched["data"]["updateDevice"]["id"], PVE_01, "{switched}");

    let answer = agents.push(&token, &cpu_batch(PVE_01, now, 5.0)).await;
    assert_eq!(answer, (StatusCode::UNAUTHORIZED, refusal("TOKEN_REFUSED")));
    assert_eq!(samples_of(&pool, PVE_01).await, 0);
    assert_eq!(
        warnings(&pool).await,
        [refused(Some("pve-01"), PERMITTED, "TOKEN_REFUSED")]
    );
}
