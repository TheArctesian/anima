//! Validation of what a window applies to (MW-003), and the rule for plain text.
//!
//! Names and tags reach the event log and the audit log, which are kept for a year and cannot
//! be edited (LOG-004).

use super::{
    model::{Target, TargetForm, TargetType},
    rules::{FieldError, Problems},
};
use crate::{
    ids::{CheckId, DeviceId},
    text::is_plain,
};

const TARGETS_MAX: usize = 100;
const TAG_MAX: usize = 32;

/// One to 100 targets, without repeats, in first-seen order (MW-003). A tag follows the same
/// rules as a device tag.
///
/// # Errors
/// `targets`, with a message that names the first target in error.
pub(super) fn validate(forms: &[TargetForm]) -> Result<Vec<Target>, Vec<FieldError>> {
    if forms.is_empty() {
        return Err(vec![FieldError::new(
            "targets",
            "Choose at least one device, check or tag.",
        )]);
    }
    if forms.len() > TARGETS_MAX {
        return Err(vec![FieldError::new(
            "targets",
            format!("A window can have at most {TARGETS_MAX} targets."),
        )]);
    }
    let mut problems = Problems::default();
    let mut kept: Vec<Target> = Vec::with_capacity(forms.len());
    for (index, form) in forms.iter().enumerate() {
        if let Some(target) = problems.field(target(form, index + 1))
            && !kept.contains(&target)
        {
            kept.push(target);
        }
    }
    problems.finish(Some(kept))
}

fn target(form: &TargetForm, position: usize) -> Result<Target, FieldError> {
    let text = form.target.trim();
    let bad = |message: &str| FieldError::new("targets", format!("Target {position}: {message}"));
    match form.kind {
        TargetType::Device => text
            .parse::<DeviceId>()
            .map(Target::Device)
            .map_err(|_| bad("a device target must be a device ID.")),
        TargetType::Check => text
            .parse::<CheckId>()
            .map(Target::Check)
            .map_err(|_| bad("a check target must be a check ID.")),
        TargetType::Tag if text.is_empty() => Err(bad("the tag is empty.")),
        TargetType::Tag if !is_plain(text) => Err(bad(
            "a tag cannot contain control characters or text-direction marks.",
        )),
        TargetType::Tag if text.chars().count() > TAG_MAX => {
            Err(bad(&format!("a tag is at most {TAG_MAX} characters.")))
        }
        TargetType::Tag => Ok(Target::Tag(text.to_owned())),
    }
}
