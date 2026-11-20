//! The object of each work type and the secrets that it can name (K10 Section 6): `build` and
//! `runner`. The list is fixed in this program; the hub cannot add a type.

use serde_json::{Map, Value};

use super::shape::{has_exactly, text};

type Object = Map<String, Value>;

/// The recipes of a `build` that this daemon has (K10 Section 6.1).
pub const RECIPES: [&str; 1] = ["desktop_release"];

/// The version of a runner program, as GitHub numbers it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version(pub u64, pub u64, pub u64);

impl Version {
    /// Reads `major.minor.patch`, each a number with no sign and no other character.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let mut parts = text.split('.').map(|part| {
            let digits = !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit());
            digits.then(|| part.parse::<u64>().ok()).flatten()
        });
        let version = Self(parts.next()??, parts.next()??, parts.next()??);
        parts.next().is_none().then_some(version)
    }
}

fn is_hex(text: &str, length: usize) -> bool {
    text.len() == length && text.bytes().all(|b| b.is_ascii_hexdigit())
}

/// A short word of a release: a target, a version or a label.
fn is_word(text: &str) -> bool {
    (1..=64).contains(&text.len())
        && text
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-' | b'+'))
}

/// Whether `parameters` is the object of a `build` and its secrets are permitted for its
/// origin: an item whose origin is `other` names no `project_secret` (SF-009).
pub(super) fn build_holds(parameters: &Object, secrets: &[&str]) -> bool {
    const MEMBERS: [&str; 6] = ["recipe", "commit", "target", "version", "trigger", "origin"];
    let origin = text(parameters, "origin");
    has_exactly(parameters, &MEMBERS)
        && text(parameters, "recipe").is_some_and(|recipe| RECIPES.contains(&recipe))
        && text(parameters, "commit").is_some_and(|commit| is_hex(commit, 40))
        && text(parameters, "target").is_some_and(is_word)
        && text(parameters, "version").is_some_and(is_word)
        && text(parameters, "trigger")
            .is_some_and(|trigger| ["operator", "push"].contains(&trigger))
        && origin.is_some_and(|origin| ["default_branch", "other"].contains(&origin))
        && !(origin == Some("other") && secrets.contains(&"project_secret"))
}

/// The version of the runner program of a `runner` item; `None` when `parameters` is not the
/// object of that type.
pub(super) fn runner_version(parameters: &Object) -> Option<Version> {
    const MEMBERS: [&str; 3] = ["runnerVersion", "runnerSha256", "labels"];
    let labels = parameters.get("labels")?.as_array()?;
    let labels_hold = labels.len() <= 16
        && labels
            .iter()
            .all(|label| label.as_str().is_some_and(is_word));
    let holds = has_exactly(parameters, &MEMBERS)
        && text(parameters, "runnerSha256").is_some_and(|sha| is_hex(sha, 64))
        && labels_hold;
    holds
        .then(|| text(parameters, "runnerVersion").and_then(Version::parse))
        .flatten()
}

/// The purposes that an item of the type can name (K10 Sections 6 and 8).
pub(super) fn purposes_of(work_type: &str) -> &'static [&'static str] {
    match work_type {
        "build" => &["repository_token", "project_secret"],
        "runner" => &["jit_config"],
        _ => &[],
    }
}
