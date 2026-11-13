//! The parts every tool shares: reading its arguments, a page (`limit`, `after`), the
//! `confirm` of a delete (MCP-007) and the short form of a result (MCP-009).

use rmcp::model::JsonObject;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::{Map, Value};

use super::outcome::ToolError;

/// The arguments of a call as the tool's input type.
///
/// # Errors
/// [`ToolError::Refused`] naming what does not fit the input schema.
pub fn parse<T: DeserializeOwned>(arguments: Option<JsonObject>) -> Result<T, ToolError> {
    let object = Value::Object(arguments.unwrap_or_default());
    serde_json::from_value(object).map_err(|error| {
        ToolError::Refused(format!(
            "The arguments do not fit the tool's input schema: {error}. Correct them and call \
             again."
        ))
    })
}

/// `limit` and `after` of a list (MCP-009).
#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Page {
    /// How many to return, 1 to 100 (default 25).
    pub limit: Option<u32>,
    /// The `nextCursor` of the previous result, to get the next page.
    pub after: Option<String>,
}

/// The default page size.
const DEFAULT_LIMIT: u32 = 25;

/// The largest page.
const MAX_LIMIT: u32 = 100;

impl Page {
    /// The page size, between 1 and 100.
    #[must_use]
    pub fn size(&self) -> i64 {
        i64::from(self.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT))
    }

    /// This page of a whole list the hub returns at once; the cursor is the next position.
    #[must_use]
    pub fn of(&self, all: Vec<Value>) -> (Vec<Value>, Option<String>) {
        let start = self
            .after
            .as_deref()
            .and_then(|after| after.parse::<usize>().ok())
            .unwrap_or_default();
        let size = usize::try_from(self.size()).unwrap_or(1);
        let total = all.len();
        let page: Vec<Value> = all.into_iter().skip(start).take(size).collect();
        let end = start + page.len();
        (page, (end < total).then(|| end.to_string()))
    }
}

/// Refuses a delete unless `confirm` is exactly `name` (MCP-007).
///
/// # Errors
/// [`ToolError::Refused`] saying which name to give.
pub fn confirmed(confirm: &str, name: &str, what: &str) -> Result<(), ToolError> {
    if confirm == name {
        return Ok(());
    }
    Err(ToolError::Refused(format!(
        "Nothing was deleted: `confirm` must be exactly the {what}'s name. Ask the operator \
         first, then call again with confirm set to that name."
    )))
}

/// `value` (a K1 object) with only these keys, unless `detail` asks for all of them.
#[must_use]
pub fn short(value: Value, keys: &[&str], detail: bool) -> Value {
    match value {
        Value::Object(fields) if !detail => Value::Object(
            fields
                .into_iter()
                .filter(|(key, _)| keys.contains(&key.as_str()))
                .collect::<Map<String, Value>>(),
        ),
        other => other,
    }
}

/// `value` serialised; a generated K1 answer always serialises.
#[must_use]
pub fn json<T: serde::Serialize>(value: &T) -> Value {
    serde_json::to_value(value).unwrap_or(Value::Null)
}

/// The fields of `value` that are given (not null), as K1 input fields.
#[must_use]
pub fn present(value: &impl serde::Serialize) -> Map<String, Value> {
    match json(value) {
        Value::Object(fields) => fields
            .into_iter()
            .filter(|(_, value)| !value.is_null())
            .collect(),
        _ => Map::new(),
    }
}

/// `fields` as the generated K1 input type `T`; serde checks every value.
///
/// # Errors
/// [`ToolError::Refused`] naming `what` and the field that does not fit.
pub fn k1_input<T: DeserializeOwned>(
    fields: Map<String, Value>,
    what: &str,
) -> Result<T, ToolError> {
    serde_json::from_value(Value::Object(fields)).map_err(|error| {
        ToolError::Refused(format!(
            "The {what} is not complete or not valid: {error}. Correct it and call again."
        ))
    })
}

/// The JSON object of these named values.
#[must_use]
pub fn object<const N: usize>(fields: [(&str, Value); N]) -> Map<String, Value> {
    fields
        .into_iter()
        .map(|(key, value)| (key.to_owned(), value))
        .collect()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn mcp_009_a_page_is_between_1_and_100() {
        let page = |limit| Page { limit, after: None };
        assert_eq!(page(None).size(), 25);
        assert_eq!(page(Some(0)).size(), 1);
        assert_eq!(page(Some(5_000)).size(), 100);
    }

    #[test]
    fn mcp_009_a_whole_list_is_paged_with_a_position_cursor() {
        let all: Vec<Value> = (0..5).map(|index| json!(index)).collect();
        let first = Page {
            limit: Some(2),
            after: None,
        };
        assert_eq!(
            first.of(all.clone()),
            (vec![json!(0), json!(1)], Some("2".to_owned()))
        );
        let last = Page {
            limit: Some(2),
            after: Some("4".to_owned()),
        };
        assert_eq!(last.of(all), (vec![json!(4)], None));
    }

    #[test]
    fn mcp_007_a_delete_needs_the_exact_name() {
        assert!(confirmed("web-01", "web-01", "device").is_ok());
        assert!(confirmed("web-0", "web-01", "device").is_err());
        assert!(confirmed("WEB-01", "web-01", "device").is_err());
    }

    #[test]
    fn mcp_009_the_short_form_keeps_only_the_named_keys() {
        let value = json!({ "id": "1", "name": "a", "macAddress": "x" });
        assert_eq!(
            short(value.clone(), &["id", "name"], false),
            json!({ "id": "1", "name": "a" })
        );
        assert_eq!(short(value.clone(), &["id"], true), value);
    }

    #[test]
    fn mcp_009_wrong_arguments_are_refused_with_the_reason() {
        let mut arguments = JsonObject::new();
        arguments.insert("limit".to_owned(), json!("many"));
        let error = parse::<Page>(Some(arguments)).expect_err("a refusal");
        assert!(error.to_string().contains("input schema"), "{error}");
    }
}
