//! Tests of the syntax check of a Compose file and of the `docker run` converter (HOST-022):
//! good files and each problem (`syntax`), hostile text (`hostile`), what a problem and a
//! result may hold (`secrecy`), the words of a shell line (`words`), commands as people write
//! them (`commands`, `real`, `options`), each refusal (`refused`), and the writer (`written`).

mod commands;
mod hostile;
mod options;
mod real;
mod refused;
mod secrecy;
mod syntax;
mod words;
mod written;

use super::{
    Converted, ProblemCode, check, convert,
    yaml::{self, Node, Shape},
};

/// The codes of the problems of `text`, in their order.
fn codes(text: &str) -> Vec<ProblemCode> {
    let problems = check(text).problems;
    problems.iter().map(|problem| problem.code).collect()
}

/// The Compose file of `command`, which must pass the syntax check with no problem at all.
fn converted(command: &str) -> Converted {
    let done = convert(command).unwrap_or_else(|refusal| panic!("{command}: {refusal:?}"));
    let found = check(&done.compose_file);
    assert_eq!(found.problems, [], "{command}\n{}", done.compose_file);
    done
}

/// The one document of `text` as the reader of the check reads it.
fn tree(text: &str) -> Node {
    yaml::read(text).expect("YAML").expect("a document")
}

/// The node at `path`: a key of a map, or the index of an entry of a list.
fn at<'a>(node: &'a Node, path: &[&str]) -> &'a Node {
    path.iter().fold(node, |node, step| match &node.shape {
        Shape::Map(entries) => entries
            .iter()
            .find(|(key, _)| key.shape == Shape::Text((*step).to_owned()))
            .map_or_else(|| panic!("no key {step}"), |(_, value)| value),
        Shape::List(entries) => &entries[step.parse::<usize>().expect("an index")],
        other => panic!("{step} of {other:?}"),
    })
}

/// The text of the scalar at `path`.
fn text<'a>(node: &'a Node, path: &[&str]) -> &'a str {
    match &at(node, path).shape {
        Shape::Text(text) => text,
        other => panic!("{path:?} is {other:?}"),
    }
}

/// The texts of the list at `path`.
fn list<'a>(node: &'a Node, path: &[&str]) -> Vec<&'a str> {
    let Shape::List(entries) = &at(node, path).shape else {
        panic!("{path:?} is not a list");
    };
    let scalar = |entry: &'a Node| match &entry.shape {
        Shape::Text(text) => text.as_str(),
        other => panic!("{other:?}"),
    };
    entries.iter().map(scalar).collect()
}

/// The keys of the map at `path`.
fn keys<'a>(node: &'a Node, path: &[&str]) -> Vec<&'a str> {
    let Shape::Map(entries) = &at(node, path).shape else {
        panic!("{path:?} is not a map");
    };
    let name = |(key, _): &'a (Node, Node)| match &key.shape {
        Shape::Text(text) => text.as_str(),
        other => panic!("{other:?}"),
    };
    entries.iter().map(name).collect()
}
