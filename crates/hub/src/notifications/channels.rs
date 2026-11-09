//! A notification channel as the operator sets it up (NOT-002, NOT-003, D-17, CR-14): the
//! settings of each kind, the secrets kept apart from them, and the rules a new or changed
//! channel passes before anything is stored.
//!
//! Everything here is pure: no I/O, no clock, no randomness. A rule that refuses a value names
//! the input field and never repeats the value (LOG-007).
//!
//! The secrets of each kind are the whole webhook URL and the header values, the ntfy topic and
//! token, the SMTP password, and the Apprise URLs or configuration key. The settings hold only
//! what the API may show: the webhook origin, method and header names, the ntfy server and
//! priority, the mail server, sender and recipients, and the Apprise server, mode and tags
//! (SEC-005).

mod apprise;
mod apprise_url;
mod email;
mod ntfy;
mod outcome;
mod rules;
mod secrets;
mod settings;
#[cfg(test)]
mod tests;
mod webhook;

use super::template;
use crate::api_errors::FieldError;

pub(super) use apprise::{AppriseDraft, AppriseSecrets, AppriseSettings};
pub(super) use email::{EmailDraft, EmailSecrets, EmailSettings};
pub(super) use ntfy::{NtfyDraft, NtfySecrets, NtfySettings};
pub(super) use outcome::{Kept, Touched};
pub(super) use secrets::{Secrets, Wire};
pub(super) use settings::{AppriseMode, Settings, WebhookMethod};
pub(super) use webhook::{WebhookDraft, WebhookSecrets, WebhookSettings};

pub(super) use apprise_url::permitted as permitted_apprise_url;

use outcome::Outcome;
use rules::Problems;

/// The longest channel name, in characters (K2 `channels_name_check`).
const NAME_MAX: usize = 100;

/// A template that replaces the installation's default (NOT-007). Its length and placeholders
/// are checked by `template::field_errors`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Template {
    pub(super) title: String,
    pub(super) body: String,
}

/// The settings of one kind, as the operator sent them.
pub(super) enum SettingsDraft {
    Webhook(WebhookDraft),
    Ntfy(NtfyDraft),
    Email(EmailDraft),
    Apprise(AppriseDraft),
}

/// A channel as the operator sent it, secrets still in hand. It has no `Debug`.
pub(super) struct Draft {
    pub(super) name: String,
    pub(super) on_by_default: bool,
    pub(super) settings: SettingsDraft,
    pub(super) template: Option<Template>,
}

/// A channel that passed every rule, ready to seal and store.
pub(super) struct Prepared {
    pub(super) name: String,
    pub(super) on_by_default: bool,
    pub(super) settings: Settings,
    /// `None` when the kind has nothing to seal (an e-mail channel without a password).
    pub(super) secrets: Option<Secrets>,
    pub(super) template: Option<Template>,
    /// Which secret fields were set or cleared, by name, for the audit entry.
    pub(super) touched: Touched,
}

/// The rules for a new channel.
///
/// # Errors
/// Every field the operator must fix, by input field name.
pub(super) fn prepare_new(draft: Draft) -> Result<Prepared, Vec<FieldError>> {
    let mut problems = Problems::default();
    check_common(&mut problems, &draft);
    let outcome = match draft.settings {
        SettingsDraft::Webhook(input) => webhook::new(input),
        SettingsDraft::Ntfy(input) => ntfy::new(input),
        SettingsDraft::Email(input) => email::new(input),
        SettingsDraft::Apprise(input) => apprise::new(input),
    };
    finish(
        problems,
        outcome,
        (draft.name, draft.on_by_default, draft.template),
    )
}

/// The rules for a change to a stored channel whose settings are `before` and whose sealed
/// secrets gave `stored` (CR-14): a null secret keeps the stored one, an empty one clears it
/// where it is optional, a new target needs every stored secret again, and the kind never
/// changes.
///
/// # Errors
/// Every field the operator must fix; a change of kind is the only error then.
pub(super) fn prepare_change(
    before: &Settings,
    stored: Kept<Secrets>,
    draft: Draft,
) -> Result<Prepared, Vec<FieldError>> {
    let mut problems = Problems::default();
    check_common(&mut problems, &draft);
    let outcome = match (before, draft.settings) {
        (Settings::Webhook(before), SettingsDraft::Webhook(input)) => {
            webhook::change(before, stored.map(Secrets::into_webhook), input)
        }
        (Settings::Ntfy(before), SettingsDraft::Ntfy(input)) => {
            ntfy::change(before, stored.map(Secrets::into_ntfy), input)
        }
        (Settings::Email(before), SettingsDraft::Email(input)) => {
            email::change(before, stored.map(Secrets::into_email), input)
        }
        (Settings::Apprise(before), SettingsDraft::Apprise(input)) => {
            apprise::change(before, stored.map(Secrets::into_apprise), input)
        }
        _ => {
            return Err(vec![FieldError::new(
                "settings",
                "A channel's kind cannot change. Create a new channel.",
            )]);
        }
    };
    finish(
        problems,
        outcome,
        (draft.name, draft.on_by_default, draft.template),
    )
}

/// The name and template rules every channel passes.
fn check_common(problems: &mut Problems, draft: &Draft) {
    let length = draft.name.chars().count();
    if draft.name.trim().is_empty() {
        problems.add("name", "Enter a name.");
    } else if length > NAME_MAX {
        problems.add("name", format!("Use at most {NAME_MAX} characters."));
    } else if !crate::text::is_plain(&draft.name) {
        problems.add("name", "Remove line breaks and invisible characters.");
    }
    if let Some(template) = &draft.template {
        for error in template::field_errors(&template.title, &template.body) {
            problems.add(error.field, error.message);
        }
    }
}

/// The prepared channel, or every problem of the common fields and the kind together.
fn finish(
    problems: Problems,
    outcome: Result<Outcome, Vec<FieldError>>,
    (name, on_by_default, template): (String, bool, Option<Template>),
) -> Result<Prepared, Vec<FieldError>> {
    let mut errors = problems.into_errors();
    match outcome {
        Ok(outcome) if errors.is_empty() => Ok(Prepared {
            name,
            on_by_default,
            settings: outcome.settings,
            secrets: outcome.secrets,
            template,
            touched: outcome.touched,
        }),
        Ok(_) => Err(errors),
        Err(kind_errors) => {
            errors.extend(kind_errors);
            Err(errors)
        }
    }
}
