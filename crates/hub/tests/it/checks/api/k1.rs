//! The checks part of the hub declares exactly the K1 root fields it owns (CON-002; the exact
//! schema comparison is `contract`).

use super::harness::schema_for;
use crate::contract::{k1, schema};

/// The K1 root fields the checks part builds; the others belong to other packages.
const OWNED_ROOT: [&str; 9] = [
    "check",
    "checkDefaults",
    "createCheck",
    "deleteCheck",
    "deviceStatusChanged",
    "heartbeatAdded",
    "setCheckDefaults",
    "setCheckPaused",
    "updateCheck",
];

#[tokio::test]
async fn k1_the_checks_part_declares_exactly_its_root_fields_as_k1_does() {
    // The schema never touches the database for its SDL, so a lazy pool is enough.
    let pool = sqlx::PgPool::connect_lazy("postgres://unused@127.0.0.1:1/none").unwrap();
    let part = schema(&schema_for(&pool).sdl());
    let k1 = k1();
    let mut root = Vec::new();
    for (ours, theirs) in [
        ("ChecksQuery", "Query"),
        ("ChecksMutation", "Mutation"),
        ("ChecksSubscription", "Subscription"),
    ] {
        for (field, signature) in &part[ours].members {
            assert_eq!(k1[theirs].members.get(field), Some(signature), "{field}");
            root.push(field.as_str());
        }
    }
    root.sort_unstable();
    assert_eq!(root, OWNED_ROOT);
}
