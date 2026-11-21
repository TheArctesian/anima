//! No file of a recipe holds a secret value. A secret is made at the install: its line in the
//! example environment file has no value, and the line above it says so; the Compose file gives
//! it only as a variable.

use compose_rules::Node;

use crate::{Code, privileges::environment};

/// The line above a variable that the install makes.
pub(crate) const GENERATED: &str = "# generated at install";

/// The parts of a name, between two `_`, that mark a secret: `DB_PASS`, `API_KEY`.
const SECRET_PARTS: [&str; 2] = ["PASS", "KEY"];
/// The texts that mark a secret in each place of a name: `DB_PASSWORD`, `SECRET_KEY_BASE`,
/// `MYAPIKEY`, `SMTP_PASSWD`, `AWS_CREDENTIALS`, `PRIVATE_KEY` (SF-225).
const SECRET_TEXTS: [&str; 7] = [
    "PASSWORD",
    "PASSWD",
    "SECRET",
    "TOKEN",
    "APIKEY",
    "CREDENTIAL",
    "PRIVATE_KEY",
];

/// True when the name marks a secret, in capital or small letters.
fn is_secret_name(name: &str) -> bool {
    let name = name.to_ascii_uppercase();
    name.split('_').any(|part| SECRET_PARTS.contains(&part))
        || SECRET_TEXTS.iter().any(|text| name.contains(text))
}

/// `${NAME}`: one variable with no default, which the environment file fills.
fn is_one_variable(value: &str) -> bool {
    value
        .strip_prefix("${")
        .and_then(|rest| rest.strip_suffix('}'))
        .is_some_and(|name| {
            !name.is_empty()
                && name
                    .bytes()
                    .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
        })
}

/// True when a service of the Compose file, as its author wrote it, gives a variable whose
/// name marks a secret a value that is not one variable: a literal, or a default after `:-`.
pub(crate) fn secret_in_file(services: &[(String, Node)]) -> bool {
    services.iter().any(|(_, service)| {
        environment(service)
            .into_iter()
            .any(|(name, value)| is_secret_name(name) && !is_one_variable(value))
    })
}

/// One variable of the file, with the line above it.
struct Line<'a> {
    name: &'a str,
    value: &'a str,
    marked: bool,
}

fn lines(text: &str) -> Vec<Line<'_>> {
    let mut above = "";
    let mut found = Vec::new();
    for line in text.lines() {
        if let Some((name, value)) = line.split_once('=')
            && !line.starts_with('#')
        {
            found.push(Line {
                name,
                value,
                marked: above == GENERATED,
            });
        }
        above = line;
    }
    found
}

/// The codes of the example file `text` of a recipe whose install makes `generated`.
pub(crate) fn check(text: &str, generated: &[String]) -> Vec<Code> {
    let found = lines(text);
    let mut codes = Vec::new();
    let secret = found.iter().any(|line| {
        !line.value.is_empty()
            && (is_secret_name(line.name) || generated.iter().any(|name| name == line.name))
    });
    if secret {
        codes.push(Code::SecretInExample);
    }
    let each_marked = generated
        .iter()
        .all(|name| found.iter().any(|line| line.name == name && line.marked));
    let each_listed = found
        .iter()
        .filter(|line| line.marked)
        .all(|line| generated.iter().any(|name| name == line.name));
    if !each_marked || !each_listed {
        codes.push(Code::GeneratedNotMarked);
    }
    codes
}
