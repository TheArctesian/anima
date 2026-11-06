//! The sessions part of the hub declares exactly its K1 root fields (CON-002; the exact schema
//! comparison is `contract`).

use sqlx::PgPool;

use super::harness::schema_for;
use crate::contract::{Schema, k1, schema};
use anima_hub::sessions::EndReason;

/// The sessions part alone, for the root fields it declares.
fn part() -> Schema {
    // The schema never touches the database for its SDL, so a lazy pool is enough.
    let pool = PgPool::connect_lazy("postgres://unused@127.0.0.1:1/none").unwrap();
    schema(&schema_for(&pool).sdl())
}

fn fields(schema: &Schema, root: &str) -> Vec<String> {
    schema[root].members.keys().cloned().collect()
}

#[tokio::test]
async fn k1_the_sessions_part_declares_exactly_its_root_fields_as_k1_does() {
    let (part, k1) = (part(), k1());
    assert_eq!(fields(&part, "SessionsQuery"), ["sessions"]);
    assert_eq!(
        fields(&part, "SessionsMutation"),
        [
            "acceptHostKey",
            "endSession",
            "startSession",
            "testConnection"
        ]
    );
    for (ours, theirs) in [("SessionsQuery", "Query"), ("SessionsMutation", "Mutation")] {
        for (field, signature) in &part[ours].members {
            assert_eq!(k1[theirs].members.get(field), Some(signature), "{field}");
        }
    }
}

#[test]
fn ra_008_every_k1_end_reason_is_stored_as_its_lower_snake_name() {
    let k1 = k1();
    let names: Vec<String> = k1["SessionEndReason"]
        .members
        .keys()
        .map(|n| n.to_lowercase())
        .collect();
    let mut stored: Vec<String> = EndReason::ALL
        .iter()
        .map(|r| r.as_str().to_owned())
        .collect();
    stored.sort();
    assert_eq!(stored, names);
    for reason in EndReason::ALL {
        assert_eq!(reason.as_str().parse::<EndReason>(), Ok(reason));
    }
    assert!("crashed".parse::<EndReason>().is_err());
}
