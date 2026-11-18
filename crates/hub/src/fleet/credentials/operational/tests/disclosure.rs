//! After a whole life of a key, no table, audit entry, event, error text or `Debug` output
//! holds a value: only the sealed column holds it, and not as plain bytes (LOG-007, SEC-003).

use sqlx::PgPool;

use super::{Desk, NEXT_VALUE, VALUE, database_now, metadata, pve};
use crate::fleet::credentials::Secret;
use crate::fleet::credentials::operational::{Error, NewKey, Problem, create, key, remove};

/// Every text of the operational key tables and of both logs, as one text.
async fn stored_text(pool: &PgPool) -> String {
    let parts: Vec<String> = sqlx::query_scalar(
        "SELECT row_to_json(k)::text FROM credentials.operational_keys k \
         UNION ALL SELECT row_to_json(c)::text FROM credentials.operational_key_consumers c \
         UNION ALL SELECT json_build_object('state', v.state, 'by', v.created_by)::text \
             FROM credentials.operational_key_versions v \
         UNION ALL SELECT a.change::text || a.target_type || a.action FROM events.audit_log a \
         UNION ALL SELECT e.message FROM events.event_log e",
    )
    .fetch_all(pool)
    .await
    .unwrap();
    parts.join("\n")
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn log_007_a_whole_life_leaves_no_value_outside_the_sealed_column(pool: PgPool) {
    let now = database_now(&pool).await;
    let desk = Desk::new(&pool);
    desk.create("cloudflare.dns", metadata(), now).await;
    desk.grant("cloudflare.dns", pve(), now).await;
    desk.fetch(pve(), "cloudflare.dns", now).await.unwrap();
    let rotated = desk.rotate("cloudflare.dns", NEXT_VALUE, now).await;
    desk.fetch(pve(), "cloudflare.dns", now).await.unwrap();
    let revoked = desk.revoke("cloudflare.dns", now).await;
    let mut tx = pool.begin().await.unwrap();
    let removed = remove(&mut tx, "cloudflare.dns", desk.trail(now))
        .await
        .unwrap();
    tx.commit().await.unwrap();

    let text = stored_text(&pool).await;
    assert!(text.contains("cloudflare.dns"), "the scan sees the rows");
    for value in [VALUE, NEXT_VALUE] {
        assert!(!text.contains(value), "a value is stored as text");
        for shown in [
            format!("{rotated:?}"),
            format!("{revoked:?}"),
            format!("{removed:?}"),
        ] {
            assert!(!shown.contains(value), "Debug shows a value");
        }
    }
    let sealed: Vec<Vec<u8>> =
        sqlx::query_scalar("SELECT sealed FROM credentials.operational_key_versions")
            .fetch_all(&pool)
            .await
            .unwrap();
    for bytes in sealed {
        for value in [VALUE, NEXT_VALUE] {
            assert!(
                !bytes.windows(value.len()).any(|w| w == value.as_bytes()),
                "a sealed value holds its plain bytes"
            );
        }
    }
    assert!(key(&pool, "cloudflare.dns", now).await.unwrap().is_none());
}

#[sqlx::test(migrations = "./migrations")]
async fn log_007_a_refused_create_names_the_field_and_never_the_value(pool: PgPool) {
    let now = database_now(&pool).await;
    let desk = Desk::new(&pool);
    let pasted_in_the_wrong_field = metadata();
    let new = NewKey {
        name: VALUE.to_owned(),
        metadata: pasted_in_the_wrong_field,
        value: Secret::from(String::new()),
    };
    let mut tx = pool.begin().await.unwrap();
    let refused = create(&mut tx, &desk.vault, new, desk.trail(now)).await;

    let Err(Error::Invalid(problems)) = &refused else {
        panic!("{refused:?}");
    };
    let fields: Vec<&str> = problems.iter().map(|p: &Problem| p.field).collect();
    assert_eq!(fields, ["name", "value"]);
    let shown = format!("{refused:?} {}", refused.as_ref().err().unwrap());
    assert!(!shown.contains(VALUE), "{shown}");
}
