//! The event log shows in the timeline (TML-007) with every Evidence field (TML-002), and the
//! timeline copies nothing (TML-001).

use serde_json::{Value, json};
use sqlx::PgPool;

use anima_hub::fleet::events::Source;

use super::{seed, signed_in, timeline, titles};

#[sqlx::test(migrations = "./migrations")]
async fn tml_007_an_event_shows_as_evidence_with_each_field_of_tml_002(pool: PgPool) {
    let (hub, token) = signed_in(&pool).await;
    let id = seed(&pool, &hub, (Source::Check, "Ping failed."), 10).await;
    let response = timeline(&hub, &token, Value::Null).await;
    let edges = response["data"]["timeline"]["edges"].as_array().unwrap();
    assert_eq!(edges.len(), 1, "{response}");
    assert_eq!(
        edges[0]["node"],
        json!({
            "reference": format!("event:{id}"),
            "source": "event_log",
            "category": "DEVICE",
            "title": "Ping failed.",
            "start": "2030-05-05T11:50:00+00:00",
            "end": null,
            "basis": "OBSERVED",
            "uncertainty": ["End time not given by the source."],
            "supersededBy": null,
        })
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn tml_004_a_session_event_is_a_session_and_a_category_filter_selects_it(pool: PgPool) {
    let (hub, token) = signed_in(&pool).await;
    seed(&pool, &hub, (Source::Session, "SSH session started."), 20).await;
    seed(&pool, &hub, (Source::Vpn, "A key expires soon."), 10).await;
    let all = timeline(&hub, &token, Value::Null).await;
    assert_eq!(
        titles(&all),
        ["A key expires soon.", "SSH session started."]
    );
    let sessions = timeline(&hub, &token, json!({ "categories": ["SESSION"] })).await;
    assert_eq!(titles(&sessions), ["SSH session started."]);
    assert_eq!(
        sessions["data"]["timeline"]["edges"][0]["node"]["category"],
        "SESSION"
    );
    let devices = timeline(&hub, &token, json!({ "categories": ["DEVICE", "SLEEP"] })).await;
    assert_eq!(titles(&devices), ["A key expires soon."]);
    let none = timeline(&hub, &token, json!({ "categories": [] })).await;
    assert_eq!(titles(&none), Vec::<&str>::new());
    let sleep = timeline(&hub, &token, json!({ "categories": ["SLEEP"] })).await;
    assert_eq!(titles(&sleep), Vec::<&str>::new());
}

#[sqlx::test(migrations = "./migrations")]
async fn api_006_pages_follow_each_other_and_events_of_one_instant_are_not_lost(pool: PgPool) {
    let (hub, token) = signed_in(&pool).await;
    for (message, minutes) in [("one", 30), ("two", 20), ("three", 20), ("four", 5)] {
        seed(&pool, &hub, (Source::Hub, message), minutes).await;
    }
    let mut seen = Vec::new();
    let mut after = Value::Null;
    for page in 0..3 {
        let response = timeline(&hub, &token, json!({ "first": 2, "after": after })).await;
        seen.extend(titles(&response).into_iter().map(str::to_owned));
        let end = &response["data"]["timeline"]["pageInfo"];
        assert_eq!(end["hasNextPage"], page == 0, "page {page}: {response}");
        if end["hasNextPage"] == false {
            break;
        }
        after = end["endCursor"].clone();
    }
    // "two" and "three" share an instant; the later ID comes first.
    assert_eq!(seen, ["four", "three", "two", "one"]);
}

#[sqlx::test(migrations = "./migrations")]
async fn tml_001_reading_the_timeline_stores_nothing(pool: PgPool) {
    let (hub, token) = signed_in(&pool).await;
    seed(
        &pool,
        &hub,
        (Source::Hub, "Kept in the event log only."),
        10,
    )
    .await;
    let tables = "SELECT count(*) FROM information_schema.tables \
                  WHERE table_schema NOT IN ('pg_catalog', 'information_schema')";
    let before: i64 = sqlx::query_scalar(tables).fetch_one(&pool).await.unwrap();
    let events: i64 = sqlx::query_scalar("SELECT count(*) FROM events.event_log")
        .fetch_one(&pool)
        .await
        .unwrap();
    let response = timeline(&hub, &token, Value::Null).await;
    assert_eq!(titles(&response).len(), 1);
    let after: i64 = sqlx::query_scalar(tables).fetch_one(&pool).await.unwrap();
    let events_after: i64 = sqlx::query_scalar("SELECT count(*) FROM events.event_log")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!((before, events), (after, events_after));
    let no_timeline_schema: bool = sqlx::query_scalar(
        "SELECT NOT EXISTS (SELECT 1 FROM information_schema.schemata \
         WHERE schema_name = 'timeline')",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(no_timeline_schema);
}
