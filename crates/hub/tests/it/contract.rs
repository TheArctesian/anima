//! The hub schema and K1 declare the same types, member for member (CON-002, R-011(c)).

use std::collections::BTreeMap;

use async_graphql::parser::{
    Positioned, parse_schema,
    types::{ConstDirective, InputValueDefinition, TypeKind, TypeSystemDefinition},
};

use crate::hub::hub_sdl;

/// One declaration: what kind it is (with directives, union members and interfaces), then each
/// member's name → signature.
#[derive(Debug, PartialEq, Eq)]
pub struct Declaration {
    pub header: String,
    pub members: BTreeMap<String, String>,
}

/// Every named declaration of an SDL document, by name. Directive definitions appear as
/// `@name`.
pub type Schema = BTreeMap<String, Declaration>;

fn directives(list: &[Positioned<ConstDirective>]) -> String {
    let mut names: Vec<String> = list
        .iter()
        .map(|d| format!(" @{}", d.node.name.node))
        .collect();
    names.sort();
    names.concat()
}

fn argument(input: &InputValueDefinition) -> String {
    let default = input
        .default_value
        .as_ref()
        .map_or_else(String::new, |value| format!(" = {}", value.node));
    format!(
        "{}: {}{default}{}",
        input.name.node,
        input.ty.node,
        directives(&input.directives)
    )
}

fn field_signatures(
    fields: &[Positioned<async_graphql::parser::types::FieldDefinition>],
) -> BTreeMap<String, String> {
    fields
        .iter()
        .map(|field| {
            let field = &field.node;
            let mut args: Vec<_> = field.arguments.iter().map(|a| argument(&a.node)).collect();
            args.sort();
            (
                field.name.node.to_string(),
                format!(
                    "({}): {}{}",
                    args.join(", "),
                    field.ty.node,
                    directives(&field.directives)
                ),
            )
        })
        .collect()
}

/// Parses an SDL into its declarations: objects, interfaces, inputs, enums, unions, scalars and
/// directive definitions; arguments are sorted, so declaration order never matters.
pub fn schema(sdl: &str) -> Schema {
    let document = parse_schema(sdl).expect("SDL parses");
    let mut declarations = Schema::new();
    for definition in &document.definitions {
        match definition {
            TypeSystemDefinition::Type(definition) => {
                let definition = &definition.node;
                let (kind, members) = match &definition.kind {
                    TypeKind::Scalar => ("scalar".to_owned(), BTreeMap::new()),
                    TypeKind::Object(object) => {
                        let mut implements: Vec<_> = object
                            .implements
                            .iter()
                            .map(|n| n.node.to_string())
                            .collect();
                        implements.sort();
                        (
                            format!("type implements [{}]", implements.join(", ")),
                            field_signatures(&object.fields),
                        )
                    }
                    TypeKind::Interface(interface) => {
                        ("interface".to_owned(), field_signatures(&interface.fields))
                    }
                    TypeKind::Union(union) => {
                        let mut names: Vec<_> =
                            union.members.iter().map(|n| n.node.to_string()).collect();
                        names.sort();
                        (format!("union = {}", names.join(" | ")), BTreeMap::new())
                    }
                    TypeKind::Enum(enumeration) => (
                        "enum".to_owned(),
                        enumeration
                            .values
                            .iter()
                            .map(|v| (v.node.value.node.to_string(), String::new()))
                            .collect(),
                    ),
                    TypeKind::InputObject(input) => (
                        "input".to_owned(),
                        input
                            .fields
                            .iter()
                            .map(|f| (f.node.name.node.to_string(), argument(&f.node)))
                            .collect(),
                    ),
                };
                let header = format!("{kind}{}", directives(&definition.directives));
                declarations.insert(
                    definition.name.node.to_string(),
                    Declaration { header, members },
                );
            }
            TypeSystemDefinition::Directive(directive) => {
                let directive = &directive.node;
                let mut locations: Vec<_> = directive
                    .locations
                    .iter()
                    .map(|l| format!("{:?}", l.node))
                    .collect();
                locations.sort();
                let mut args: Vec<_> = directive
                    .arguments
                    .iter()
                    .map(|a| argument(&a.node))
                    .collect();
                args.sort();
                declarations.insert(
                    format!("@{}", directive.name.node),
                    Declaration {
                        header: format!(
                            "directive ({}) on {}",
                            args.join(", "),
                            locations.join(" | ")
                        ),
                        members: BTreeMap::new(),
                    },
                );
            }
            TypeSystemDefinition::Schema(_) => {}
        }
    }
    declarations
}

