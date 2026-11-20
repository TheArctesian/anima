//! The check of the syntax of a Compose file (HOST-022): is the text YAML, and does it have
//! the form of a Compose file. It does not say that a machine accepts the file.

#![expect(
    clippy::unused_async_trait_impl,
    reason = "the async-graphql Enum derive generates async trait impls"
)]

use std::collections::HashSet;

use async_graphql::Enum;

use super::{
    LARGEST_FILE,
    keys::{self, Accepts},
    yaml::{self, Node, Place, Shape, Unread},
};

/// The most problems that one check gives.
pub(super) const MOST_PROBLEMS: usize = 100;
/// The most characters of a name of a service or of a key in a problem.
const LONGEST_NAME: usize = 64;

/// What is wrong, as a fixed code (K1 `ComposeFileProblemCode`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
#[graphql(name = "ComposeFileProblemCode")]
pub enum ProblemCode {
    /// The text is larger than 256 KiB.
    TooLarge,
    /// The text is not YAML.
    NotYaml,
    /// The nesting is deeper than 64 levels.
    TooDeep,
    /// The file has more nodes than the check reads.
    TooManyNodes,
    /// The text holds more than one YAML document.
    MoreThanOneDocument,
    /// The top level is not a mapping.
    TopLevelNotMapping,
    /// A key is not a text.
    KeyNotText,
    /// A mapping has the same key two times.
    DuplicateKey,
    /// The file has no `services`.
    NoServices,
    /// `services` is not a mapping.
    ServicesNotMapping,
    /// `services` holds no service.
    ServicesEmpty,
    /// A service is not a mapping.
    ServiceNotMapping,
    /// A service has neither `image` nor `build`.
    NoImageOrBuild,
    /// The value of a key of a service has a form that Compose does not accept for it.
    WrongType,
    /// The value of `restart` is not a restart policy.
    RestartNotKnown,
    /// A key of the top level that Compose does not know.
    UnknownTopLevelKey,
    /// A key of a service that Compose does not know.
    UnknownServiceKey,
    /// `version` is obsolete: Compose ignores it.
    VersionObsolete,
}

impl ProblemCode {
    /// Whether Compose refuses the file for this problem, or only ignores a part of it.
    #[must_use]
    pub const fn severity(self) -> Severity {
        match self {
            Self::UnknownTopLevelKey | Self::UnknownServiceKey | Self::VersionObsolete => {
                Severity::Warning
            }
            _ => Severity::Error,
        }
    }
}

/// How bad a problem is (K1 `ComposeFileProblemSeverity`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
#[graphql(name = "ComposeFileProblemSeverity")]
pub enum Severity {
    Error,
    Warning,
}

/// One problem of a Compose file. It holds no value of the file (LOG-007).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Problem {
    pub code: ProblemCode,
    pub place: Place,
    /// The name of the service, when the problem is in one and the name is safe to show.
    pub service: Option<String>,
    /// The name of the key, when the problem is about one and the name is safe to show.
    pub key: Option<String>,
}

impl Problem {
    const fn at(code: ProblemCode, place: Place) -> Self {
        Self {
            code,
            place,
            service: None,
            key: None,
        }
    }

    fn in_service(mut self, name: Option<&str>) -> Self {
        self.service = name.and_then(shown);
        self
    }

    fn of_key(mut self, name: &str) -> Self {
        self.key = shown(name);
        self
    }
}

/// A name as a problem shows it: letters, digits, `.`, `_` and `-` only, and cut to 64
/// characters. A name with a different character is not shown: it can be a value that the
/// operator wrote in the wrong place.
fn shown(name: &str) -> Option<String> {
    let safe = |c: char| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-');
    (!name.is_empty() && name.chars().all(safe)).then(|| name.chars().take(LONGEST_NAME).collect())
}

/// The result of the check of one file.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Check {
    /// The problems in the order of the file, errors and warnings.
    pub problems: Vec<Problem>,
    /// The file has more problems than the check gives.
    pub cut: bool,
}

impl Check {
    /// Whether the check found no error. A warning does not count.
    #[must_use]
    pub fn syntax_valid(&self) -> bool {
        !self
            .problems
            .iter()
            .any(|problem| problem.code.severity() == Severity::Error)
    }

    fn add(&mut self, problem: Problem) {
        if self.problems.len() < MOST_PROBLEMS {
            self.problems.push(problem);
        } else {
            self.cut = true;
        }
    }
}

/// One key of a mapping that is a text, with its value.
struct Entry<'a> {
    name: &'a str,
    key: &'a Node,
    value: &'a Node,
}

/// The entries of a mapping whose key is a text. A key that is not a text and a key that is
/// there two times are problems. The merge key `<<` may be there more than one time.
fn entries<'a>(
    pairs: &'a [(Node, Node)],
    service: Option<&str>,
    check: &mut Check,
) -> Vec<Entry<'a>> {
    let mut seen = HashSet::new();
    let mut found = Vec::with_capacity(pairs.len());
    for (key, value) in pairs {
        let Shape::Text(name) = &key.shape else {
            check.add(Problem::at(ProblemCode::KeyNotText, key.place).in_service(service));
            continue;
        };
        if !seen.insert(name.as_str()) && name != "<<" {
            let problem = Problem::at(ProblemCode::DuplicateKey, key.place);
            check.add(problem.in_service(service).of_key(name));
        }
        found.push(Entry { name, key, value });
    }
    found
}

