//! Reading the K1 values of the forge reads for a table. The documents select different parts
//! of one type, so each is rendered from its JSON.

use serde_json::Value;

/// The text under `key`, or `-` when it is null or missing.
#[must_use]
pub fn text<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or("-")
}

#[must_use]
pub fn flag(value: &Value, key: &str) -> bool {
    value.get(key).and_then(Value::as_bool).unwrap_or(false)
}

/// The elements under `key`; none when it is null or missing.
#[must_use]
pub fn many<'a>(value: &'a Value, key: &str) -> &'a [Value] {
    value
        .get(key)
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice)
}

/// The names under `key` joined with commas; `-` when there are none.
#[must_use]
pub fn names(value: &Value, key: &str) -> String {
    let names: Vec<&str> = many(value, key).iter().filter_map(Value::as_str).collect();
    if names.is_empty() {
        "-".to_owned()
    } else {
        names.join(", ")
    }
}

/// The number under `key`, or `-`.
#[must_use]
pub fn number(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_i64)
        .map_or_else(|| "-".to_owned(), |number| number.to_string())
}