pub fn k1() -> Schema {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../contracts/schema.graphql"
    );
    schema(&std::fs::read_to_string(path).unwrap())
}

/// What async-graphql emits and K1 leaves implicit: the built-in scalars and the directives the
/// GraphQL specification defines for every server. Nothing else is normalised, and a name K1
/// does declare is always compared.
const EMITTED_NOT_DECLARED: [&str; 8] = [
    "Boolean",
    "Float",
    "ID",
    "Int",
    "String",
    "@specifiedBy",
    "@include",
    "@skip",
];

/// The hub's schema without the declarations the library emits itself.
fn hub(k1: &Schema) -> Schema {
    let mut hub = schema(&hub_sdl());
    for name in EMITTED_NOT_DECLARED {
        if !k1.contains_key(name) {
            hub.remove(name);
        }
    }
    hub
}

/// Every way `hub` and `k1` differ, one line each.
fn differences(hub: &Schema, k1: &Schema) -> Vec<String> {
    let mut differences = Vec::new();
    for name in hub.keys().filter(|n| !k1.contains_key(*n)) {
        differences.push(format!("{name}: declared by the hub, not in K1"));
    }
    for (name, theirs) in k1 {
        let Some(ours) = hub.get(name) else {
            differences.push(format!("{name}: declared by K1, not in the hub"));
            continue;
        };
        if ours.header != theirs.header {
            differences.push(format!(
                "{name}: hub `{}`, K1 `{}`",
                ours.header, theirs.header
            ));
        }
        for member in ours
            .members
            .keys()
            .filter(|m| !theirs.members.contains_key(*m))
        {
            differences.push(format!("{name}.{member}: in the hub, not in K1"));
        }
        for (member, signature) in &theirs.members {
            match ours.members.get(member) {
                None => differences.push(format!("{name}.{member}: in K1, not in the hub")),
                Some(own) if own != signature => {
                    differences.push(format!("{name}.{member}: hub `{own}`, K1 `{signature}`"));
                }
                Some(_) => {}
            }
        }
    }
    differences
}

#[tokio::test]
async fn con_002_the_hub_schema_and_k1_declare_the_same_types_with_the_same_members() {
    let k1 = k1();
    let hub = hub(&k1);
    let differences = differences(&hub, &k1);
    assert!(
        differences.is_empty(),
        "CON-002 differences:\n{}",
        differences.join("\n")
    );
    // R-026(4): the liveness probe is `GET /health`, not a field.
    assert!(!hub["Query"].members.contains_key("health"));
    assert!(
        hub["Query"].members.contains_key("me") && hub["Mutation"].members.contains_key("login")
    );
}

#[tokio::test]
async fn con_002_a_missing_member_a_changed_signature_and_a_stray_type_are_each_reported() {
    let k1 = k1();
    let mut hub = hub(&k1);
    hub.get_mut("Device").unwrap().members.remove("name");
    hub.get_mut("Site")
        .unwrap()
        .members
        .insert("extra".into(), "(): Int".into());
    hub.get_mut("Query")
        .unwrap()
        .members
        .insert("me".into(), "(): Int".into());
    hub.insert(
        "Stray".into(),
        Declaration {
            header: "enum".into(),
            members: BTreeMap::new(),
        },
    );
    hub.remove("Mutation");

    let found = differences(&hub, &k1);
    for expected in [
        "Device.name: in K1, not in the hub",
        "Site.extra: in the hub, not in K1",
        "Stray: declared by the hub, not in K1",
        "Mutation: declared by K1, not in the hub",
    ] {
        assert!(
            found.iter().any(|d| d == expected),
            "{expected} in {found:?}"
        );
    }
    assert!(
        found
            .iter()
            .any(|d| d.starts_with("Query.me: hub `(): Int`")),
        "{found:?}"
    );
}
