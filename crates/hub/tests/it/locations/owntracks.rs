//! The OwnTracks path and its ingest tokens (K-E4 Section 11; R-145, R-146, LOG-007).

use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use serde_json::{Value, json};
use sqlx::PgPool;

use super::{LocationHub, NOW, count, fix, logs};
use crate::hub::{error, start};

#[sqlx::test(migrations = "./migrations")]
async fn r_145_an_owntracks_post_is_stored_once_with_its_provider_and_both_times(pool: PgPool) {
    assert_eq!(start().timestamp(), NOW, "the fixture time of these tests");
    let hub = LocationHub::new(&pool).await;
    let (id, token) = hub.ingest_token("Phone").await;
    let observed = NOW - 3600;

    let (status, answer) = hub.owntracks(&token, &fix(observed)).await;
    assert_eq!((status, answer), (StatusCode::OK, json!([])));
    // The app sends the same fix again after a lost answer: still one row.
    let (status, _) = hub.owntracks(&token, &fix(observed)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(count(&pool, "locations.sample").await, 1);

    let read = hub.samples().await;
    let samples = read["data"]["locationSamples"].as_array().unwrap();
    assert_eq!(samples.len(), 1, "{read}");
    let sample = &samples[0];
    assert_eq!(
        (&sample["provider"], &sample["account"], &sample["key"]),
        (
            &json!("OWNTRACKS"),
            &json!(id),
            &json!(format!("ph:{observed}"))
        )
    );
    assert_eq!(sample["observedAt"], "2030-05-05T11:00:00+00:00");
    assert_eq!(sample["receivedAt"], "2030-05-05T12:00:00+00:00");
    assert_eq!(
        (
            &sample["latitude"],
            &sample["longitude"],
            &sample["accuracyM"]
        ),
        (&json!(52.0), &json!(4.0), &json!(12.0))
    );
    assert!(sample["altitudeM"].is_null() && sample["speedMps"].is_null());
}

#[sqlx::test(migrations = "./migrations")]
async fn r_146_log_007_a_position_is_sealed_and_no_log_holds_it_or_the_token(pool: PgPool) {
    let hub = LocationHub::new(&pool).await;
    let (_, token) = hub.ingest_token("Phone").await;
    let body = json!({ "_type": "location", "tst": NOW, "lat": 52.123_456, "lon": 4.654_321 });
    assert_eq!(
        hub.owntracks(&token, &body.to_string()).await.0,
        StatusCode::OK
    );
    hub.samples().await;

    let sealed: Vec<u8> = sqlx::query_scalar("select sealed from locations.sample")
        .fetch_one(&pool)
        .await
        .unwrap();
    let sealed = String::from_utf8_lossy(&sealed).into_owned();
    let stored = logs(&pool).await;
    for secret in ["52.123456", "4.654321", token.as_str()] {
        assert!(!sealed.contains(secret), "the sealed column holds {secret}");
        assert!(!stored.contains(secret), "a log holds {secret}: {stored}");
    }
    let hashes: i64 = sqlx::query_scalar(
        "select count(*) from locations.ingest_token where length(token_hash) = 32",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(hashes, 1, "the hub keeps the hash of the token only");
    let reads: Vec<Value> = sqlx::query_scalar(
        "select change from events.audit_log where target_type = 'location_samples' \
         and action = 'read'",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(reads.len(), 1, "one audit entry for the read");
    assert_eq!(reads[0]["count"], 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn r_145_the_owntracks_path_refuses_a_wrong_a_revoked_and_a_login_token(pool: PgPool) {
    let hub = LocationHub::new(&pool).await;
    let (id, token) = hub.ingest_token("Phone").await;
    for wrong in ["anl_wrong", hub.token.as_str(), ""] {
        let (status, _) = hub.owntracks(wrong, &fix(NOW)).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{wrong}");
    }
    let (status, _) = hub
        .owntracks(&token, r#"{"_type":"transition","tst":1}"#)
        .await;
    assert_eq!(status, StatusCode::OK, "another type is accepted");
    let (status, _) = hub.owntracks(&token, "not json").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(count(&pool, "locations.sample").await, 0);

    let listed = hub
        .post(
            "{ locationIngestTokens { id name lastUsedAt revokedAt } }",
            json!({}),
        )
        .await;
    let shown = &listed["data"]["locationIngestTokens"][0];
    assert_eq!(shown["id"], id.as_str(), "{listed}");
    assert!(shown["lastUsedAt"].is_string() && shown["revokedAt"].is_null());

    let revoked = hub
        .post(
            "mutation($id: ID!) { revokeLocationIngestToken(id: $id) { revokedAt } }",
            json!({ "id": id }),
        )
        .await;
    assert!(revoked["data"]["revokeLocationIngestToken"]["revokedAt"].is_string());
    let (status, _) = hub.owntracks(&token, &fix(NOW)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(count(&pool, "locations.sample").await, 0);
    // The ingest token opens nothing else of the hub.
    let request = Request::post("/graphql")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({ "query": "{ places { id } }" }).to_string(),
        ))
        .unwrap();
    let (_, answer) = hub.send(request).await;
    assert_eq!(error(&answer).1, "UNAUTHENTICATED");
}
