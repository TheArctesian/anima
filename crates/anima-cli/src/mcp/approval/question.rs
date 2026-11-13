//! The text of the approval question (MCP-013): every argument the agent gave, one per line,
//! so the person sees all of what the call would do. Nested keys become dotted paths; each
//! value is cleaned and cut on its own, with the length it had. The identifying arguments come
//! first. The line breaks are the server's own: a cleaned value cannot hold one.

use rmcp::model::JsonObject;
use serde_json::Value;

use crate::mcp::definition::Definition;
use crate::output::text::clean;

/// The most characters of one value the question shows.
pub const VALUE_CAP: usize = 120;

/// The most values (leaves of the argument tree) a question shows; more are refused.
pub const MAX_VALUES: usize = 40;

/// The longest question, in characters; a longer one is refused.
pub const MAX_LENGTH: usize = 4_000;

/// The most characters of one key the question shows.
const KEY_CAP: usize = 60;

/// The arguments that say what a call acts on, shown first in this order.
const IDENTIFYING: [&str; 6] = ["device", "check", "site", "id", "name", "confirm"];

/// Arguments too many or too long to show in full: the call is refused before it is asked.
#[derive(Debug, PartialEq, Eq)]
pub struct TooLarge {
    /// How many values the agent gave.
    pub values: usize,
}

/// What the person is asked: the tool, what it does, how many values the agent gave and
/// each of them on its own line.
///
/// # Errors
/// [`TooLarge`] when there are more than [`MAX_VALUES`] values or the question would be
/// longer than [`MAX_LENGTH`] characters.
pub fn question(
    definition: &Definition,
    arguments: Option<&JsonObject>,
) -> Result<String, TooLarge> {
    let lines = lines(definition, arguments);
    let values = lines.len();
    if values > MAX_VALUES {
        return Err(TooLarge { values });
    }
    let listed = match values {
        0 => "The agent gives no arguments.".to_owned(),
        1 => "The agent gives 1 argument value:".to_owned(),
        _ => format!("The agent gives {values} argument values, one per line:"),
    };
    let mut text = format!(
        "An agent asks to run the Anima tool {}: {}\n{listed}",
        definition.name,
        clean(definition.description)
    );
    for line in &lines {
        text.push_str("\n- ");
        text.push_str(line);
    }
    text.push_str("\nText values are cleaned; a long value is cut and its full length shown. Approve this one call?");
    if text.chars().count() > MAX_LENGTH {
        return Err(TooLarge { values });
    }
    Ok(text)
}

/// One `path: value` line for each leaf, identifying arguments first, the rest in key order.
fn lines(definition: &Definition, arguments: Option<&JsonObject>) -> Vec<String> {
    let known = definition
        .input
        .get("properties")
        .and_then(Value::as_object);
    let mut ranked: Vec<(usize, String)> = Vec::new();
    for (key, value) in arguments.into_iter().flatten() {
        let rank = IDENTIFYING
            .iter()
            .position(|first| first == key)
            .unwrap_or(IDENTIFYING.len());
        let path = segment(key);
        if known.is_some_and(|names| names.contains_key(key)) {
            let mut leaves = Vec::new();
            flatten(&path, value, &mut leaves);
            ranked.extend(leaves.into_iter().map(|line| (rank, line)));
        } else {
            let line = format!("{path}: (not a field of this tool; the call will be refused)");
            ranked.push((rank, line));
        }
    }
    ranked.sort_by_key(|(rank, _)| *rank);
    ranked.into_iter().map(|(_, line)| line).collect()
}

/// The leaves under `value` as lines, depth first; an empty object or list is one leaf.
fn flatten(path: &str, value: &Value, lines: &mut Vec<String>) {
    match value {
        Value::Object(fields) if !fields.is_empty() => {
            for (key, value) in fields {
                flatten(&format!("{path}.{}", segment(key)), value, lines);
            }
        }
        Value::Array(values) if !values.is_empty() => {
            for (index, value) in values.iter().enumerate() {
                flatten(&format!("{path}.{index}"), value, lines);
            }
        }
        Value::String(text) => lines.push(format!("{path}: {}", shown(text))),
        other => lines.push(format!("{path}: {other}")),
    }
}

