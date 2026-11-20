//! The keys that a file may have, at each level (K10 Section 6.4.4). A key that is not here is
//! refused: thus `include`, `extends`, `env_file`, `profiles`, `build`, `name`, `privileged`
//! and each key of a newer Compose are refused with no rule of their own.

use crate::{Node, Refusal, Rule};

/// The form of a value.
pub(crate) enum Shape {
    /// One scalar.
    Text,
    /// One scalar, or a list of scalars.
    Texts,
    /// A map of names to scalars, or a list of scalars.
    Pairs,
    /// A map with these keys and no other.
    Record(&'static [(&'static str, &'static Shape)]),
    /// A map of names of the file to values of one form.
    Named(&'static Shape),
    /// A list of values of one form.
    Each(&'static Shape),
    /// A list of names, or a map of names to values of one form.
    NamesOr(&'static Shape),
}

const NOTHING: Shape = Shape::Record(&[]);
const NETWORK: Shape = Shape::Record(&[("driver", &Shape::Text), ("internal", &Shape::Text)]);
const CONFIG: Shape = Shape::Record(&[("content", &Shape::Text)]);
const HEALTHCHECK: Shape = Shape::Record(&[
    ("test", &Shape::Texts),
    ("interval", &Shape::Text),
    ("timeout", &Shape::Text),
    ("retries", &Shape::Text),
    ("start_period", &Shape::Text),
    ("start_interval", &Shape::Text),
    ("disable", &Shape::Text),
]);
const DEPENDENCY: Shape = Shape::Record(&[
    ("condition", &Shape::Text),
    ("restart", &Shape::Text),
    ("required", &Shape::Text),
]);
const LOGGING: Shape = Shape::Record(&[("driver", &Shape::Text)]);
const PORT: Shape = Shape::Record(&[
    ("target", &Shape::Text),
    ("published", &Shape::Text),
    ("host_ip", &Shape::Text),
    ("protocol", &Shape::Text),
]);
const MOUNT: Shape = Shape::Record(&[
    ("type", &Shape::Text),
    ("source", &Shape::Text),
    ("target", &Shape::Text),
    ("read_only", &Shape::Text),
]);
const ATTACHMENT: Shape = Shape::Record(&[("aliases", &Shape::Texts)]);
const CONFIG_MOUNT: Shape = Shape::Record(&[
    ("source", &Shape::Text),
    ("target", &Shape::Text),
    ("mode", &Shape::Text),
]);
const SERVICE: Shape = Shape::Record(&[
    ("image", &Shape::Text),
    ("command", &Shape::Texts),
    ("entrypoint", &Shape::Texts),
    ("environment", &Shape::Pairs),
    ("restart", &Shape::Text),
    ("depends_on", &Shape::NamesOr(&DEPENDENCY)),
    ("healthcheck", &HEALTHCHECK),
    ("user", &Shape::Text),
    ("working_dir", &Shape::Text),
    ("hostname", &Shape::Text),
    ("stop_signal", &Shape::Text),
    ("stop_grace_period", &Shape::Text),
    ("read_only", &Shape::Text),
    ("tmpfs", &Shape::Texts),
    ("init", &Shape::Text),
    ("cap_drop", &Shape::Texts),
    ("expose", &Shape::Texts),
    ("mem_limit", &Shape::Text),
    ("cpus", &Shape::Text),
    ("pids_limit", &Shape::Text),
    ("labels", &Shape::Pairs),
    ("logging", &LOGGING),
    ("security_opt", &Shape::Texts),
    ("ports", &Shape::Each(&PORT)),
    ("volumes", &Shape::Each(&MOUNT)),
    ("networks", &Shape::NamesOr(&ATTACHMENT)),
    ("configs", &Shape::Each(&CONFIG_MOUNT)),
]);
const FILE: Shape = Shape::Record(&[
    ("services", &Shape::Named(&SERVICE)),
    ("volumes", &Shape::Named(&NOTHING)),
    ("networks", &Shape::Named(&NETWORK)),
    ("configs", &Shape::Named(&CONFIG)),
]);

/// Where the walk is: the position of the service, when it is in one.
#[derive(Clone, Copy)]
struct Place(Option<usize>);

impl Place {
    const fn refusal(self, rule: Rule) -> Refusal {
        Refusal::File {
            rule,
            service: self.0,
        }
    }
}

fn scalars(items: &[Node]) -> bool {
    items.iter().all(|item| matches!(item, Node::Text(_)))
}

fn record(node: &Node, keys: &[(&str, &Shape)], place: Place) -> Result<(), Refusal> {
    let entries = node
        .entries()
        .ok_or(place.refusal(Rule::ValueNotPermitted))?;
    for (key, value) in entries {
        let (_, shape) = keys
            .iter()
            .find(|(name, _)| name == key)
            .ok_or(place.refusal(Rule::KeyNotPermitted))?;
        walk(value, shape, place)?;
    }
    Ok(())
}

fn named(node: &Node, shape: &Shape, place: Place, services: bool) -> Result<(), Refusal> {
    let entries = node
        .entries()
        .ok_or(place.refusal(Rule::ValueNotPermitted))?;
    for (index, (_, value)) in entries.iter().enumerate() {
        let place = if services { Place(Some(index)) } else { place };
        walk(value, shape, place)?;
    }
    Ok(())
}

fn walk(node: &Node, shape: &Shape, place: Place) -> Result<(), Refusal> {
    let wrong = place.refusal(Rule::ValueNotPermitted);
    match (shape, node) {
        (Shape::Text, Node::Text(_) | Node::Null) | (Shape::Texts, Node::Text(_)) => Ok(()),
        (Shape::Texts | Shape::Pairs | Shape::NamesOr(_), Node::List(items)) if scalars(items) => {
            Ok(())
        }
        (Shape::Pairs, Node::Map(entries)) => {
            if entries
                .iter()
                .all(|(_, value)| !matches!(value, Node::List(_) | Node::Map(_)))
            {
                Ok(())
            } else {
                Err(wrong)
            }
        }
        (Shape::Record(keys), _) => record(node, keys, place),
        (Shape::Named(inner) | Shape::NamesOr(inner), Node::Map(_) | Node::Null) => {
            named(node, inner, place, false)
        }
        (Shape::Each(inner), Node::List(items)) => {
            items.iter().try_for_each(|item| walk(item, inner, place))
        }
        _ => Err(wrong),
    }
}

/// Checks that `document` has only keys of the list, each with a value of its form.
///
/// # Errors
/// `key_not_permitted` for a key that is not in the list, at any level;
/// `value_not_permitted` for a value of a different form, also a short form of a port, a
/// mount or a config.
pub(crate) fn permitted(document: &Node) -> Result<(), Refusal> {
    let top = Place(None);
    let entries = document
        .entries()
        .ok_or(top.refusal(Rule::ValueNotPermitted))?;
    let Shape::Record(keys) = FILE else {
        return Err(top.refusal(Rule::KeyNotPermitted));
    };
    for (key, value) in entries {
        let (_, shape) = keys
            .iter()
            .find(|(name, _)| name == key)
            .ok_or(top.refusal(Rule::KeyNotPermitted))?;
        match shape {
            Shape::Named(inner) => named(value, inner, top, key == "services")?,
            other => walk(value, other, top)?,
        }
    }
    Ok(())
}
