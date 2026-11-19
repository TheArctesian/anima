//! The context of a transfer event with the evidence for the operator (K-E4 Section 12; R-145).

use serde_json::{Value, json};
use sqlx::PgPool;

use super::{LocationHub, NOW, fix};
use crate::hub::start;

/// The context of the event `event`, or null.
async fn context(hub: &LocationHub, event: uuid::Uuid) -> Value {
    let query = "query($e: ID!) { transferContext(event: $e) { eventId direction actorKind \
                 actor clientDeviceId source { deviceId siteId placeId } \
                 destination { deviceId siteId placeId } \
                 operator { basis sampleId offsetSeconds uncertain placeId } } }";
    let answer = hub.post(query, json!({ "e": event.to_string() })).await;
    assert!(answer["errors"].is_null(), "{answer}");
    answer["data"]["transferContext"].clone()
}

#[sqlx::test(migrations = "./migrations")]
async fn r_145_the_context_of_a_transfer_gives_the_roles_and_the_operator_apart(pool: PgPool) {
    let hub = LocationHub::new(&pool).await;
    let place: uuid::Uuid = sqlx::query_scalar(
        "insert into places.place \
         (id, name, latitude, longitude, radius_m, category, icon, created_at, updated_at) \
         values (gen_random_uuid(), 'Cafe', 52.0, 4.0, 100, 'leisure', 'cup', now(), now()) \
         returning id",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let (event, device, site, office) = (
        uuid::Uuid::now_v7(),
        uuid::Uuid::now_v7(),
        uuid::Uuid::now_v7(),
        uuid::Uuid::now_v7(),
    );
    sqlx::query(
        "insert into locations.transfer_context (event_id, occurred_at, direction, \
         source_device, source_site, source_place, actor_kind, actor) \
         values ($1, $2, 'download', $3, $4, $5, 'human', 'operator')",
    )
    .bind(event)
    .bind(start())
    .bind(device)
    .bind(site)
    .bind(office)
    .execute(&pool)
    .await
    .unwrap();
    let unknown = context(&hub, event).await;
    assert_eq!(unknown["operator"]["basis"], "UNKNOWN");
    assert!(unknown["operator"]["placeId"].is_null());
    assert_eq!(unknown["source"]["placeId"], office.to_string());
    assert!(
        unknown["destination"]["deviceId"].is_null(),
        "unknown, not the hub"
    );
    assert_eq!(
        (&unknown["actorKind"], &unknown["actor"]),
        (&json!("HUMAN"), &json!("operator"))
    );

    // A fix that the phone made one minute before the event arrives late.
    let (_, token) = hub.ingest_token("Phone").await;
    hub.owntracks(&token, &fix(NOW - 60)).await;
    let at_the_time = context(&hub, event).await;
    let operator = &at_the_time["operator"];
    assert_eq!(
        (&operator["basis"], &operator["offsetSeconds"]),
        (&json!("AT_THE_TIME"), &json!(-60))
    );
    assert_eq!(
        operator["placeId"],
        place.to_string(),
        "the operator is at the cafe"
    );
    assert_eq!(
        at_the_time["source"]["placeId"],
        office.to_string(),
        "the device is not"
    );

    // A newer position elsewhere does not replace the evidence of the older event.
    let later = json!({ "_type": "location", "tst": NOW + 7200, "lat": 48.0, "lon": 11.0,
                        "acc": 5, "tid": "ph" });
    hub.owntracks(&token, &later.to_string()).await;
    assert_eq!(context(&hub, event).await["operator"], *operator);

    let other = context(&hub, uuid::Uuid::now_v7()).await;
    assert!(other.is_null(), "not a transfer: {other}");
}
