//! A check's configuration as a JSON file (`--from-file`): the K1 `CheckConfigInput`, one key
//! for the kind, such as `{"tcp": {"port": 22}}`. A file is the only way in because an SNMP
//! community, a password or a bearer token must never be an argument (CMD-025).

use std::fs::File;
use std::io::{self, Read as _};
use std::path::Path;

use serde::de::DeserializeOwned;
use zeroize::Zeroizing;

use crate::failure::Failure;

/// The largest configuration file read.
const MAX_INPUT: u64 = 1024 * 1024;

/// The kinds of `CheckConfigInput`, by their K1 field names.
const KINDS: [&str; 12] = [
    "icmp",
    "tcp",
    "tls",
    "http",
    "dns",
    "push",
    "group",
    "docker",
    "snmp",
    "tailscalePing",
    "systemd",
    "domainExpiry",
];

/// The configuration in `path` (`-` for standard input) as the generated `CheckConfigInput`
/// of one operation.
///
/// # Errors
/// [`Failure::Input`] for an unreadable file; [`Failure::Usage`] when the JSON is not one
/// kind's settings. The message names the position, never the content.
pub fn read<T: DeserializeOwned>(path: &Path) -> Result<T, Failure> {
    let bytes = read_bytes(path)?;
    let value: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|error| Failure::Usage {
            message: format!(
                "{} is not JSON (line {}, column {}).",
                path.display(),
                error.line(),
                error.column()
            ),
            hint: None,
        })?;
    check_shape(&value, path)?;
    serde_json::from_value(value).map_err(|_| Failure::Usage {
        message: format!(
            "A value in {} does not fit the check's settings; see CheckConfigInput in K1.",
            path.display()
        ),
        hint: Some("anima check create --help".to_owned()),
    })
}

fn check_shape(value: &serde_json::Value, path: &Path) -> Result<(), Failure> {
    let usage = |message: String| Failure::Usage {
        message,
        hint: Some("anima check create --help".to_owned()),
    };
    let Some(object) = value.as_object() else {
        return Err(usage(format!(
            "{} must hold a JSON object.",
            path.display()
        )));
    };
    if object.len() != 1 {
        return Err(usage(format!(
            "{} must have exactly one key, the kind: one of {}.",
            path.display(),
            KINDS.join(", ")
        )));
    }
    let kind = object.keys().next().map_or("", String::as_str);
    if !KINDS.contains(&kind) {
        return Err(usage(format!(
            "{} has the key {kind}; the kind must be one of {}.",
            path.display(),
            KINDS.join(", ")
        )));
    }
    Ok(())
}

fn read_bytes(path: &Path) -> Result<Zeroizing<Vec<u8>>, Failure> {
    let failed = |reason: String| Failure::Input {
        what: path.display().to_string(),
        reason,
    };
    let mut bytes = Zeroizing::new(Vec::new());
    let result = if path.as_os_str() == "-" {
        io::stdin()
            .lock()
            .take(MAX_INPUT + 1)
            .read_to_end(&mut bytes)
    } else {
        File::open(path).and_then(|file| file.take(MAX_INPUT + 1).read_to_end(&mut bytes))
    };
    result.map_err(|error| failed(error.to_string()))?;
    if bytes.len() as u64 > MAX_INPUT {
        return Err(failed(format!("it is larger than {MAX_INPUT} bytes")));
    }
    Ok(bytes)
}
