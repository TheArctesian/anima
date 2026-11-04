//! Secret values never reach a stored event, the bus or a stored audit entry (LOG-007, D-3).

use serde_json::{Value, json};
use sqlx::PgPool;

use super::{DEVICE, day, device_event, new_audit, stored_event};
use crate::events::{Bus, NewEvent, Signal, audit, record};

/// A URL password, an HTTP credential or a private key in the message or the device name never
/// reaches the stored row or the bus (H-6).
#[sqlx::test(migrations = "./migrations")]
async fn log_007_record_stores_and_publishes_message_and_device_name_redacted(pool: PgPool) {
    let mut conn = pool.acquire().await.unwrap();
    let bus = Bus::new(4);
    let mut receiver = bus.subscribe();
    let mut event = device_event("Ping lost on postgres://admin:hunter2@db:5432/anima");
    event.device = Some((DEVICE.parse().unwrap(), "postgres://admin:pw@db".to_owned()));
    let bearer = NewEvent {
        message: "Bearer eyJhbGciOi".to_owned(),
        ..device_event("")
    };

    let id = record(&mut conn, &bus, day(2026, 3, 14), event)
        .await
        .unwrap();
    let bearer_id = record(&mut conn, &bus, day(2026, 3, 14), bearer)
        .await
        .unwrap();

    let stored = stored_event(&mut conn, id).await;
    assert_eq!(
        stored.message,
        "Ping lost on postgres://admin:[redacted]@db:5432/anima"
    );
    assert_eq!(
        stored.device,
        Some((
            DEVICE.parse().unwrap(),
            "postgres://admin:[redacted]@db".to_owned()
        ))
    );
    assert_eq!(
        stored_event(&mut conn, bearer_id).await.message,
        "[redacted]"
    );
    let Ok(Signal::Event(published)) = receiver.try_recv() else {
        panic!("no event published");
    };
    assert_eq!(published, stored);
    let Ok(Signal::Event(published)) = receiver.try_recv() else {
        panic!("no second event published");
    };
    assert_eq!(published.message, "[redacted]");
}

#[sqlx::test(migrations = "./migrations")]
async fn log_007_audit_stores_secret_values_redacted(pool: PgPool) {
    let mut conn = pool.acquire().await.unwrap();
    let change = json!({ "ssh": { "username": "admin", "password": "hunter2" } });

    audit(&mut conn, day(2026, 3, 14), new_audit(change))
        .await
        .unwrap();

    let stored: Value = sqlx::query_scalar("select change from events.audit_log")
        .fetch_one(&mut *conn)
        .await
        .unwrap();
    assert_eq!(
        stored,
        json!({ "ssh": { "username": "admin", "password": "[redacted]" } })
    );
}
