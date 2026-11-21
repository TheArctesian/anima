//! The example environment file of a recipe holds no secret value. A secret is made at the
//! install: its line has no value, and the line above it says so.

use crate::Code;

/// The line above a variable that the install makes.
pub(crate) const GENERATED: &str = "# generated at install";

/// The parts of a name that mark a secret, for example `DB_PASSWORD` or `SECRET_KEY_BASE`.
const SECRET_PARTS: [&str; 5] = ["PASSWORD", "PASS", "SECRET", "TOKEN", "KEY"];

fn is_secret_name(name: &str) -> bool {
    name.split('_').any(|part| SECRET_PARTS.contains(&part))
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
