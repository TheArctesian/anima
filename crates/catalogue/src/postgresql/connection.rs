//! The connection values that the program writes into the variables that the recipe of an
//! application names (Section 14.2, step 5).

use std::fmt;

use super::{names::Names, statements::Password};
use crate::entry::Variables;

/// Where the application reaches the server: the name of the server on the network of the
/// pair (`anima-db-<application>`) and its port in the container.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Address {
    pub host: String,
    pub port: u16,
}

/// One line of the environment file of the application. The value of the password and of the
/// address that holds it are not shown by `Debug`.
#[derive(Clone, PartialEq, Eq)]
pub struct Value {
    pub variable: String,
    pub value: String,
    secret: bool,
}

impl fmt::Debug for Value {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = if self.secret { "hidden" } else { &self.value };
        write!(formatter, "{}={value}", self.variable)
    }
}

/// A host name of the network of Docker: letters, digits, `.`, `_` and `-`.
fn is_host(host: &str) -> bool {
    !host.is_empty()
        && host.len() <= 253
        && host
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
}

/// `text` with each byte that is not unreserved in an address (RFC 3986) written `%XX`.
fn escaped(text: &str) -> String {
    text.bytes()
        .map(|byte| {
            if byte.is_ascii_alphanumeric() || b"-._~".contains(&byte) {
                char::from(byte).to_string()
            } else {
                format!("%{byte:02X}")
            }
        })
        .collect()
}

/// The value of each variable that `variables` names, in the order url, host, port, name,
/// user, password. `None` when the host is not a host name.
#[must_use]
pub fn values(
    variables: &Variables,
    address: &Address,
    names: &Names,
    password: &Password,
) -> Option<Vec<Value>> {
    if !is_host(&address.host) {
        return None;
    }
    let url = format!(
        "postgresql://{}:{}@{}:{}/{}",
        names.user,
        escaped(password.text()),
        address.host,
        address.port,
        names.database
    );
    let each = [
        (&variables.url, url, true),
        (&variables.host, address.host.clone(), false),
        (&variables.port, address.port.to_string(), false),
        (&variables.name, names.database.to_string(), false),
        (&variables.user, names.user.to_string(), false),
        (&variables.password, password.text().to_owned(), true),
    ];
    Some(
        each.into_iter()
            .filter_map(|(variable, value, secret)| {
                variable.as_ref().map(|variable| Value {
                    variable: variable.clone(),
                    value,
                    secret,
                })
            })
            .collect(),
    )
}