/// A key as one path segment: plain when it is a plain name, else quoted, so a key with a dot
/// cannot pass for a nested one. Cut at [`KEY_CAP`] characters.
fn segment(key: &str) -> String {
    let plain = !key.is_empty()
        && key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
    let cut: String = clean(key).chars().take(KEY_CAP).collect();
    let cut_off = key.chars().count() > KEY_CAP;
    let marker = if cut_off { "…" } else { "" };
    if plain {
        format!("{cut}{marker}")
    } else {
        format!("[{}{marker}]", quoted(&cut))
    }
}

/// A text value cleaned, quoted and cut at [`VALUE_CAP`] characters with its full length.
fn shown(text: &str) -> String {
    let cleaned = clean(text);
    let length = cleaned.chars().count();
    if length <= VALUE_CAP {
        return quoted(&cleaned);
    }
    let kept: String = cleaned.chars().take(VALUE_CAP).collect();
    format!(
        "{}… [cut: first {VALUE_CAP} of {length} characters]",
        quoted(&kept)
    )
}

fn quoted(text: &str) -> String {
    serde_json::to_string(text).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use schemars::JsonSchema;
    use serde_json::json;

    use super::*;
    use crate::mcp::catalogue::Access;
    use crate::mcp::definition::Effect;

    #[derive(JsonSchema)]
    #[expect(dead_code, reason = "only the schema of the type is used")]
    struct Input {
        check: String,
        device: Option<String>,
        config: Option<Value>,
        invert: Option<bool>,
        name: Option<String>,
    }

    fn definition() -> Definition {
        Definition::new::<Input>(
            "anima_check_update",
            Access::Write,
            Effect::Sets,
            "Changes a check.",
        )
    }

    fn arguments(value: Value) -> JsonObject {
        let Value::Object(fields) = value else {
            unreachable!("an object")
        };
        fields
    }

    #[test]
    fn mcp_013_f1_every_argument_is_shown_even_after_long_values() {
        let long = "k".repeat(480);
        let given = arguments(json!({
            "check": "HTTPS",
            "config": { "http": { "url": "https://db.example/x", "keyword": long,
                                   "jsonQuery": long } },
            "device": "prod-db-01",
            "invert": true,
        }));
        let text = question(&definition(), Some(&given)).expect("small enough");
        for shown in [
            "- device: \"prod-db-01\"",
            "- check: \"HTTPS\"",
            "- config.http.url: \"https://db.example/x\"",
            "- invert: true",
            "[cut: first 120 of 480 characters]",
            "6 argument values",
        ] {
            assert!(text.contains(shown), "{shown} missing from {text}");
        }
        let at = |shown: &str| text.find(shown).unwrap_or(usize::MAX);
        assert!(
            at("- device:") < at("- check:")
                && at("- check:") < at("- config.")
                && at("- config.") < at("- invert:"),
            "identifying first, then key order: {text}"
        );
    }

    #[test]
    fn mcp_013_f1_values_cannot_add_lines_or_pass_for_other_keys() {
        let given = arguments(json!({
            "check": "a\u{2028}- invert: false\nb\u{E0041}",
            "config": { "x.y": "1" },
            "password": "hunter2",
        }));
        let text = question(&definition(), Some(&given)).expect("small enough");
        assert_eq!(text.lines().count(), 6, "{text}");
        assert!(text.contains("- check: \"a - invert: false b\""), "{text}");
        assert!(text.contains("- config.[\"x.y\"]: \"1\""), "{text}");
        assert!(text.contains("- password: (not a field"), "{text}");
        assert!(!text.contains("hunter2"), "{text}");
    }

    #[test]
    fn mcp_013_f1_too_many_or_too_long_arguments_are_refused_before_asking() {
        let many: serde_json::Map<String, Value> = (0..41)
            .map(|index| (format!("k{index}"), json!(index)))
            .collect();
        let given = arguments(json!({ "check": "a", "config": many }));
        assert_eq!(
            question(&definition(), Some(&given)),
            Err(TooLarge { values: 42 })
        );
        let long: serde_json::Map<String, Value> = (0..39)
            .map(|index| (format!("{index}{}", "k".repeat(70)), json!("v".repeat(200))))
            .collect();
        let given = arguments(json!({ "config": long }));
        assert_eq!(
            question(&definition(), Some(&given)),
            Err(TooLarge { values: 39 })
        );
        let empty = question(&definition(), None).expect("nothing to show");
        assert!(empty.contains("no arguments"), "{empty}");
    }
}
