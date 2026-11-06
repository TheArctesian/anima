//! The hub schema and K1 declare the same types, member for member (CON-002, R-011(c)).

mod pending;
#[cfg(test)]
mod tests;

use std::collections::BTreeMap;

use async_graphql::parser::{
    Positioned, parse_schema,
    types::{
        ConstDirective, DirectiveDefinition, EnumType, FieldDefinition, InputObjectType,
        InputValueDefinition, TypeDefinition, TypeKind, TypeSystemDefinition,
    },
};

use self::pending::{FieldParts, Pending, Variants, is_pending, is_pending_directive};
use crate::hub::hub_sdl;

/// One declaration: what kind it is (with directives, union members and interfaces), then each
/// member's name → signature, and what K1 marks `@pending` in it.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Declaration {
    header: String,
    pub members: BTreeMap<String, String>,
    pending: Pending,
    fields: BTreeMap<String, FieldParts>,
    variants: Option<Variants>,
}

impl Declaration {
    /// The member names the hub must serve today: those K1 does not mark `@pending` (P2-002).
    pub fn served_members(&self) -> impl Iterator<Item = &String> {
        self.members
            .keys()
            .filter(|member| !self.pending.members.contains(*member))
    }
}

/// Every named declaration of an SDL document, by name. Directive definitions appear as
/// `@name`.
pub type Schema = BTreeMap<String, Declaration>;

