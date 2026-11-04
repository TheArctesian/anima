//! Error texts that echo what the sender wrote, and the fixed texts that replace them
//! (H-6 (e) and (f), LOG-007, K7).
//!
//! async-graphql quotes the query line in a syntax error, an input value or an unknown input
//! field name in a validation error, and the value in an argument it could not coerce. Any of
//! them may be a password typed inline. Each is replaced by a fixed text; only the code, the
//! position and the schema names of the argument path stay.

use std::collections::BTreeMap;

use async_graphql::{
    ErrorExtensionValues, Pos, Positioned, ServerError,
    parser::types::{ExecutableDocument, Selection, SelectionSet},
    value,
};

pub(super) const SYNTAX_MESSAGE: &str = "The request is not valid GraphQL.";
pub(super) const ARGUMENT_MESSAGE: &str = "An argument has the wrong type or shape.";
const CODE: &str = "code";
const VALIDATION: &str = "VALIDATION";

/// How async-graphql 7.2 begins a validation error about an argument value
/// (`ArgumentsOfCorrectType`, `DefaultValuesOfCorrectType`). The rest is the argument path in
/// quotes, then a reason that can quote the value or an unknown input field name.
const VALUE_PREFIXES: [&str; 2] = [
    "Invalid value for argument ",
    "Invalid default value for argument: ",
];

/// How async-graphql 7.2 begins the message of an argument it could not coerce while
/// resolving (`InputValueError::expected_type` and `custom`). Used only when the request's
/// document is unknown, so the error cannot be placed by position.
const COERCION_PREFIXES: [&str; 2] = [r#"Expected input type ""#, r#"Failed to parse ""#];

/// Where each argument value of the document starts, and the argument's name.
pub(super) type Arguments = BTreeMap<Pos, String>;

/// Replaces a syntax error's text, which quotes the query line, keeping its position.
pub(super) fn hide_syntax(error: &mut ServerError) {
    SYNTAX_MESSAGE.clone_into(&mut error.message);
}

/// Whether `message` is a validation error about an argument value.
pub(super) fn is_value_error(message: &str) -> bool {
    VALUE_PREFIXES
        .iter()
        .any(|prefix| message.starts_with(prefix))
}

/// The argument path of a value error: the quoted part after the prefix. It is made of schema
/// names and list indexes only, never of input, so it can be shown.
fn value_path(message: &str) -> &str {
    VALUE_PREFIXES
        .iter()
        .find_map(|prefix| message.strip_prefix(prefix))
        .and_then(|rest| rest.strip_prefix('"'))
        .and_then(|rest| rest.split_once('"'))
        .map_or("", |(path, _)| path)
}

/// Turns a value error into `VALIDATION` with a fixed text naming the argument path.
pub(super) fn hide_value(error: &mut ServerError) {
    let field = value_path(&error.message).to_owned();
    refuse(error, &field);
}

/// Whether a resolver error is an argument async-graphql could not coerce. Such an error is
/// placed at the argument's value, while a resolver's own error is placed at the field, so
/// the position tells them apart whatever the text. Without the document, the text decides.
pub(super) fn coerced_argument<'a>(
    error: &ServerError,
    arguments: Option<&'a Arguments>,
) -> Option<&'a str> {
    match arguments {
        Some(arguments) => error
            .locations
            .first()
            .and_then(|pos| arguments.get(pos))
            .map(String::as_str),
        None => COERCION_PREFIXES
            .iter()
            .any(|prefix| error.message.starts_with(prefix))
            .then_some(""),
    }
}

/// `VALIDATION` with the fixed argument text, whatever code and text the error had: a parser
/// may set its own code and still quote the value (COULD 2).
pub(super) fn refuse(error: &mut ServerError, field: &str) {
    ARGUMENT_MESSAGE.clone_into(&mut error.message);
    error.source = None;
    let extensions = error.extensions.insert(ErrorExtensionValues::default());
    extensions.set(CODE, VALIDATION);
    extensions.set(
        "fields",
        value!([{ "field": field, "message": ARGUMENT_MESSAGE }]),
    );
}

/// Every argument value in the document's operations and fragments, by position.
pub(super) fn arguments(document: &ExecutableDocument) -> Arguments {
    let mut found = Arguments::new();
    let mut pending: Vec<&SelectionSet> = document
        .operations
        .iter()
        .map(|(_, operation)| &operation.node.selection_set.node)
        .chain(
            document
                .fragments
                .values()
                .map(|fragment| &fragment.node.selection_set.node),
        )
        .collect();
    while let Some(set) = pending.pop() {
        pending.extend(set.items.iter().filter_map(|item| nested(item, &mut found)));
    }
    found
}

/// Records a field's argument positions and returns the selection set inside the selection;
/// a spread has none here, its fragment is walked on its own.
fn nested<'a>(
    selection: &'a Positioned<Selection>,
    found: &mut Arguments,
) -> Option<&'a SelectionSet> {
    match &selection.node {
        Selection::Field(field) => {
            for (name, value) in &field.node.arguments {
                found.insert(value.pos, name.node.to_string());
            }
            Some(&field.node.selection_set.node)
        }
        Selection::InlineFragment(fragment) => Some(&fragment.node.selection_set.node),
        Selection::FragmentSpread(_) => None,
    }
}
