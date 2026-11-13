//! CMD-031: every K1 root field is used by a document of the command line, or is in
//! `k1_exceptions.rs` with a reason. An exception for a field that is used fails too, so the
//! list only shrinks. CMD-004: every document is typed by a `query_path` in the crate.

mod k1_exceptions;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use graphql_parser::query::{Definition, OperationDefinition, Selection, SelectionSet};
use graphql_parser::schema::{self, TypeDefinition, TypeExtension};

use k1_exceptions::EXCEPTIONS;

const SCHEMA: &str = include_str!("../../../../contracts/schema.graphql");
const ROOTS: [&str; 3] = ["Query", "Mutation", "Subscription"];

/// `Type.field` for every root field of K1.
fn root_fields() -> BTreeSet<String> {
    let document = schema::parse_schema::<String>(SCHEMA).expect("K1 parses");
    let mut fields = BTreeSet::new();
    for definition in document.definitions {
        let (name, names): (String, Vec<String>) = match definition {
            schema::Definition::TypeDefinition(TypeDefinition::Object(object)) => (
                object.name,
                object.fields.into_iter().map(|field| field.name).collect(),
            ),
            schema::Definition::TypeExtension(TypeExtension::Object(object)) => (
                object.name,
                object.fields.into_iter().map(|field| field.name).collect(),
            ),
            _ => continue,
        };
        if ROOTS.contains(&name.as_str()) {
            fields.extend(names.into_iter().map(|field| format!("{name}.{field}")));
        }
    }
    fields
}

fn crate_source() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

/// Every file under `dir` with this extension.
fn files(dir: &Path, extension: &str) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir(dir).expect("a source folder") {
        let path = entry.expect("an entry").path();
        if path.is_dir() {
            found.extend(files(&path, extension));
        } else if path.extension().is_some_and(|found| found == extension) {
            found.push(path);
        }
    }
    found.sort();
    found
}

/// `Type.field` for every root field the crate's documents select, with the files using each.
fn used_fields() -> BTreeMap<String, Vec<String>> {
    let mut used: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for path in files(&crate_source(), "graphql") {
        let text = std::fs::read_to_string(&path).expect("a document");
        let document = graphql_parser::parse_query::<String>(&text)
            .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        for definition in &document.definitions {
            let Definition::Operation(operation) = definition else {
                continue;
            };
            let (root, selection) = match operation {
                OperationDefinition::Query(query) => ("Query", &query.selection_set),
                OperationDefinition::SelectionSet(set) => ("Query", set),
                OperationDefinition::Mutation(mutation) => ("Mutation", &mutation.selection_set),
                OperationDefinition::Subscription(subscription) => {
                    ("Subscription", &subscription.selection_set)
                }
            };
            for field in field_names(selection, &document.definitions) {
                let name = path
                    .file_name()
                    .expect("a name")
                    .to_string_lossy()
                    .into_owned();
                used.entry(format!("{root}.{field}"))
                    .or_default()
                    .push(name);
            }
        }
    }
    used
}

/// The field names directly in `set`, through inline fragments and fragment spreads.
fn field_names(
    set: &SelectionSet<'_, String>,
    definitions: &[Definition<'_, String>],
) -> Vec<String> {
    let mut names = Vec::new();
    for selection in &set.items {
        match selection {
            Selection::Field(field) => names.push(field.name.clone()),
            Selection::InlineFragment(fragment) => {
                names.extend(field_names(&fragment.selection_set, definitions));
            }
            Selection::FragmentSpread(spread) => {
                let fragment = definitions.iter().find_map(|definition| match definition {
                    Definition::Fragment(fragment) if fragment.name == spread.fragment_name => {
                        Some(fragment)
                    }
                    _ => None,
                });
                let fragment = fragment.expect("a spread fragment is defined in its file");
                names.extend(field_names(&fragment.selection_set, definitions));
            }
        }
    }
    names
}

#[test]
fn cmd_031_every_k1_root_field_is_used_or_excepted_with_a_reason() {
    let roots = root_fields();
    assert!(
        roots.contains("Query.devices") && roots.len() > 80,
        "K1 read: {}",
        roots.len()
    );
    let used = used_fields();
    let excepted: BTreeMap<&str, &str> = EXCEPTIONS.iter().copied().collect();
    assert_eq!(
        excepted.len(),
        EXCEPTIONS.len(),
        "an exception is listed twice"
    );

    let missing: Vec<&String> = roots
        .iter()
        .filter(|field| !used.contains_key(*field) && !excepted.contains_key(field.as_str()))
        .collect();
    assert!(
        missing.is_empty(),
        "K1 fields no command uses and no exception names: {missing:?}"
    );

    let stale: Vec<(&str, &Vec<String>)> = excepted
        .keys()
        .filter_map(|field| used.get(*field).map(|files| (*field, files)))
        .collect();
    assert!(
        stale.is_empty(),
        "now used, so remove their exceptions: {stale:?}"
    );

    let unknown: Vec<&&str> = excepted
        .keys()
        .filter(|field| !roots.contains(**field))
        .collect();
    assert!(
        unknown.is_empty(),
        "exceptions for fields K1 does not have: {unknown:?}"
    );

    let not_k1: Vec<&String> = used
        .keys()
        .filter(|field| !roots.contains(*field))
        .collect();
    assert!(
        not_k1.is_empty(),
        "documents select root fields K1 does not have: {not_k1:?}"
    );

    let without_reason: Vec<&&str> = excepted
        .iter()
        .filter(|(_, reason)| reason.trim().len() < 4)
        .map(|(field, _)| field)
        .collect();
    assert!(
        without_reason.is_empty(),
        "exceptions need a reason: {without_reason:?}"
    );
}

#[test]
fn cmd_004_every_document_is_typed_by_a_query_path() {
    let sources: String = files(&crate_source(), "rs")
        .iter()
        .map(|path| std::fs::read_to_string(path).expect("source"))
        .collect();
    let base = crate_source().parent().expect("the crate").to_owned();
    for document in files(&crate_source(), "graphql") {
        let relative = document.strip_prefix(&base).expect("under the crate");
        let declared = format!("query_path = \"{}\"", relative.display());
        assert!(
            sources.contains(&declared),
            "{} has no #[derive(GraphQLQuery)] with {declared}, so it is not checked against K1",
            relative.display()
        );
    }
}
