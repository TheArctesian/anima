//! The sessions part of the hub schema matches K1 member for member (R-011(c)).

use std::collections::BTreeMap;

use async_graphql::parser::{
    parse_schema,
    types::{InputValueDefinition, TypeKind, TypeSystemDefinition},
};
use sqlx::PgPool;

use super::harness::schema_for;
use anima_hub::sessions::EndReason;

/// Types T1 builds whole; `Query` is checked for `sessions` alone.
const WHOLE: [&str; 7] = [
    "Session",
    "SessionEdge",
    "SessionConnection",
    "SessionEndReason",
    "Protocol",
    "HostKey",
    "PageInfo",
];

/// For each object or enum, member name → signature (sorted arguments and type).
fn members(sdl: &str) -> BTreeMap<String, BTreeMap<String, String>> {
    let document = parse_schema(sdl).expect("SDL parses");
    let mut types = BTreeMap::new();
    for definition in document.definitions {
        let TypeSystemDefinition::Type(definition) = definition else {
            continue;
        };
        let definition = definition.node;
        let signatures: BTreeMap<String, String> = match definition.kind {
            TypeKind::Object(object) => object
                .fields
                .into_iter()
                .map(|field| {
                    let field = field.node;
                    let mut args: Vec<_> =
                        field.arguments.iter().map(|a| argument(&a.node)).collect();
                    args.sort();
                    (
                        field.name.node.to_string(),
                        format!("({}): {}", args.join(", "), field.ty.node),
                    )
                })
                .collect(),
            TypeKind::Enum(enumeration) => enumeration
                .values
                .into_iter()
                .map(|value| (value.node.value.node.to_string(), String::new()))
                .collect(),
            _ => continue,
        };
        types.insert(definition.name.node.to_string(), signatures);
    }
    types
}

fn argument(input: &InputValueDefinition) -> String {
    let default = input
        .default_value
        .as_ref()
        .map_or_else(String::new, |value| format!(" = {}", value.node));
    format!("{}: {}{default}", input.name.node, input.ty.node)
}

fn k1() -> BTreeMap<String, BTreeMap<String, String>> {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../contracts/schema.graphql"
    );
    members(&std::fs::read_to_string(path).unwrap())
}

fn hub() -> BTreeMap<String, BTreeMap<String, String>> {
    // The schema never touches the database for its SDL, so a lazy pool is enough.
    let pool = PgPool::connect_lazy("postgres://unused@127.0.0.1:1/none").unwrap();
    members(&schema_for(&pool).sdl())
}

#[tokio::test]
async fn k1_session_types_and_host_key_equal_k1() {
    let (hub, k1) = (hub(), k1());
    for name in WHOLE {
        assert_eq!(hub.get(name), k1.get(name), "{name}");
    }
    assert_eq!(
        hub["Query"].keys().collect::<Vec<_>>(),
        ["sessions"],
        "T1 declares one root field"
    );
    assert_eq!(hub["Query"]["sessions"], k1["Query"]["sessions"]);
}

#[test]
fn ra_008_every_k1_end_reason_is_stored_as_its_lower_snake_name() {
    let k1 = k1();
    let names: Vec<String> = k1["SessionEndReason"]
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
