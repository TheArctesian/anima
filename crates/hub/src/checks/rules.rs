//! Validation of operator input for checks: names, and the configuration a definition
//! stores. Pure. Errors name the field, never its value (LOG-007).

use super::{
    config::{
        CheckConfig, ConfigError, HttpAuth, HttpConfig, HttpCredentials, HttpInput, HttpSecrets,
        SnmpConfig, SnmpCredentials, SnmpInput, SnmpSecrets, SnmpVersion, is_secret_header,
    },
    model::{Definition, FieldError, PushToken},
};
use crate::credentials::Secret;

/// The secrets a definition seals, of its type.
pub(super) enum Sealable {
    Http(HttpSecrets),
    Snmp(SnmpSecrets),
}

/// What a definition stores: the secret-free configuration, the secrets to seal, and the
/// push token.
pub(super) struct Prepared {
    pub(super) config: CheckConfig,
    pub(super) secrets: Option<Sealable>,
    pub(super) push_token: Option<PushToken>,
}

/// The stored HTTP configuration and its opened secrets, for the empty-secret rule.
pub(super) type StoredHttp = (HttpConfig, HttpSecrets);

/// The stored SNMP configuration and its opened secrets, for the empty-secret rule.
pub(super) type StoredSnmp = (SnmpConfig, SnmpSecrets);

/// The check's current setup with its secrets opened, for the empty-secret rules.
pub(super) enum Stored {
    Nothing,
    Http(StoredHttp),
    Snmp(StoredSnmp),
}

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
/// check's current setup, used only for empty secrets (see `CheckChange`).
pub(super) fn prepare(definition: Definition, stored: Stored) -> Result<Prepared, FieldError> {
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
            let stored = match stored {
                Stored::Http(stored) => Some(stored),
                _ => None,
            };
            let (config, secrets) = keep_stored(input, stored)?.split().map_err(config_error)?;
            Ok(Prepared {
                config: CheckConfig::Http(config),
                secrets: (!secrets.is_empty()).then(|| Sealable::Http(secrets)),
                push_token: None,
            })
        }
        Definition::Snmp(input) => {
            let (config, given) = input.split().map_err(config_error)?;
            let stored = match stored {
                Stored::Snmp(stored) => Some(stored),
                _ => None,
            };
            let secrets = keep_snmp(&config, given, stored)?;
            Ok(Prepared {
                config: CheckConfig::Snmp(config),
                secrets: Some(Sealable::Snmp(secrets)),
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

/// Whether the input leaves out a secret of its version, which must come from the stored
/// setup.
pub(super) fn snmp_needs_stored(input: &SnmpInput) -> bool {
    let given = |secret: &Option<crate::credentials::Secret>| {
        secret.as_ref().is_some_and(|s| !s.expose().is_empty())
    };
    let credentials = &input.credentials;
    match input.config.version {
        SnmpVersion::V2c => !given(&credentials.community),
        SnmpVersion::V3 => {
            !given(&credentials.auth_password) || !given(&credentials.privacy_password)
        }
    }
}

/// Fills each secret left out from the stored secrets, only when they were given for the
/// same version and user name (SEC-005); otherwise the field error asks for it.
fn keep_snmp(
    config: &SnmpConfig,
    given: SnmpCredentials,
    stored: Option<StoredSnmp>,
) -> Result<SnmpSecrets, FieldError> {
    let kept = stored
        .filter(|(kept, _)| kept.version == config.version && kept.username == config.username)
        .map(|(_, secrets)| secrets);
    let missing = |field: &'static str, what: &str| {
        FieldError::new(
            field,
            format!("Enter the {what}: it is kept only for the same version and user name."),
        )
    };
    match (config.version, kept) {
        (SnmpVersion::V2c, kept) => {
            let stored = match kept {
                Some(SnmpSecrets::Community(community)) => Some(community),
                _ => None,
            };
            let community = given.community.or(stored);
            community
                .map(SnmpSecrets::Community)
                .ok_or_else(|| missing("community", "community"))
        }
        (SnmpVersion::V3, kept) => {
            let (stored_auth, stored_privacy) = match kept {
                Some(SnmpSecrets::User { auth, privacy }) => (Some(auth), Some(privacy)),
                _ => (None, None),
            };
            let auth = given.auth_password.or(stored_auth);
            let privacy = given.privacy_password.or(stored_privacy);
            let auth = auth.ok_or_else(|| missing("authPassword", "authentication password"))?;
            let privacy = privacy.ok_or_else(|| missing("privacyPassword", "privacy password"))?;
            Ok(SnmpSecrets::User { auth, privacy })
        }
    }
}

/// The origin of a URL as `scheme://host:port`, with the scheme's default port written out.
pub(super) fn origin(url: &str) -> Option<String> {
    let url = reqwest::Url::parse(url).ok()?;
    let host = url.host_str()?;
    let port = url.port_or_known_default()?;
    Some(format!("{}://{host}:{port}", url.scheme()))
}

fn secret_names<'a>(names: impl Iterator<Item = &'a str>) -> Vec<String> {
    let mut names: Vec<String> = names.map(str::to_ascii_lowercase).collect();
    names.sort_unstable();
    names
}

/// Whether stored secrets may go where the input sends them (SEC-005): the same origin, the
/// same kind of authentication and basic user name, TLS checks not turned off, and the same
/// secret header names.
fn same_destination(input: &HttpInput, stored: &HttpConfig) -> bool {
    let origin_kept = origin(&input.url).is_some_and(|o| Some(o) == origin(&stored.url));
    let auth_kept = match (&input.credentials, &stored.auth) {
        (HttpCredentials::None, HttpAuth::None)
        | (HttpCredentials::Bearer(_), HttpAuth::Bearer) => true,
        (HttpCredentials::Basic { username, .. }, HttpAuth::Basic { username: kept }) => {
            username == kept
        }
        _ => false,
    };
    let tls_kept = input.verify_tls || !stored.verify_tls;
    let given = input
        .headers
        .iter()
        .map(|(name, _)| name.as_str())
        .filter(|n| is_secret_header(n));
    let kept = stored
        .headers
        .iter()
        .filter(|h| h.value.is_none())
        .map(|h| h.name.as_str());
    origin_kept && auth_kept && tls_kept && secret_names(given) == secret_names(kept)
}

/// Fills each empty secret from the stored setup: the n-th secret header with a name takes
/// the stored value of the n-th stored header with that name.
/// Stored secrets are kept only for the same destination (see `same_destination`).
fn keep_stored(mut input: HttpInput, stored: Option<StoredHttp>) -> Result<HttpInput, FieldError> {
    if !needs_stored(&input) {
        return Ok(input);
    }
    if stored
        .as_ref()
        .is_some_and(|(config, _)| !same_destination(&input, config))
    {
        return Err(FieldError::new(
            "config",
            "Enter every secret again: the address, the authentication or the secret headers changed.",
        ));
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