/// Directives are compared by name only, apart from `@pending`, which is not compared at all.
/// K1 uses no directive arguments, `extend` blocks or `schema {}` block today; add them here if
/// it ever does.
fn directives(list: &[Positioned<ConstDirective>]) -> String {
    let mut names: Vec<String> = list
        .iter()
        .filter(|d| !is_pending_directive(d))
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

fn field_parts(field: &FieldDefinition) -> FieldParts {
    let mut arguments: Vec<_> = field
        .arguments
        .iter()
        .map(|a| (a.node.name.node.to_string(), argument(&a.node)))
        .collect();
    arguments.sort_by(|a, b| a.1.cmp(&b.1));
    FieldParts {
        arguments,
        pending: (field.arguments.iter())
            .filter(|a| is_pending(&a.node.directives))
            .map(|a| a.node.name.node.to_string())
            .collect(),
        result: format!("{}{}", field.ty.node, directives(&field.directives)),
    }
}

fn field_members(fields: &[Positioned<FieldDefinition>]) -> Declaration {
    let mut declaration = Declaration::default();
    for field in fields {
        let name = field.node.name.node.to_string();
        let parts = field_parts(&field.node);
        if is_pending(&field.node.directives) {
            declaration.pending.members.insert(name.clone());
        }
        declaration
            .members
            .insert(name.clone(), parts.signature_without(&parts.pending));
        declaration.fields.insert(name, parts);
    }
    declaration
}

fn enum_members(enumeration: &EnumType) -> Declaration {
    let mut declaration = Declaration::default();
    for value in &enumeration.values {
        let name = value.node.value.node.to_string();
        if is_pending(&value.node.directives) {
            declaration.pending.members.insert(name.clone());
        }
        declaration.members.insert(name, String::new());
    }
    declaration
}

fn input_members(input: &InputObjectType) -> Declaration {
    let mut declaration = Declaration::default();
    for field in &input.fields {
        let name = field.node.name.node.to_string();
        if is_pending(&field.node.directives) {
            declaration.pending.members.insert(name.clone());
        }
        declaration.members.insert(name, argument(&field.node));
    }
    declaration
}

fn type_declaration(definition: &TypeDefinition) -> Declaration {
    let suffix = directives(&definition.directives);
    let (kind, members) = match &definition.kind {
        TypeKind::Scalar => ("scalar".to_owned(), Declaration::default()),
        TypeKind::Object(object) => {
            let mut implements: Vec<_> = object
                .implements
                .iter()
                .map(|n| n.node.to_string())
                .collect();
            implements.sort();
            (
                format!("type implements [{}]", implements.join(", ")),
                field_members(&object.fields),
            )
        }
        TypeKind::Interface(interface) => {
            ("interface".to_owned(), field_members(&interface.fields))
        }
        TypeKind::Union(union) => {
            let mut names: Vec<_> = union.members.iter().map(|n| n.node.to_string()).collect();
            names.sort();
            let variants = Variants {
                names,
                directives: suffix.clone(),
            };
            (
                format!("union = {}", variants.names.join(" | ")),
                Declaration {
                    variants: Some(variants),
                    ..Declaration::default()
                },
            )
        }
        TypeKind::Enum(enumeration) => ("enum".to_owned(), enum_members(enumeration)),
        TypeKind::InputObject(input) => ("input".to_owned(), input_members(input)),
    };
    let mut declaration = Declaration {
        header: format!("{kind}{suffix}"),
        ..members
    };
    declaration.pending.declaration = is_pending(&definition.directives);
    declaration
}

fn directive_declaration(directive: &DirectiveDefinition) -> Declaration {
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
    Declaration {
        header: format!(
            "directive ({}) on {}",
            args.join(", "),
            locations.join(" | ")
        ),
        // The `@pending` definition is K1-only: the hub is never required to declare it.
        pending: Pending {
            declaration: directive.name.node.as_str() == "pending",
            ..Pending::default()
        },
        ..Declaration::default()
    }
}

/// Parses an SDL into its declarations: objects, interfaces, inputs, enums, unions, scalars and
/// directive definitions; arguments are sorted, so declaration order never matters.
pub fn schema(sdl: &str) -> Schema {
    let document = parse_schema(sdl).expect("SDL parses");
    let mut declarations = Schema::new();
    for definition in &document.definitions {
        match definition {
            TypeSystemDefinition::Type(definition) => {
                let name = definition.node.name.node.to_string();
                let previous =
                    declarations.insert(name.clone(), type_declaration(&definition.node));
                assert!(previous.is_none(), "duplicate {name}");
            }
            TypeSystemDefinition::Directive(directive) => {
                let name = format!("@{}", directive.node.name.node);
                let previous =
                    declarations.insert(name.clone(), directive_declaration(&directive.node));
                assert!(previous.is_none(), "duplicate {name}");
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

/// Every way `hub` and `k1` differ, one line each. A name or member K1 marks `@pending` counts
/// only once the hub declares it.
fn differences(hub: &Schema, k1: &Schema) -> Vec<String> {
    let mut differences = Vec::new();
    for name in hub.keys().filter(|n| !k1.contains_key(*n)) {
        differences.push(format!("{name}: declared by the hub, not in K1"));
    }
    for (name, theirs) in k1 {
        let Some(ours) = hub.get(name) else {
            if !theirs.pending.declaration {
                differences.push(format!("{name}: declared by K1, not in the hub"));
            }
            continue;
        };
        if theirs.pending.declaration {
            differences.push(format!("{name}: declared by the hub, still @pending in K1"));
            continue;
        }
        let theirs_header = pending::k1_header(theirs, hub, k1);
        if ours.header != theirs_header {
            differences.push(format!(
                "{name}: hub `{}`, K1 `{theirs_header}`",
                ours.header
            ));
        }
        differences.extend(member_differences(name, ours, theirs));
    }
    differences
}

fn member_differences(name: &str, ours: &Declaration, theirs: &Declaration) -> Vec<String> {
    let mut differences = Vec::new();
    for member in (ours.members.keys()).filter(|m| !theirs.members.contains_key(*m)) {
        differences.push(format!("{name}.{member}: in the hub, not in K1"));
    }
    for member in theirs.members.keys() {
        let is_pending = theirs.pending.members.contains(member);
        match (ours.members.get(member), is_pending) {
            (None, true) => {}
            (None, false) => differences.push(format!("{name}.{member}: in K1, not in the hub")),
            (Some(_), true) => {
                differences.push(format!("{name}.{member}: in the hub, still @pending in K1"));
            }
            (Some(_), false) => {
                differences.extend(pending::signature_differences(name, member, ours, theirs));
            }
        }
    }
    differences
}
