//! The hub schema is a subset of K1, and its devices part is complete (R-011(c), CON-002).

use std::collections::BTreeMap;

use async_graphql::parser::{
    parse_schema,
    types::{ServiceDocument, TypeKind, TypeSystemDefinition},
};

use crate::hub::hub_sdl;

const CHECKED: [&str; 19] = [
    "Query",
    "Mutation",
    "CreateDeviceInput",
    "UpdateDeviceInput",
    "SshSettingsInput",
    "Site",
    "Device",
    "SshSettings",
    "DeviceConnection",
    "DeviceEdge",
    "PageInfo",
    "DeviceFilter",
    "DeviceSort",
    "OperatingSystem",
    "SshAuth",
    "SshCredentialInput",
    "PrivateKeyInput",
    "Status",
    "UptimeWindow",
];

/// For each named type, member name → signature (arguments, type, default).
fn members(document: &ServiceDocument) -> BTreeMap<String, BTreeMap<String, String>> {
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

fn signature(input: &async_graphql::parser::types::InputValueDefinition) -> String {
    let default = input
        .default_value
        .as_ref()
        .map_or_else(String::new, |value| format!(" = {}", value.node));
    format!("{}: {}{default}", input.name.node, input.ty.node)
}

#[tokio::test]
async fn k1_every_devices_member_of_the_hub_schema_exists_in_k1_with_the_same_type() {
    let hub = members(&parse_schema(hub_sdl()).expect("hub sdl parses"));
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../contracts/schema.graphql"
    );
    let k1 = members(&parse_schema(std::fs::read_to_string(path).unwrap()).expect("K1 parses"));

    for name in CHECKED {
        let ours = hub
            .get(name)
            .unwrap_or_else(|| panic!("hub declares {name}"));
        let theirs = k1.get(name).unwrap_or_else(|| panic!("K1 declares {name}"));
        assert!(!ours.is_empty(), "{name} has members");
        let is_root = matches!(name, "Query" | "Mutation");
        for (member, ours) in ours {
            // The roots merge every domain; this test is about the devices members.
            if is_root && !OWNED_ROOT.contains(&member.as_str()) {
                continue;
            }
            assert_eq!(theirs.get(member), Some(ours), "{name}.{member}");
        }
    }
}

#[tokio::test]
async fn k1_ssh_credential_input_is_a_oneof_input_as_in_k1() {
    let line = "input SshCredentialInput @oneOf {";
    assert!(hub_sdl().contains(line), "the hub declares {line}");
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../contracts/schema.graphql"
    );
    assert!(std::fs::read_to_string(path).unwrap().contains(line));
}

fn k1() -> BTreeMap<String, BTreeMap<String, String>> {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../contracts/schema.graphql"
    );
    members(&parse_schema(std::fs::read_to_string(path).unwrap()).expect("K1 parses"))
}

#[tokio::test]
async fn con_002_every_member_of_the_hub_schema_exists_in_k1_with_the_same_type() {
    let hub = members(&parse_schema(hub_sdl()).expect("hub sdl parses"));
    let k1 = k1();
    assert!(hub.len() >= CHECKED.len(), "{:?}", hub.keys());
    for (name, ours) in &hub {
        let theirs = k1.get(name).unwrap_or_else(|| panic!("K1 declares {name}"));
        for (member, ours) in ours {
            assert_eq!(theirs.get(member), Some(ours), "{name}.{member}");
        }
    }
    // R-026(4): the liveness probe is `GET /health`, not a field.
    assert!(!hub["Query"].contains_key("health"));
    assert!(hub["Query"].contains_key("me") && hub["Mutation"].contains_key("login"));
}

/// The K1 root fields WP-06 builds; the others belong to other packages.
const OWNED_ROOT: [&str; 10] = [
    "sites",
    "device",
    "devices",
    "tags",
    "createSite",
    "createDevice",
    "updateDevice",
    "deleteDevice",
    "setSshCredential",
    "clearSshCredential",
];

#[tokio::test]
async fn k1_every_devices_member_wp06_owns_is_in_the_hub() {
    let hub = members(&parse_schema(hub_sdl()).expect("hub sdl parses"));
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../contracts/schema.graphql"
    );
    let k1 = members(&parse_schema(std::fs::read_to_string(path).unwrap()).expect("K1 parses"));
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
