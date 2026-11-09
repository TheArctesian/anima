//! The rules of a new release (BLD-001, BLD-002): its version, commit and targets, the
//! refusal of a version that is not new for a target, and the version the form proposes.
//!
//! Pure: the versions already built come in as a parameter.

use super::{
    model::{Commit, FieldError, JobState, Target},
    version::Version,
};

/// A release as the operator asked for it, before the rules.
#[derive(Clone, Debug)]
pub struct ReleaseInput {
    pub version: String,
    pub commit: String,
    pub targets: Vec<Target>,
}

/// A release that passed the rules: the targets are sorted and each named once.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct NewRelease {
    pub(super) version: Version,
    pub(super) commit: Commit,
    pub(super) targets: Vec<Target>,
}

/// A job that holds a version of a target: published, or queued or under way.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Built {
    pub(super) target: Target,
    pub(super) version: Version,
    pub(super) state: JobState,
}

/// The commit as 40 lower-case hex digits; upper-case digits are lowered.
fn commit(text: &str) -> Option<Commit> {
    let text = text.trim();
    (text.len() == 40 && text.bytes().all(|b| b.is_ascii_hexdigit()))
        .then(|| Commit(text.to_ascii_lowercase()))
}

/// Checks the form of a release: every field in error is listed.
pub(super) fn validate(input: &ReleaseInput) -> Result<NewRelease, Vec<FieldError>> {
    let mut errors = Vec::new();
    let version = input.version.trim().parse::<Version>().map_err(|_| {
        errors.push(FieldError::new(
            "version",
            "Use three numbers with dots between, such as 0.1.12.",
        ));
    });
    let commit = commit(&input.commit).ok_or_else(|| {
        errors.push(FieldError::new(
            "commit",
            "Use the 40 hex digits of a commit of main.",
        ));
    });
    let mut targets = input.targets.clone();
    targets.sort_unstable();
    targets.dedup();
    if targets.is_empty() {
        errors.push(FieldError::new("targets", "Select at least one target."));
    }
    match (version, commit) {
        (Ok(version), Ok(commit)) if errors.is_empty() => Ok(NewRelease {
            version,
            commit,
            targets,
        }),
        _ => Err(errors),
    }
}

/// Refuses `version` for a target that has published it or a later version, or that has it
/// queued or under way: a version is built at most once for a target (BLD-002). A failed or
/// cancelled job does not count.
pub(super) fn check_new(
    version: Version,
    targets: &[Target],
    built: &[Built],
) -> Result<(), FieldError> {
    for target in targets {
        let of_target = built.iter().filter(|b| b.target == *target);
        let newest_published = of_target
            .clone()
            .filter(|b| b.state == JobState::Published)
            .map(|b| b.version)
            .max();
        if let Some(published) = newest_published.filter(|published| version <= *published) {
            return Err(FieldError::new(
                "version",
                format!("{target} has published {published}. Use a later version."),
            ));
        }
        let taken = of_target
            .filter(|b| !matches!(b.state, JobState::Failed | JobState::Cancelled))
            .any(|b| b.version == version);
        if taken {
            return Err(FieldError::new(
                "version",
                format!("{target} already has a build of {version}."),
            ));
        }
    }
    Ok(())
}

/// The version the release form proposes: the highest version published or under way, with
/// the patch number plus 1; `None` before the first release (Section 3.10).
pub(super) fn next_version(built: &[Built]) -> Option<Version> {
    built
        .iter()
        .filter(|b| !matches!(b.state, JobState::Failed | JobState::Cancelled))
        .map(|b| b.version)
        .max()
        .and_then(Version::next_patch)
}
