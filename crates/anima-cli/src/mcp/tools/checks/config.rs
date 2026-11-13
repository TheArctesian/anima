//! A check's settings from an agent (MCP-004): the K1 `CheckConfigInput`, one key for the
//! kind, with only the fields that never hold a secret. An HTTP header value, body or
//! authentication and an SNMP community or password are refused: they go through
//! `anima check create --from-file`, which reads them from a file, never from an agent.

use serde::de::DeserializeOwned;
use serde_json::{Map, Value};

use crate::mcp::outcome::ToolError;

/// The fields an agent may give, by kind. Anything else is refused, so a secret field that
/// K1 gains later is refused too.
const ALLOWED: [(&str, &[&str]); 12] = [
    ("icmp", &["timeoutMs", "ipVersion"]),
    ("tcp", &["port", "timeoutMs"]),
    ("tls", &["port", "serverName", "timeoutMs"]),
    (
        "http",
        &[
            "url",
            "method",
            "maxRedirects",
            "acceptedStatus",
            "keyword",
            "jsonQuery",
            "jsonExpected",
            "verifyTls",
            "timeoutMs",
        ],
    ),
    ("dns", &["name", "recordType", "resolver", "expected"]),
    ("push", &[]),
    ("group", &["childIds"]),
    ("docker", &["container", "timeoutMs"]),
    (
        "snmp",
        &[
            "port",
            "version",
            "oid",
            "expected",
            "username",
            "timeoutMs",
        ],
    ),
    ("tailscalePing", &["target", "timeoutMs"]),
    ("systemd", &["unit", "timeoutMs"]),
    ("domainExpiry", &["domain", "warnDays", "timeoutMs"]),
];

/// `config` as the generated `CheckConfigInput` of one document.
///
/// # Errors
/// [`ToolError::Refused`] for a shape that is not one kind, an unknown kind, a field that can
/// hold a secret, or a value K1 would not take.
pub fn typed<T: DeserializeOwned>(config: &Value) -> Result<T, ToolError> {
    check(config)?;
    serde_json::from_value(config.clone()).map_err(|error| {
        ToolError::Refused(format!(
            "The check config does not fit K1 CheckConfigInput: {error}. Correct it and call \
             again."
        ))
    })
}

fn check(config: &Value) -> Result<(), ToolError> {
    let kinds: Vec<&str> = ALLOWED.iter().map(|(kind, _)| *kind).collect();
    let refused = |text: String| ToolError::Refused(text);
    let Some((kind, settings)) = config
        .as_object()
        .filter(|object| object.len() == 1)
        .and_then(|object| object.iter().next())
    else {
        return Err(refused(format!(
            "config must be an object with exactly one key, the kind: one of {}. Example: \
             {{\"tcp\": {{\"port\": 22}}}}.",
            kinds.join(", ")
        )));
    };
    let Some((_, allowed)) = ALLOWED.iter().find(|(name, _)| name == kind) else {
        return Err(refused(format!(
            "{} is not a check kind; use one of {}.",
            crate::mcp::untrusted::capped(kind, 40),
            kinds.join(", ")
        )));
    };
    let empty = Map::new();
    let fields = match settings {
        Value::Object(fields) => fields,
        Value::Bool(_) if kind == "push" => &empty,
        _ => return Err(refused(format!("config.{kind} must be an object."))),
    };
    let secret: Vec<&str> = fields
        .keys()
        .map(String::as_str)
        .filter(|field| !allowed.contains(field))
        .collect();
    if secret.is_empty() {
        return Ok(());
    }
    Err(refused(format!(
        "These fields of config.{kind} are not taken here because they can hold a secret or \
         are unknown: {}. The operator sets them with `anima check create --from-file` (or \
         `anima check update --from-file`). Allowed: {}.",
        secret
            .iter()
            .map(|field| crate::mcp::untrusted::capped(field, 40))
            .collect::<Vec<_>>()
            .join(", "),
        allowed.join(", ")
    )))
}

/// Whether the check's current settings hold a secret that a new config without it would
/// drop: an HTTP check with authentication or headers.
#[must_use]
pub fn holds_secret(current: &Value) -> bool {
    let http = current["__typename"] == "HttpConfig";
    let auth = current["authKind"]
        .as_str()
        .is_some_and(|kind| kind != "NONE");
    let headers = current["headers"]
        .as_array()
        .is_some_and(|headers| !headers.is_empty());
    http && (auth || headers)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn refusal(config: &Value) -> String {
        check(config).map_or_else(|error| error.to_string(), |()| String::new())
    }

    #[test]
    fn mcp_004_plain_settings_of_each_kind_pass() {
        assert_eq!(refusal(&json!({ "tcp": { "port": 22 } })), "");
        assert_eq!(refusal(&json!({ "push": true })), "");
        assert_eq!(
            refusal(&json!({ "http": { "url": "https://a.example", "keyword": "ok" } })),
            ""
        );
        assert_eq!(
            refusal(&json!({ "snmp": { "oid": "1.3.6", "version": "V3", "username": "u" } })),
            ""
        );
    }

    #[test]
    fn mcp_004_fields_that_can_hold_a_secret_are_refused_and_point_to_from_file() {
        for config in [
            json!({ "http": { "url": "https://a.example", "auth": { "bearerToken": "t" } } }),
            json!({ "http": { "url": "https://a.example", "headers": [] } }),
            json!({ "http": { "url": "https://a.example", "body": "x" } }),
            json!({ "snmp": { "oid": "1.3.6", "community": "c" } }),
            json!({ "snmp": { "oid": "1.3.6", "authPassword": "p", "privacyPassword": "q" } }),
        ] {
            let text = refusal(&config);
            assert!(text.contains("--from-file"), "{config}: {text}");
            assert!(!text.contains("\"t\"") && !text.contains("\"p\""), "{text}");
        }
    }

    #[test]
    fn mcp_004_a_config_must_be_one_known_kind() {
        assert!(refusal(&json!({ "tcp": { "port": 1 }, "icmp": {} })).contains("exactly one"));
        assert!(refusal(&json!({ "ftp": {} })).contains("not a check kind"));
        assert!(refusal(&json!("tcp")).contains("exactly one"));
    }

    #[test]
    fn mcp_004_an_http_check_with_authentication_holds_a_secret() {
        let plain = json!({ "__typename": "HttpConfig", "authKind": "NONE", "headers": [] });
        assert!(!holds_secret(&plain));
        let bearer = json!({ "__typename": "HttpConfig", "authKind": "BEARER", "headers": [] });
        assert!(holds_secret(&bearer));
        let headers = json!({ "__typename": "HttpConfig", "authKind": "NONE",
                              "headers": [{ "name": "X-Key" }] });
        assert!(holds_secret(&headers));
        assert!(!holds_secret(
            &json!({ "__typename": "TcpConfig", "port": 22 })
        ));
    }
}
