//! The Apprise channel: a notification through an Apprise API server, by URLs sent with each
//! notification (stateless) or by the key of a configuration stored in Apprise (NOT-002,
//! D-18, D-19). The URLs or the key are sealed; the server, mode and tags are shown.

use serde::{Deserialize, Serialize};

use super::{
    AppriseMode, Kept, Secrets, Settings, Touched,
    apprise_url::permitted,
    outcome::Outcome,
    rules::{Problems, is_empty, secret_text, server_origin},
};
use crate::{api_errors::FieldError, credentials::Secret};

const SERVER: &str = "settings.apprise.server";
const URLS: &str = "settings.apprise.urls";
const KEY: &str = "settings.apprise.key";
const TAGS: &str = "settings.apprise.tags";

const URLS_MAX: usize = 20;
const KEY_MAX: usize = 128;
const TAGS_MAX: usize = 20;
const TAG_MAX: usize = 64;

/// What the API shows of an Apprise channel.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(in crate::notifications) struct AppriseSettings {
    /// `https://host[:port]` of the Apprise API.
    pub(in crate::notifications) server: String,
    pub(in crate::notifications) mode: AppriseMode,
    pub(in crate::notifications) tags: Vec<String>,
}

/// The secret of the channel's mode.
pub(in crate::notifications) enum AppriseSecrets {
    Urls(Vec<Secret>),
    Key(Secret),
}

/// An Apprise channel as the operator sent it.
pub(in crate::notifications) struct AppriseDraft {
    pub(in crate::notifications) server: String,
    pub(in crate::notifications) mode: AppriseMode,
    /// `None` on update keeps the stored URLs.
    pub(in crate::notifications) urls: Option<Vec<Secret>>,
    /// `None` on update keeps the stored key.
    pub(in crate::notifications) key: Option<Secret>,
    pub(in crate::notifications) tags: Vec<String>,
}

/// Each URL passes N-4; at least one, at most 20.
fn checked_urls(urls: &[Secret]) -> Result<(), FieldError> {
    if urls.is_empty() || urls.len() > URLS_MAX {
        return Err(FieldError::new(URLS, "Enter 1 to 20 Apprise URLs."));
    }
    for url in urls {
        permitted(secret_text(URLS, url)?)
            .map_err(|refusal| FieldError::new(URLS, refusal.message()))?;
    }
    Ok(())
}

/// A key of letters, digits, `_` and `-`: it goes into the API path, so `/` and `.` never reach
/// it.
fn checked_key(key: &Secret) -> Result<(), FieldError> {
    let text = secret_text(KEY, key)?;
    let allowed = |b: u8| b.is_ascii_alphanumeric() || b == b'_' || b == b'-';
    if text.is_empty() || text.len() > KEY_MAX || !text.bytes().all(allowed) {
        Err(FieldError::new(
            KEY,
            "Enter the configuration key: letters, digits, _ and -, at most 128.",
        ))
    } else {
        Ok(())
    }
}

/// Tags of letters, digits, `_`, `-` and `.`: Apprise reads a comma or a space in its tag
/// field as "or" and "and".
fn checked_tags(tags: &[String]) -> Result<(), FieldError> {
    let allowed = |b: u8| b.is_ascii_alphanumeric() || b"_-.".contains(&b);
    let tag_ok = |tag: &String| (1..=TAG_MAX).contains(&tag.len()) && tag.bytes().all(allowed);
    if tags.len() <= TAGS_MAX && tags.iter().all(tag_ok) {
        Ok(())
    } else {
        Err(FieldError::new(
            TAGS,
            "Use at most 20 tags of letters, digits, _, - and ., each at most 64.",
        ))
    }
}

/// The secret the draft sends for its mode, if any, checked (an empty one is refused, as the
/// mode needs it); the other mode's field must be null or empty.
fn sent_secret(draft: &mut AppriseDraft, problems: &mut Problems) -> Option<AppriseSecrets> {
    let (urls, key) = (draft.urls.take(), draft.key.take());
    match draft.mode {
        AppriseMode::StatelessUrls => {
            if key.as_ref().is_some_and(|key| !is_empty(key)) {
                problems.add(KEY, "Send a key only in the config-key mode.");
            }
            let urls = urls?;
            problems.note(checked_urls(&urls));
            Some(AppriseSecrets::Urls(urls))
        }
        AppriseMode::ConfigKey => {
            if urls.as_ref().is_some_and(|urls| !urls.is_empty()) {
                problems.add(URLS, "Send URLs only in the stateless mode.");
            }
            let key = key?;
            problems.note(checked_key(&key));
            Some(AppriseSecrets::Key(key))
        }
    }
}

