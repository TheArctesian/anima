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

/// One problem of a Compose file. It holds no text that the operator wrote (LOG-007): a
/// service is given by its number, and a key by its name only when Compose defines that name.
/// A name that the operator chose, or wrote in the wrong place, can be a secret.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Problem {
    pub code: ProblemCode,
    pub place: Place,
    /// The number of the service in `services`, from 1, when the problem is in one.
    pub service_number: Option<u32>,
    /// The name of the key, when the problem is about a key that Compose defines.
    pub key: Option<&'static str>,
}

impl Problem {
    const fn at(code: ProblemCode, place: Place) -> Self {
        Self {
            code,
            place,
            service_number: None,
            key: None,
        }
    }

    const fn in_service(mut self, number: Option<u32>) -> Self {
        self.service_number = number;
        self
    }

    fn of_key(mut self, name: &str) -> Self {
        self.key = keys::defined(name);
        self
    }
}

/// Where the keys of a mapping are: what a key of it is.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Level {
    /// The top level: each key is a key of Compose or an extension.
    Top,
    /// `services`: each key is the name of a service.
    Services,
    /// One service: each key is a key of Compose or an extension.
    Service(u32),
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

/// The number of the entry at `index`, from 1.
fn number(index: usize) -> u32 {
    u32::try_from(index + 1).unwrap_or(u32::MAX)
}

/// The entries of a mapping whose key is a text. A key that is not a text and a key that is
/// there two times are problems. The merge key `<<` may be there more than one time.
fn entries<'a>(pairs: &'a [(Node, Node)], level: Level, check: &mut Check) -> Vec<Entry<'a>> {
    let mut seen = HashSet::new();
    let mut found = Vec::with_capacity(pairs.len());
    for (index, (key, value)) in pairs.iter().enumerate() {
        let service = match level {
            Level::Top => None,
            Level::Services => Some(number(index)),
            Level::Service(number) => Some(number),
        };
        let Shape::Text(name) = &key.shape else {
            check.add(Problem::at(ProblemCode::KeyNotText, key.place).in_service(service));
            continue;
        };
        if !seen.insert(name.as_str()) && name != "<<" {
            let problem = Problem::at(ProblemCode::DuplicateKey, key.place).in_service(service);
            // The name of a service is the operator's own text: only its number is given.
            let problem = if level == Level::Services {
                problem
            } else {
                problem.of_key(name)
            };
            check.add(problem);
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
    let found = entries(pairs, Level::Top, check);
    for entry in &found {
        match entry.name {
            "services" => services(entry, check),
            "version" => check.add(Problem::at(ProblemCode::VersionObsolete, entry.key.place)),
            known if keys::top_level(known) => {}
            _ => check.add(Problem::at(
                ProblemCode::UnknownTopLevelKey,
                entry.key.place,
            )),
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
            for (index, service) in entries(pairs, Level::Services, check).iter().enumerate() {
                one_service(service, number(index), check);
            }
            return;
        }
        Shape::Map(_) | Shape::Null => ProblemCode::ServicesEmpty,
        Shape::Text(_) | Shape::List(_) => ProblemCode::ServicesNotMapping,
    };
    check.add(Problem::at(code, entry.key.place));
}

fn one_service(service: &Entry<'_>, number: u32, check: &mut Check) {
    let name = Some(number);
    let pairs = match &service.value.shape {
        Shape::Alias => return,
        Shape::Map(pairs) => pairs,
        _ => {
            let problem = Problem::at(ProblemCode::ServiceNotMapping, service.key.place);
            check.add(problem.in_service(name));
            return;
        }
    };
    let found = entries(pairs, Level::Service(number), check);
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
