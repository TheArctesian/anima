//! The devices part of the hub schema is a subset of K1 (R-011(c)).

use std::collections::BTreeMap;

use async_graphql::parser::{
    parse_schema,
    types::{ServiceDocument, TypeKind, TypeSystemDefinition},
};

use crate::hub::hub_sdl;

const CHECKED: [&str; 17] = [
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
    // The R-011 fields that need other domains are not declared yet; WP-10 has declared
    // `SshSettings.hostKey`, which the loop above checks against K1.
    for (type_name, member) in [
        ("Device", "status"),
        ("Device", "checks"),
        ("DeviceSort", "STATUS"),
    ] {
        assert!(!hub[type_name].contains_key(member), "{type_name}.{member}");
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

/// K1 members that need other domains and are not built yet: WP-09 (status, checks, uptime).
const LATER: [(&str, &str); 6] = [
    ("Device", "status"),
    ("Device", "checks"),
    ("Device", "uptime"),
    ("DeviceSort", "STATUS"),
    ("DeviceSort", "UPTIME"),
    ("DeviceFilter", "status"),
];

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
            let later = LATER.contains(&(name, member.as_str()));
            if later || (is_root && !OWNED_ROOT.contains(&member.as_str())) {
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