/// Checks the syntax of the Compose file `text`. It runs nothing and reaches no machine.
#[must_use]
pub fn check(text: &str) -> Check {
    let mut check = Check::default();
    if text.len() > LARGEST_FILE {
        check.add(Problem::at(ProblemCode::TooLarge, Place::START));
        return check;
    }
    match yaml::read(text) {
        Ok(root) => top_level(root.as_ref(), &mut check),
        Err(unread) => check.add(unread_problem(unread)),
    }
    let place = |problem: &Problem| (problem.place.line, problem.place.column);
    check.problems.sort_by_key(place);
    check
}

const fn unread_problem(unread: Unread) -> Problem {
    match unread {
        Unread::NotYaml(place) => Problem::at(ProblemCode::NotYaml, place),
        Unread::TooDeep(place) => Problem::at(ProblemCode::TooDeep, place),
        Unread::TooManyNodes(place) => Problem::at(ProblemCode::TooManyNodes, place),
        Unread::MoreThanOneDocument(place) => Problem::at(ProblemCode::MoreThanOneDocument, place),
    }
}

fn top_level(root: Option<&Node>, check: &mut Check) {
    let Some(Shape::Map(pairs)) = root.map(|node| &node.shape) else {
        check.add(Problem::at(ProblemCode::TopLevelNotMapping, Place::START));
        return;
    };
    let found = entries(pairs, None, check);
    for entry in &found {
        match entry.name {
            "services" => services(entry, check),
            "version" => check.add(Problem::at(ProblemCode::VersionObsolete, entry.key.place)),
            known if keys::top_level(known) => {}
            other => {
                let problem = Problem::at(ProblemCode::UnknownTopLevelKey, entry.key.place);
                check.add(problem.of_key(other));
            }
        }
    }
    // A file that only includes other files has its services there.
    let has = |name: &str| found.iter().any(|entry| entry.name == name);
    if !has("services") && !has("include") {
        check.add(Problem::at(ProblemCode::NoServices, Place::START));
    }
}

fn services(entry: &Entry<'_>, check: &mut Check) {
    let code = match &entry.value.shape {
        Shape::Alias => return,
        Shape::Map(pairs) if !pairs.is_empty() => {
            for service in entries(pairs, None, check) {
                one_service(&service, check);
            }
            return;
        }
        Shape::Map(_) | Shape::Null => ProblemCode::ServicesEmpty,
        Shape::Text(_) | Shape::List(_) => ProblemCode::ServicesNotMapping,
    };
    check.add(Problem::at(code, entry.key.place));
}

fn one_service(service: &Entry<'_>, check: &mut Check) {
    let name = Some(service.name);
    let pairs = match &service.value.shape {
        Shape::Alias => return,
        Shape::Map(pairs) => pairs,
        _ => {
            let problem = Problem::at(ProblemCode::ServiceNotMapping, service.key.place);
            check.add(problem.in_service(name));
            return;
        }
    };
    let found = entries(pairs, name, check);
    for entry in &found {
        if entry.name.starts_with("x-") || entry.name == "<<" {
            continue;
        }
        let code = match keys::service(entry.name) {
            None => ProblemCode::UnknownServiceKey,
            Some(accepts) => match wrong_value(accepts, &entry.value.shape) {
                Some(code) => code,
                None => continue,
            },
        };
        let problem = Problem::at(code, entry.key.place);
        check.add(problem.in_service(name).of_key(entry.name));
    }
    // An image can come from a merge, from the service that this one extends, or from a
    // provider: the check does not follow them.
    let source = ["image", "build", "extends", "provider", "<<"];
    if !found.iter().any(|entry| source.contains(&entry.name)) {
        let problem = Problem::at(ProblemCode::NoImageOrBuild, service.key.place);
        check.add(problem.in_service(name));
    }
}

/// The problem of a value that does not have a form that its key accepts.
fn wrong_value(accepts: Accepts, shape: &Shape) -> Option<ProblemCode> {
    let fits = match (accepts, shape) {
        (Accepts::Restart, Shape::Text(policy)) => {
            return (!keys::restart_policy(policy)).then_some(ProblemCode::RestartNotKnown);
        }
        (Accepts::Any, _)
        | (_, Shape::Null | Shape::Alias)
        | (Accepts::Text | Accepts::TextOrList | Accepts::TextOrMap, Shape::Text(_))
        | (Accepts::List | Accepts::TextOrList | Accepts::ListOrMap, Shape::List(_))
        | (Accepts::Map | Accepts::TextOrMap | Accepts::ListOrMap, Shape::Map(_)) => true,
        _ => false,
    };
    (!fits).then_some(ProblemCode::WrongType)
}