/// The field and the sentence that ask for the mode's secret: on create, after a mode change
/// (CR-14 item 2), or after a server change (item 1).
const fn missing(mode: AppriseMode, why: Why) -> (&'static str, &'static str) {
    match (mode, why) {
        (AppriseMode::StatelessUrls, Why::New) => (URLS, "Enter 1 to 20 Apprise URLs."),
        (AppriseMode::StatelessUrls, Why::Switched) => {
            (URLS, "The mode changed: enter the Apprise URLs.")
        }
        (AppriseMode::StatelessUrls, Why::Moved) => {
            (URLS, "The server changed: enter the Apprise URLs again.")
        }
        (AppriseMode::ConfigKey, Why::New) => (KEY, "Enter the configuration key."),
        (AppriseMode::ConfigKey, Why::Switched) => {
            (KEY, "The mode changed: enter the configuration key.")
        }
        (AppriseMode::ConfigKey, Why::Moved) => (
            KEY,
            "The server changed: enter the configuration key again.",
        ),
    }
}

/// Why the mode's secret must be in the input.
#[derive(Clone, Copy)]
enum Why {
    New,
    Switched,
    Moved,
}

const fn field(secret: &AppriseSecrets) -> &'static str {
    match secret {
        AppriseSecrets::Urls(_) => "urls",
        AppriseSecrets::Key(_) => "key",
    }
}

fn outcome(
    draft: AppriseDraft,
    server: String,
    secret: AppriseSecrets,
    touched: Touched,
) -> Outcome {
    let settings = AppriseSettings {
        server,
        mode: draft.mode,
        tags: draft.tags,
    };
    Outcome {
        settings: Settings::Apprise(settings),
        secrets: Some(Secrets::Apprise(secret)),
        touched,
    }
}

/// The rules for a new Apprise channel: the secret of its mode is required.
pub(super) fn new(mut draft: AppriseDraft) -> Result<Outcome, Vec<FieldError>> {
    let mut problems = Problems::default();
    let server = problems.take(server_origin(SERVER, &draft.server));
    problems.note(checked_tags(&draft.tags));
    let secret = sent_secret(&mut draft, &mut problems);
    if secret.is_none() {
        let (field, message) = missing(draft.mode, Why::New);
        problems.add(field, message);
    }
    let (Some(server), Some(secret)) = (server, secret) else {
        return Err(problems.into_errors());
    };
    problems.check()?;
    let mut touched = Touched::default();
    touched.set(field(&secret));
    Ok(outcome(draft, server, secret, touched))
}

/// The rules for a changed Apprise channel (CR-14): a new mode drops the old mode's secret and
/// needs the new one; a new server needs the secret again; otherwise a null secret keeps the
/// stored one.
pub(super) fn change(
    before: &AppriseSettings,
    stored: Kept<AppriseSecrets>,
    mut draft: AppriseDraft,
) -> Result<Outcome, Vec<FieldError>> {
    let mut problems = Problems::default();
    let server = problems.take(server_origin(SERVER, &draft.server));
    problems.note(checked_tags(&draft.tags));
    let moved = server
        .as_ref()
        .is_some_and(|server| *server != before.server);
    let switched = draft.mode != before.mode;
    let sent = sent_secret(&mut draft, &mut problems);
    if sent.is_none() && (moved || switched) {
        let why = if switched { Why::Switched } else { Why::Moved };
        let (field, message) = missing(draft.mode, why);
        problems.add(field, message);
    }
    let Some(server) = server else {
        return Err(problems.into_errors());
    };
    problems.check()?;
    let mut touched = Touched::default();
    if switched {
        touched.cleared(match before.mode {
            AppriseMode::StatelessUrls => "urls",
            AppriseMode::ConfigKey => "key",
        });
    }
    let secret = match sent {
        Some(secret) => {
            touched.set(field(&secret));
            secret
        }
        None => stored.opened()?,
    };
    Ok(outcome(draft, server, secret, touched))
}
