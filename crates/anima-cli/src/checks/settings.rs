//! A check's configuration as label and value pairs, whatever its kind. Reads the `--json`
//! form of the K1 `CheckConfig` (tagged by `__typename`), so `list` and `get` share it.

use serde_json::Value;

/// What identifies a check comes first; the other settings follow in alphabetical order.
const FIRST: [&str; 13] = [
    "url",
    "domain",
    "container",
    "unit",
    "name",
    "target",
    "port",
    "serverName",
    "method",
    "recordType",
    "version",
    "oid",
    "children",
];

/// The settings of `config`, the identifying ones first. Empty values are left out.
#[must_use]
pub fn settings(config: &Value) -> Vec<(String, String)> {
    let Some(fields) = config.as_object() else {
        return Vec::new();
    };
    let mut pairs: Vec<(String, String)> = fields
        .iter()
        .filter(|(name, _)| name.as_str() != "__typename")
        .filter_map(|(name, value)| render(value).map(|text| (name.clone(), text)))
        .collect();
    pairs.sort_by_key(|(name, _)| {
        FIRST
            .iter()
            .position(|first| first == name)
            .unwrap_or(FIRST.len())
    });
    pairs
}

/// The settings as `name=value` text for one table cell.
#[must_use]
pub fn one_line(config: &Value) -> String {
    settings(config)
        .iter()
        .map(|(name, value)| format!("{name}={value}"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// A label for people: `timeoutMs` as `Timeout ms`.
#[must_use]
pub fn label(name: &str) -> String {
    let mut out = String::new();
    for (index, character) in name.chars().enumerate() {
        if character.is_uppercase() {
            out.push(' ');
            out.extend(character.to_lowercase());
        } else if index == 0 {
            out.extend(character.to_uppercase());
        } else {
            out.push(character);
        }
    }
    out
}

fn render(value: &Value) -> Option<String> {
    match value {
        Value::Null => None,
        Value::String(text) => Some(text.clone()),
        Value::Array(items) => {
            let parts: Vec<String> = items.iter().filter_map(render_member).collect();
            (!parts.is_empty()).then(|| parts.join(", "))
        }
        other => Some(other.to_string()),
    }
}

/// A member of a list: a status range as `200-299`, a header or child as its name or ID.
fn render_member(member: &Value) -> Option<String> {
    match member {
        Value::Object(fields) => match (fields.get("from"), fields.get("to")) {
            (Some(from), Some(to)) => Some(format!("{from}-{to}")),
            _ => ["name", "id"]
                .iter()
                .find_map(|key| fields.get(*key).and_then(Value::as_str))
                .map(str::to_owned),
        },
        other => render(other),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn cmd_020_settings_read_each_kind_of_value() {
        let config = json!({
            "__typename": "HttpConfig", "url": "https://a.example", "method": "GET",
            "headers": [{ "name": "X-Key" }], "acceptedStatus": [{ "from": 200, "to": 299 }],
            "keyword": null, "timeoutMs": 10000, "verifyTls": true,
        });
        assert_eq!(
            one_line(&config),
            "url=https://a.example method=GET acceptedStatus=200-299 headers=X-Key \
             timeoutMs=10000 verifyTls=true"
        );
        assert_eq!(label("timeoutMs"), "Timeout ms");
    }
}
