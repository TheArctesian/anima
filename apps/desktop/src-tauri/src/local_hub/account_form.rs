//! The forms of the operator account's values as they pass between the web view and
//! `anima-local-hub account` (F-4, F-5). Pure: bytes in, checked values out.

use std::ffi::OsString;

use serde::Serialize;
use zeroize::Zeroizing;

use super::LocalHubError;

const MOST_NAME: usize = 64;
const MOST_PASSWORD: usize = 1024;
const MOST_SECRET: usize = 64;
const CODE_DIGITS: usize = 6;

/// The account's name. It goes on the command line, so it must not look like an option.
pub fn name(name: &str) -> Result<OsString, LocalHubError> {
    let fits = !name.is_empty()
        && name.len() <= MOST_NAME
        && !name.starts_with('-')
        && !name.chars().any(char::is_control);
    if !fits {
        return Err(LocalHubError::Argument);
    }
    Ok(name.into())
}

/// The three lines that `account create` reads: the password, the TOTP secret and a code.
/// A line break in a value would move the lines, so none is permitted.
pub fn create_input(
    password: &str,
    secret: &str,
    code: &str,
) -> Result<Zeroizing<Vec<u8>>, LocalHubError> {
    let password_fits = !password.is_empty()
        && password.len() <= MOST_PASSWORD
        && !password.contains(['\n', '\r', '\0']);
    let secret_fits = !secret.is_empty()
        && secret.len() <= MOST_SECRET
        && secret
            .bytes()
            .all(|byte| matches!(byte, b'A'..=b'Z' | b'a'..=b'z' | b'2'..=b'7' | b'='));
    let code_fits = code.len() == CODE_DIGITS && code.bytes().all(|byte| byte.is_ascii_digit());
    if !(password_fits && secret_fits && code_fits) {
        return Err(LocalHubError::Argument);
    }
    let mut input = Zeroizing::new(Vec::with_capacity(
        password.len() + secret.len() + code.len() + 3,
    ));
    for line in [password, secret, code] {
        input.extend_from_slice(line.as_bytes());
        input.push(b'\n');
    }
    Ok(input)
}

/// The operator account as `account status` prints it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Account {
    pub name: String,
    pub totp: bool,
}

/// `account status`: one account, or `null` before the first run made it.
pub fn parse_account(printed: &[u8]) -> Result<Option<Account>, LocalHubError> {
    let value: serde_json::Value =
        serde_json::from_slice(printed).map_err(|_| LocalHubError::Failed)?;
    if value.is_null() {
        return Ok(None);
    }
    let name = value.get("name").and_then(serde_json::Value::as_str);
    let totp = value.get("totp").and_then(serde_json::Value::as_bool);
    match (name, totp) {
        (Some(name), Some(totp)) => Ok(Some(Account {
            name: name.to_owned(),
            totp,
        })),
        _ => Err(LocalHubError::Failed),
    }
}

/// A new TOTP secret and its `otpauth://` URI, as `account totp-new` prints them. Both are
/// secrets: they go to the web view, which shows them once, and nowhere else.
#[derive(Clone, PartialEq, Eq, Serialize)]
pub struct NewTotp {
    pub secret: String,
    pub uri: String,
}

impl std::fmt::Debug for NewTotp {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        out.write_str("NewTotp(..)")
    }
}

/// `account totp-new`: `{ "secret", "uri" }`, the URI an `otpauth://totp/` one.
pub fn parse_new_totp(printed: &[u8]) -> Result<NewTotp, LocalHubError> {
    let value: serde_json::Value =
        serde_json::from_slice(printed).map_err(|_| LocalHubError::Failed)?;
    let text = |field| value.get(field).and_then(serde_json::Value::as_str);
    match (text("secret"), text("uri")) {
        (Some(secret), Some(uri)) if !secret.is_empty() && uri.starts_with("otpauth://totp/") => {
            Ok(NewTotp {
                secret: secret.to_owned(),
                uri: uri.to_owned(),
            })
        }
        _ => Err(LocalHubError::Failed),
    }
}
