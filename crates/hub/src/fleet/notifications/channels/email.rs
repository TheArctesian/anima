//! The e-mail channel: a message through an SMTP server, always over implicit TLS or STARTTLS
//! (NOT-002, D-17). Only the password is sealed; the server, the user name, the sender and the
//! recipients are shown. Recipient addresses are never written to the process log (K7).

use std::{net::IpAddr, str::FromStr};

use lettre::message::Mailbox;
use serde::{Deserialize, Serialize};

use super::{
    Kept, Secrets, Settings, Touched,
    outcome::Outcome,
    rules::{Problems, is_empty, secret_text},
};
use crate::{
    api_errors::FieldError,
    fleet::{credentials::Secret, notifications::egress::SmtpSecurity},
};

const HOST: &str = "settings.email.host";
const PORT: &str = "settings.email.port";
const USERNAME: &str = "settings.email.username";
const PASSWORD: &str = "settings.email.password";
const FROM: &str = "settings.email.from";
const TO: &str = "settings.email.to";

const HOST_MAX: usize = 253;
const USERNAME_MAX: usize = 254;
const PASSWORD_MAX: usize = 512;
const ADDRESS_MAX: usize = 320;
const RECIPIENTS_MAX: usize = 20;

/// What the API shows of an e-mail channel.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(in crate::fleet::notifications) struct EmailSettings {
    /// A host name or IP literal.
    pub(in crate::fleet::notifications) host: String,
    pub(in crate::fleet::notifications) port: u16,
    pub(in crate::fleet::notifications) security: SmtpSecurity,
    pub(in crate::fleet::notifications) username: Option<String>,
    pub(in crate::fleet::notifications) from: String,
    pub(in crate::fleet::notifications) to: Vec<String>,
    pub(in crate::fleet::notifications) password_stored: bool,
}

/// The SMTP password; an e-mail channel without one seals nothing.
pub(in crate::fleet::notifications) struct EmailSecrets {
    pub(in crate::fleet::notifications) password: Secret,
}

/// An e-mail channel as the operator sent it.
pub(in crate::fleet::notifications) struct EmailDraft {
    pub(in crate::fleet::notifications) host: String,
    pub(in crate::fleet::notifications) port: i32,
    pub(in crate::fleet::notifications) security: SmtpSecurity,
    pub(in crate::fleet::notifications) username: Option<String>,
    /// `None` on update keeps the stored password; an empty one removes it.
    pub(in crate::fleet::notifications) password: Option<Secret>,
    pub(in crate::fleet::notifications) from: String,
    pub(in crate::fleet::notifications) to: Vec<String>,
}

/// A host name of letters, digits and hyphens, or an IP literal; never a URL.
fn checked_host(host: &str) -> Result<(), FieldError> {
    let label = |label: &str| {
        (1..=63).contains(&label.len())
            && !label.starts_with('-')
            && !label.ends_with('-')
            && label
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-')
    };
    let valid = !host.is_empty()
        && host.len() <= HOST_MAX
        && (IpAddr::from_str(host).is_ok() || host.split('.').all(label));
    if valid {
        Ok(())
    } else {
        Err(FieldError::new(
            HOST,
            "Enter the mail server's host name or IP address, without a scheme or port.",
        ))
    }
}

fn checked_port(port: i32) -> Result<u16, FieldError> {
    u16::try_from(port)
        .ok()
        .filter(|port| *port != 0)
        .ok_or_else(|| FieldError::new(PORT, "Use a port from 1 to 65535."))
}

/// An empty user name means none.
fn checked_username(username: Option<String>) -> Result<Option<String>, FieldError> {
    let username = username.filter(|name| !name.is_empty());
    match &username {
        Some(name) if name.len() > USERNAME_MAX || name.chars().any(char::is_control) => Err(
            FieldError::new(USERNAME, "Use at most 254 characters and no line breaks."),
        ),
        _ => Ok(username),
    }
}

fn checked_password(password: &Secret) -> Result<(), FieldError> {
    let text = secret_text(PASSWORD, password)?;
    if text.len() > PASSWORD_MAX || text.chars().any(char::is_control) {
        Err(FieldError::new(
            PASSWORD,
            "Use at most 512 bytes and no control characters.",
        ))
    } else {
        Ok(())
    }
}

