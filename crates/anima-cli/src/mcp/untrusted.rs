//! Text from devices and other people goes to the agent only after this (MCP-008): device and
//! check names, event messages, log lines and VPN host names could hold control characters or
//! text written to steer the agent. Each string is cleaned and capped, and a whole result is
//! kept under [`RESULT_CAP`] bytes so it fits the agent's context.

use serde_json::{Map, Value};

use crate::output::text::clean;

/// The longest string in a result, in characters, except a log line.
pub const TEXT_CAP: usize = 500;

/// The longest log line (`text` of a build log line), in characters.
pub const LOG_LINE_CAP: usize = 2_000;

/// The largest result, in bytes of JSON: about 15,000 tokens.
pub const RESULT_CAP: usize = 60_000;

/// `text` without control characters, cut to `cap` characters with a visible marker.
#[must_use]
pub fn capped(text: &str, cap: usize) -> String {
    let cleaned = clean(text);
    let length = cleaned.chars().count();
    if length <= cap {
        return cleaned;
    }
    let kept: String = cleaned.chars().take(cap).collect();
    format!("{kept}…[cut: {} more characters]", length - cap)
}

/// `value` with every string cleaned and capped; the key `text` (a log line) gets the
/// larger cap.
#[must_use]
pub fn cleaned(value: Value) -> Value {
    cleaned_under(value, TEXT_CAP)
}

fn cleaned_under(value: Value, cap: usize) -> Value {
    match value {
        Value::String(text) => Value::String(capped(&text, cap)),
        Value::Array(values) => Value::Array(
            values
                .into_iter()
                .map(|value| cleaned_under(value, cap))
                .collect(),
        ),
        Value::Object(fields) => Value::Object(
            fields
                .into_iter()
                .map(|(key, value)| {
                    let cap = if key == "text" {
                        LOG_LINE_CAP
                    } else {
                        TEXT_CAP
                    };
                    (capped(&key, TEXT_CAP), cleaned_under(value, cap))
                })
                .collect(),
        ),
        other => other,
    }
}

/// `result` cut to [`RESULT_CAP`]: the last entries of its longest top-level list go first.
/// A cut result says so in `truncated` and `note`, with how to get the rest.
#[must_use]
pub fn fitted(mut result: Map<String, Value>) -> Map<String, Value> {
    if size(&result) <= RESULT_CAP {
        return result;
    }
    let list = result
        .iter()
        .filter_map(|(key, value)| value.as_array().map(|values| (key.clone(), values.len())))
        .max_by_key(|(_, length)| *length)
        .map(|(key, _)| key)
        .unwrap_or_default();
    let mut dropped = 0_usize;
    while size(&result) > RESULT_CAP {
        let popped = result
            .get_mut(&list)
            .and_then(Value::as_array_mut)
            .and_then(Vec::pop);
        if popped.is_none() {
            return too_large();
        }
        dropped += 1;
    }
    result.insert("truncated".to_owned(), Value::Bool(true));
    result.insert(
        "note".to_owned(),
        Value::String(format!(
            "The result was cut to fit: the last {dropped} entries of {list} were left out. \
             Ask again with a smaller limit, without detail, or from nextCursor."
        )),
    );
    result
}

fn too_large() -> Map<String, Value> {
    let mut result = Map::new();
    result.insert("truncated".to_owned(), Value::Bool(true));
    result.insert(
        "note".to_owned(),
        Value::String(
            "The result was too large to return. Ask again without detail or with a smaller \
             limit."
                .to_owned(),
        ),
    );
    result
}

fn size(result: &Map<String, Value>) -> usize {
    serde_json::to_vec(result).map_or(usize::MAX, |bytes| bytes.len())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn mcp_008_control_characters_go_and_long_text_is_cut_with_a_marker() {
        assert_eq!(capped("web\u{1b}[2J01\nx", 50), "web [2J01 x");
        let long = "a".repeat(10_000);
        let cut = capped(&long, TEXT_CAP);
        assert!(cut.starts_with(&"a".repeat(TEXT_CAP)));
        assert!(cut.ends_with("…[cut: 9500 more characters]"), "{cut}");
    }

    #[test]
    fn mcp_008_every_string_in_a_tree_is_cleaned_and_log_lines_keep_more() {
        let value = cleaned(json!({
            "events": [{ "message": "x".repeat(900), "deviceName": "a\u{7}b" }],
            "lines": [{ "text": "y".repeat(1_500), "sequence": 3 }],
        }));
        let message = value["events"][0]["message"].as_str().unwrap_or_default();
        assert!(message.contains("[cut: 400 more characters]"));
        assert_eq!(value["events"][0]["deviceName"], "a b");
        assert_eq!(value["lines"][0]["text"], "y".repeat(1_500));
        assert_eq!(value["lines"][0]["sequence"], 3);
    }

    #[test]
    fn mcp_008_a_large_result_is_cut_and_says_how_to_get_the_rest() {
        let events: Vec<Value> = (0..400)
            .map(|index| json!({ "id": index, "message": "m".repeat(400) }))
            .collect();
        let mut result = Map::new();
        result.insert("events".to_owned(), Value::Array(events));
        result.insert("nextCursor".to_owned(), json!("c-9"));
        let fitted = fitted(result);
        assert!(size(&fitted) <= RESULT_CAP);
        assert_eq!(fitted["truncated"], true);
        let kept = fitted["events"].as_array().map_or(0, Vec::len);
        assert!(kept > 50 && kept < 400, "{kept}");
        let note = fitted["note"].as_str().unwrap_or_default();
        assert!(note.contains("entries of events"), "{note}");
    }

    #[test]
    fn mcp_008_a_small_result_is_left_as_it_is_and_one_huge_value_is_replaced() {
        let mut result = Map::new();
        result.insert("events".to_owned(), json!([1, 2]));
        assert_eq!(fitted(result.clone()), result);
        let mut huge = Map::new();
        huge.insert("device".to_owned(), json!({ "a": "x".repeat(RESULT_CAP) }));
        let fitted = fitted(huge);
        assert!(fitted.get("device").is_none());
        assert_eq!(fitted["truncated"], true);
    }
}
