//! The ntfy channel: a message to a topic on an ntfy server (NOT-002). On a public server the
//! topic is the credential, so it is sealed with the access token; the server and priority
//! are shown.

use serde::{Deserialize, Serialize};

use super::{
    Kept, Secrets, Settings, Touched,
    outcome::{Outcome, unopened},
    rules::{Problems, is_empty, secret_text, server_origin},
};
use crate::{api_errors::FieldError, credentials::Secret};

const SERVER: &str = "settings.ntfy.server";
const TOPIC: &str = "settings.ntfy.topic";
const TOKEN: &str = "settings.ntfy.token";
const PRIORITY: &str = "settings.ntfy.priority";

/// The longest topic ntfy accepts, and the longest token the hub stores.
const TOPIC_MAX: usize = 64;
const TOKEN_MAX: usize = 256;

/// What the API shows of an ntfy channel.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(in crate::notifications) struct NtfySettings {
    /// `https://host[:port]`.
    pub(in crate::notifications) server: String,
    /// 1 (min) to 5 (max).
    pub(in crate::notifications) priority: u8,
    pub(in crate::notifications) token_stored: bool,
}

/// The topic and the optional access token.
pub(in crate::notifications) struct NtfySecrets {
    pub(in crate::notifications) topic: Secret,
    pub(in crate::notifications) token: Option<Secret>,
}

/// An ntfy channel as the operator sent it.
pub(in crate::notifications) struct NtfyDraft {
    pub(in crate::notifications) server: String,
    /// `None` on update keeps the stored topic.
    pub(in crate::notifications) topic: Option<Secret>,
    /// `None` on update keeps the stored token; an empty one removes it.
    pub(in crate::notifications) token: Option<Secret>,
    pub(in crate::notifications) priority: i32,
}

/// ntfy's own topic rule: letters, digits, `_` and `-`. The topic goes into the URL path, so
/// nothing else may reach it.
fn checked_topic(topic: &Secret) -> Result<(), FieldError> {
    let text = secret_text(TOPIC, topic)?;
    let allowed = |b: u8| b.is_ascii_alphanumeric() || b == b'_' || b == b'-';
    if text.is_empty() {
        Err(FieldError::new(TOPIC, "Enter the topic."))
    } else if text.len() > TOPIC_MAX || !text.bytes().all(allowed) {
        Err(FieldError::new(
            TOPIC,
            "Use letters, digits, _ and - in the topic, at most 64.",
        ))
    } else {
        Ok(())
    }
}

/// A token of visible ASCII characters, the shape an HTTP `Authorization` value needs.
fn checked_token(token: &Secret) -> Result<(), FieldError> {
    let text = secret_text(TOKEN, token)?;
    if text.len() > TOKEN_MAX || !text.bytes().all(|b| b.is_ascii_graphic()) {
        Err(FieldError::new(
            TOKEN,
            "Use the token as ntfy shows it: at most 256 characters, no spaces.",
        ))
    } else {
        Ok(())
    }
}

fn checked_priority(priority: i32) -> Result<u8, FieldError> {
    u8::try_from(priority)
        .ok()
        .filter(|priority| (1..=5).contains(priority))
        .ok_or_else(|| FieldError::new(PRIORITY, "Use a priority from 1 to 5."))
}

fn outcome(settings: NtfySettings, secrets: NtfySecrets, touched: Touched) -> Outcome {
    Outcome {
        settings: Settings::Ntfy(settings),
        secrets: Some(Secrets::Ntfy(secrets)),
        touched,
    }
}

/// The rules for a new ntfy channel: the topic is required, the token optional.
pub(super) fn new(draft: NtfyDraft) -> Result<Outcome, Vec<FieldError>> {
    let mut problems = Problems::default();
    let server = problems.take(server_origin(SERVER, &draft.server));
    let topic = draft.topic.unwrap_or_else(|| Secret::new(Vec::new()));
    problems.note(checked_topic(&topic));
    let token = draft.token.filter(|token| !is_empty(token));
    if let Some(token) = &token {
        problems.note(checked_token(token));
    }
    let priority = problems.take(checked_priority(draft.priority));
    let (Some(server), Some(priority)) = (server, priority) else {
        return Err(problems.into_errors());
    };
    problems.check()?;
    let mut touched = Touched::default();
    touched.set("topic");
    if token.is_some() {
        touched.set("token");
    }
    let settings = NtfySettings {
        server,
        priority,
        token_stored: token.is_some(),
    };
    Ok(outcome(settings, NtfySecrets { topic, token }, touched))
}

/// The rules for a changed ntfy channel (CR-14): a new server needs the topic again, and the
/// token again or cleared; otherwise a null secret keeps the stored one.
pub(super) fn change(
    before: &NtfySettings,
    stored: Kept<NtfySecrets>,
    draft: NtfyDraft,
) -> Result<Outcome, Vec<FieldError>> {
    let mut problems = Problems::default();
    let server = problems.take(server_origin(SERVER, &draft.server));
    let moved = server
        .as_ref()
        .is_some_and(|server| *server != before.server);
    match &draft.topic {
        Some(topic) => problems.note(checked_topic(topic)),
        None if moved => problems.add(TOPIC, "The server changed: enter the topic again."),
        None => {}
    }
    let token_kept = draft.token.is_none() && before.token_stored;
    match &draft.token {
        Some(token) if !is_empty(token) => problems.note(checked_token(token)),
        None if moved && token_kept => problems.add(
            TOKEN,
            "The server changed: enter the token again, or an empty one to remove it.",
        ),
        Some(_) | None => {}
    }
    let priority = problems.take(checked_priority(draft.priority));
    let (Some(server), Some(priority)) = (server, priority) else {
        return Err(problems.into_errors());
    };
    problems.check()?;
    let (stored_topic, stored_token) = if draft.topic.is_none() || token_kept {
        let opened = stored.opened()?;
        (Some(opened.topic), opened.token)
    } else {
        (None, None)
    };
    let mut touched = Touched::default();
    if draft.topic.is_some() {
        touched.set("topic");
    }
    let topic = draft.topic.or(stored_topic).ok_or_else(unopened)?;
    let token = match draft.token {
        Some(token) if is_empty(&token) => {
            if before.token_stored {
                touched.cleared("token");
            }
            None
        }
        Some(token) => {
            touched.set("token");
            Some(token)
        }
        None if token_kept => Some(stored_token.ok_or_else(unopened)?),
        None => None,
    };
    let settings = NtfySettings {
        server,
        priority,
        token_stored: token.is_some(),
    };
    Ok(outcome(settings, NtfySecrets { topic, token }, touched))
}
