//! The checks part of the hub schema is a subset of K1, and covers every K1 checks member
//! (the schema contract check).

use std::collections::BTreeMap;

use async_graphql::parser::{
    parse_schema,
    types::{InputValueDefinition, ServiceDocument, TypeKind, TypeSystemDefinition},
};

use super::harness::schema_for;

const CHECKED: [&str; 38] = [
    "Query",
    "Mutation",
    "Subscription",
    "CheckType",
    "Check",
    "CheckConfig",
    "IpVersion",
    "IcmpConfig",
    "TcpConfig",
    "TlsConfig",
    "HttpMethod",
    "HttpHeader",
    "StatusRange",
    "HttpAuthKind",
    "HttpConfig",
    "DnsRecordType",
    "DnsConfig",
    "PushConfig",
    "GroupConfig",
    "CheckConfigInput",
    "IcmpConfigInput",
    "TcpConfigInput",
    "TlsConfigInput",
    "HttpHeaderInput",
    "StatusRangeInput",
    "HttpAuthInput",
    "BasicAuthInput",
    "HttpConfigInput",
    "DnsConfigInput",
    "GroupConfigInput",
    "CreateCheckInput",
    "UpdateCheckInput",
    "Heartbeat",
    "HeartbeatEdge",
    "HeartbeatConnection",
    "UptimeWindow",
    "DeviceStatusChange",
    "HeartbeatAdded",
];

/// The K1 root fields the checks part builds; the others belong to other packages.
const OWNED_ROOT: [&str; 7] = [
    "check",
    "createCheck",
    "updateCheck",
    "deleteCheck",
    "setCheckPaused",
    "deviceStatusChanged",
    "heartbeatAdded",
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
            TypeKind::Union(union) => {
                for member in &union.members {
                    signatures.insert(member.node.to_string(), String::new());
                }
            }
            _ => continue,
        }
        // `@oneOf` marks an input whose fields are alternatives; the marker must match too.
        let one_of = definition
            .directives
            .iter()
            .any(|directive| directive.node.name.node == "oneOf");
        signatures.insert("@oneOf".to_owned(), one_of.to_string());
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
    let mut hub = members(&parse_schema(schema_for(&pool).sdl()).expect("hub sdl parses"));
    // The domain's roots are this schema's root types, under their own names.
    for (domain, root) in [
        ("ChecksQuery", "Query"),
        ("ChecksMutation", "Mutation"),
        ("ChecksSubscription", "Subscription"),
    ] {
        if let Some(fields) = hub.remove(domain) {
            hub.insert(root.to_owned(), fields);
        }
    }
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../contracts/schema.graphql"
    );
    let k1 = members(&parse_schema(std::fs::read_to_string(path).unwrap()).expect("K1 parses"));
    (hub, k1)
}

#[tokio::test]
async fn k1_every_checks_member_of_the_hub_schema_exists_in_k1_with_the_same_type() {
    let (hub, k1) = hub_and_k1();
    for name in CHECKED {
        let ours = hub
            .get(name)
            .unwrap_or_else(|| panic!("hub declares {name}"));
        let theirs = k1.get(name).unwrap_or_else(|| panic!("K1 declares {name}"));
        assert!(ours.len() > 1, "{name} has members");
        for (member, ours) in ours {
            assert_eq!(theirs.get(member), Some(ours), "{name}.{member}");
        }
    }
    let root: usize = ["Query", "Mutation", "Subscription"]
        .iter()
        .map(|name| hub[*name].len() - 1)
        .sum();
    assert_eq!(root, OWNED_ROOT.len(), "{:?}", hub["Query"]);
}

#[tokio::test]
async fn k1_every_checks_member_in_k1_is_in_the_hub() {
    let (hub, k1) = hub_and_k1();
    let mut missing = Vec::new();
    for name in CHECKED {
        let is_root = matches!(name, "Query" | "Mutation" | "Subscription");
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