/// One mailbox, `name <user@host>` or `user@host`, with no line break: a line break in an
/// address would be a header injection.
fn is_mailbox(text: &str) -> bool {
    text.len() <= ADDRESS_MAX
        && !text.chars().any(char::is_control)
        && Mailbox::from_str(text).is_ok()
}

fn checked_addresses(from: &str, to: &[String]) -> Vec<FieldError> {
    let mut errors = Vec::new();
    if !is_mailbox(from) {
        errors.push(FieldError::new(
            FROM,
            "Enter one address, such as Anima <hub@example.com>.",
        ));
    }
    if to.is_empty() || to.len() > RECIPIENTS_MAX {
        errors.push(FieldError::new(TO, "Enter 1 to 20 recipients."));
    } else if !to.iter().all(|address| is_mailbox(address)) {
        errors.push(FieldError::new(TO, "Enter one address in each recipient."));
    }
    errors
}

/// Everything but the password, checked.
fn checked_settings(
    draft: &mut EmailDraft,
    problems: &mut Problems,
) -> Option<(u16, Option<String>)> {
    problems.note(checked_host(&draft.host));
    let port = problems.take(checked_port(draft.port));
    let username = problems.take(checked_username(draft.username.take()));
    for error in checked_addresses(&draft.from, &draft.to) {
        problems.add(error.field, error.message);
    }
    port.zip(username)
}

/// The settings and secrets of a checked draft with `password`.
fn outcome(
    draft: EmailDraft,
    (port, username): (u16, Option<String>),
    password: Option<Secret>,
    touched: Touched,
) -> Result<Outcome, Vec<FieldError>> {
    if password.is_some() && username.is_none() {
        return Err(vec![FieldError::new(
            USERNAME,
            "Enter the user name the password belongs to.",
        )]);
    }
    let settings = EmailSettings {
        host: draft.host,
        port,
        security: draft.security,
        username,
        from: draft.from,
        to: draft.to,
        password_stored: password.is_some(),
    };
    Ok(Outcome {
        settings: Settings::Email(settings),
        secrets: password.map(|password| Secrets::Email(EmailSecrets { password })),
        touched,
    })
}

/// The rules for a new e-mail channel: the password is optional.
pub(super) fn new(mut draft: EmailDraft) -> Result<Outcome, Vec<FieldError>> {
    let mut problems = Problems::default();
    let checked = checked_settings(&mut draft, &mut problems);
    let password = draft.password.take().filter(|password| !is_empty(password));
    if let Some(password) = &password {
        problems.note(checked_password(password));
    }
    let Some(checked) = checked else {
        return Err(problems.into_errors());
    };
    problems.check()?;
    let mut touched = Touched::default();
    if password.is_some() {
        touched.set("password");
    }
    outcome(draft, checked, password, touched)
}

/// The rules for a changed e-mail channel (CR-14): a new host or port needs the password
/// again, or an empty one to remove it; otherwise a null password keeps the stored one.
pub(super) fn change(
    before: &EmailSettings,
    stored: Kept<EmailSecrets>,
    mut draft: EmailDraft,
) -> Result<Outcome, Vec<FieldError>> {
    let mut problems = Problems::default();
    let checked = checked_settings(&mut draft, &mut problems);
    let moved = draft.host != before.host || checked.as_ref().is_some_and(|c| c.0 != before.port);
    let password_kept = draft.password.is_none() && before.password_stored;
    match &draft.password {
        Some(password) if !is_empty(password) => problems.note(checked_password(password)),
        None if moved && password_kept => problems.add(
            PASSWORD,
            "The server changed: enter the password again, or an empty one to remove it.",
        ),
        Some(_) | None => {}
    }
    let Some(checked) = checked else {
        return Err(problems.into_errors());
    };
    problems.check()?;
    let mut touched = Touched::default();
    let password = match draft.password.take() {
        Some(password) if is_empty(&password) => {
            if before.password_stored {
                touched.cleared("password");
            }
            None
        }
        Some(password) => {
            touched.set("password");
            Some(password)
        }
        None if password_kept => Some(stored.opened()?.password),
        None => None,
    };
    outcome(draft, checked, password, touched)
}
