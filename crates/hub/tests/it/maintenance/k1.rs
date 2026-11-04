//! The maintenance part of the hub schema is a subset of K1, and covers every K1 maintenance
//! member (R-011(c)).

use std::collections::BTreeMap;

use async_graphql::parser::{
    parse_schema,
    types::{InputValueDefinition, ServiceDocument, TypeKind, TypeSystemDefinition},
};

use super::harness::schema_for;

const CHECKED: [&str; 9] = [
    "Query",
    "Mutation",
    "WindowKind",
    "Weekday",
    "TargetType",
    "WindowTarget",
    "MaintenanceWindow",
    "WindowTargetInput",
    "MaintenanceWindowInput",
];

/// The K1 root fields the maintenance part builds; the others belong to other packages.
const OWNED_ROOT: [&str; 5] = [
    "maintenanceWindows",
    "createMaintenanceWindow",
    "updateMaintenanceWindow",
    "deleteMaintenanceWindow",
    "setManualWindowActive",
];

/// For each named type, member name to signature (arguments, type, default).
type Members = BTreeMap<String, BTreeMap<String, String>>;

fn members(document: &ServiceDocument) -> Members {
    let mut types = BTreeMap::new();
    for definition in &document.definitions {
        let TypeSystemDefinition::Type(definition) = definition else {
            continue;
        };
        let definition = &definition.node;
        let mut signatures = BTreeMap::new();
        match &definition.kind {
            TypeKind::Object(object) => {
                for field in &object.fields {
                    let mut args: Vec<_> = field
                        .node
                        .arguments
                        .iter()
                        .map(|arg| signature(&arg.node))
                        .collect();
                    args.sort();
                    signatures.insert(
                        field.node.name.node.to_string(),
                        format!("({}): {}", args.join(", "), field.node.ty.node),
                    );
                }
            }
            TypeKind::InputObject(input) => {
                for field in &input.fields {
                    signatures.insert(field.node.name.node.to_string(), signature(&field.node));
                }
            }
            TypeKind::Enum(enumeration) => {
                for value in &enumeration.values {
                    signatures.insert(value.node.value.node.to_string(), String::new());
                }
            }
            _ => continue,
        }
        types.insert(definition.name.node.to_string(), signatures);
    }
    types
}

fn signature(input: &InputValueDefinition) -> String {
    let default = input
        .default_value
        .as_ref()
        .map_or_else(String::new, |value| format!(" = {}", value.node));
    format!("{}: {}{default}", input.name.node, input.ty.node)
}

fn hub_and_k1() -> (Members, Members) {
    // The schema never touches the database for its SDL, so a lazy pool is enough.
    let pool = sqlx::PgPool::connect_lazy("postgres://unused@127.0.0.1:1/none").unwrap();
    let hub = members(&parse_schema(schema_for(&pool).sdl()).expect("hub sdl parses"));
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../contracts/schema.graphql"
    );
    let k1 = members(&parse_schema(std::fs::read_to_string(path).unwrap()).expect("K1 parses"));
    (hub, k1)
}

#[tokio::test]
async fn k1_every_maintenance_member_of_the_hub_schema_exists_in_k1_with_the_same_type() {
    let (hub, k1) = hub_and_k1();
    for name in CHECKED {
        let ours = hub
            .get(name)
            .unwrap_or_else(|| panic!("hub declares {name}"));
        let theirs = k1.get(name).unwrap_or_else(|| panic!("K1 declares {name}"));
        assert!(!ours.is_empty(), "{name} has members");
        for (member, ours) in ours {
            assert_eq!(theirs.get(member), Some(ours), "{name}.{member}");
        }
    }
    let root: Vec<&String> = hub["Query"].keys().chain(hub["Mutation"].keys()).collect();
    assert_eq!(root.len(), OWNED_ROOT.len(), "{root:?}");
}

#[tokio::test]
async fn k1_every_maintenance_member_in_k1_is_in_the_hub() {
    let (hub, k1) = hub_and_k1();
    let mut missing = Vec::new();
    for name in CHECKED {
        let is_root = matches!(name, "Query" | "Mutation");
        for member in k1[name].keys() {
            if is_root && !OWNED_ROOT.contains(&member.as_str()) {
                continue;
            }
            if !hub[name].contains_key(member) {
                missing.push(format!("{name}.{member}"));
            }
        }
    }
    assert!(
        missing.is_empty(),
        "K1 members missing from the hub: {missing:?}"
    );
}
