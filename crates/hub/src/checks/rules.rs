//! Validation of operator input for checks: names, and the configuration a definition
//! stores. Pure. Errors name the field, never its value (LOG-007).

use super::{
    config::{
        CheckConfig, ConfigError, HttpConfig, HttpCredentials, HttpInput, HttpSecrets,
        is_secret_header,
    },
    model::{Definition, FieldError, PushToken},
};
use crate::credentials::Secret;

/// What a definition stores: the secret-free configuration, the HTTP secrets to seal, and
/// the push token.
pub(super) struct Prepared {
    pub(super) config: CheckConfig,
    pub(super) secrets: Option<HttpSecrets>,
    pub(super) push_token: Option<PushToken>,
}

/// The stored HTTP configuration and its opened secrets, for the empty-secret rule.
pub(super) type StoredHttp = (HttpConfig, HttpSecrets);

/// Whether the text has no control character, line or paragraph separator, or mark that
/// changes the reading direction: names reach the event and audit logs (LOG-004).
fn is_plain(text: &str) -> bool {
    !text.chars().any(|c| {
        c.is_control()
            || matches!(
                c,
                '\u{2028}'
                    | '\u{2029}'
                    | '\u{200E}'
                    | '\u{200F}'
                    | '\u{061C}'
                    | '\u{202A}'..='\u{202E}'
                    | '\u{2066}'..='\u{2069}'
            )
    })
}

/// A trimmed, plain, non-empty name. The length limit is the database's (`checks_name_check`).
pub(super) fn name(text: &str) -> Result<String, FieldError> {
    let name = text.trim();
    if name.is_empty() {
        return Err(FieldError::new("name", "Enter a name for the check."));
    }
    if !is_plain(name) {
        return Err(FieldError::new(
            "name",
            "A name cannot contain control characters or text-direction marks.",
        ));
    }
    Ok(name.to_owned())
}

fn config_error(error: ConfigError) -> FieldError {
    FieldError::new(error.field, format!("This value {}.", error.reason))
}

/// Validates the definition and separates what is stored from the secrets. `stored` is the
/// check's current HTTP setup, used only for empty secrets (see `CheckChange`).
pub(super) fn prepare(
    definition: Definition,
    stored: Option<StoredHttp>,
) -> Result<Prepared, FieldError> {
    match definition {
        Definition::Config(CheckConfig::Http(_)) => Err(FieldError::new(
            "config",
            "Give an HTTP check with its headers and authentication.",
        )),
        Definition::Config(CheckConfig::Push) => {
            Err(FieldError::new("config", "A push check needs a token."))
        }
        Definition::Config(config) => {
            config.validate().map_err(config_error)?;
            Ok(Prepared {
                config,
                secrets: None,
                push_token: None,
            })
        }
        Definition::Http(input) => {
            let (config, secrets) = keep_stored(input, stored)?.split().map_err(config_error)?;
            Ok(Prepared {
                config: CheckConfig::Http(config),
                secrets: (!secrets.is_empty()).then_some(secrets),
                push_token: None,
            })
        }
        Definition::Push(token) => Ok(Prepared {
            config: CheckConfig::Push,
            secrets: None,
            push_token: Some(token),
        }),
    }
}

/// Whether the input has an empty secret that must come from the stored setup.
pub(super) fn needs_stored(input: &HttpInput) -> bool {
    let empty_header = input
        .headers
        .iter()
        .any(|(name, value)| is_secret_header(name) && value.expose().is_empty());
    let empty_credential = match &input.credentials {
        HttpCredentials::None => false,
        HttpCredentials::Basic { password, .. } => password.expose().is_empty(),
        HttpCredentials::Bearer(token) => token.expose().is_empty(),
    };
    empty_header || empty_credential
}

/// Fills each empty secret from the stored setup: the n-th secret header with a name takes
/// the stored value of the n-th stored header with that name.
fn keep_stored(mut input: HttpInput, stored: Option<StoredHttp>) -> Result<HttpInput, FieldError> {
    if !needs_stored(&input) {
        return Ok(input);
    }
    let (config, secrets) = stored.unwrap_or_else(|| (HttpConfig::new(""), HttpSecrets::default()));
    let HttpSecrets {
        password,
        bearer_token,
        headers,
    } = secrets;
    let mut kept: Vec<(String, Option<Secret>)> = config
        .headers
        .into_iter()
        .filter(|header| header.value.is_none())
        .map(|header| header.name)
        .zip(headers.into_iter().map(Some))
        .collect();
    let mut seen: Vec<String> = Vec::new();
    for (name, value) in &mut input.headers {
        if !is_secret_header(name) {
            continue;
        }
        let position = seen.iter().filter(|n| n.eq_ignore_ascii_case(name)).count();
        seen.push(name.clone());
        if value.expose().is_empty() {
            *value = kept
                .iter_mut()
                .filter(|(stored_name, _)| stored_name.eq_ignore_ascii_case(name))
                .nth(position)
                .and_then(|(_, stored_value)| stored_value.take())
                .ok_or_else(|| FieldError::new("headers", "Enter the value of this header."))?;
        }
    }
    let missing = || FieldError::new("auth", "Enter the password or token.");
    input.credentials = match input.credentials {
        HttpCredentials::Basic {
            username,
            password: given,
        } if given.expose().is_empty() => HttpCredentials::Basic {
            username,
            password: password.ok_or_else(missing)?,
        },
        HttpCredentials::Bearer(given) if given.expose().is_empty() => {
            HttpCredentials::Bearer(bearer_token.ok_or_else(missing)?)
        }
        other => other,
    };
    Ok(input)
}
