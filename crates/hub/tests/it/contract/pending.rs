//! The `@pending` rule: K1 may declare a name before the hub serves it (CON-002, D-1, P2-002).
//! While the hub lacks the name, K1's declaration is not compared; once the hub has it, the
//! comparison fails until K1 drops the mark.

use std::collections::BTreeSet;

use async_graphql::parser::{Positioned, types::ConstDirective};

use super::{Declaration, Schema};

/// True for the `@pending` directive, which never takes part in a compared header or signature.
pub fn is_pending_directive(directive: &Positioned<ConstDirective>) -> bool {
    directive.node.name.node.as_str() == "pending"
}

pub fn is_pending(directives: &[Positioned<ConstDirective>]) -> bool {
    directives.iter().any(is_pending_directive)
}

/// What K1 marks `@pending` in one declaration.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Pending {
    /// The whole declaration is pending.
    pub declaration: bool,
    /// Pending fields, input fields and enum values.
    pub members: BTreeSet<String>,
}

/// A field's arguments apart from its result type, so a pending argument can be left out.
#[derive(Debug, PartialEq, Eq)]
pub struct FieldParts {
    /// Argument name and text, sorted by text.
    pub arguments: Vec<(String, String)>,
    pub pending: BTreeSet<String>,
    /// The result type and its directives.
    pub result: String,
}

impl FieldParts {
    pub fn signature_without(&self, left_out: &BTreeSet<String>) -> String {
        let arguments: Vec<_> = self
            .arguments
            .iter()
            .filter(|(name, _)| !left_out.contains(name))
            .map(|(_, text)| text.as_str())
            .collect();
        format!("({}): {}", arguments.join(", "), self.result)
    }

    fn has(&self, argument: &str) -> bool {
        self.arguments.iter().any(|(name, _)| name == argument)
    }
}

/// A union's members and directives, so pending members can be left out of the header.
#[derive(Debug, PartialEq, Eq)]
pub struct Variants {
    pub names: Vec<String>,
    pub directives: String,
}

/// K1's header with the union members dropped whose types are `@pending` in K1 and not in the hub.
pub fn k1_header(theirs: &Declaration, hub: &Schema, k1: &Schema) -> String {
    let Some(variants) = &theirs.variants else {
        return theirs.header.clone();
    };
    let served = |name: &&String| {
        hub.contains_key(*name) || !k1.get(*name).is_some_and(|d| d.pending.declaration)
    };
    let names: Vec<_> = variants.names.iter().filter(served).cloned().collect();
    format!("union = {}{}", names.join(" | "), variants.directives)
}

/// The differences in one field's signature: the hub's arguments that K1 still marks pending,
/// then any other change, compared without the pending arguments.
pub fn signature_differences(
    name: &str,
    member: &str,
    ours: &Declaration,
    theirs: &Declaration,
) -> Vec<String> {
    let mut found = Vec::new();
    let pending_parts = theirs
        .fields
        .get(member)
        .filter(|parts| !parts.pending.is_empty());
    let own = match (pending_parts, ours.fields.get(member)) {
        (Some(pending), Some(parts)) => {
            for argument in pending.pending.iter().filter(|a| parts.has(a)) {
                found.push(format!(
                    "{name}.{member}({argument}): in the hub, still @pending in K1"
                ));
            }
            parts.signature_without(&pending.pending)
        }
        _ => ours.members[member].clone(),
    };
    let signature = &theirs.members[member];
    if own != *signature {
        found.push(format!("{name}.{member}: hub `{own}`, K1 `{signature}`"));
    }
    found
}
