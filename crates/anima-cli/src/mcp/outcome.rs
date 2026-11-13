//! A tool's answer as an MCP result (MCP-010): the value as structured content, or a tool
//! error (`isError`) with one sentence that says what to do next. No sentence holds the token:
//! no error the crate makes holds it (CMD-014).

use rmcp::model::{CallToolResult, ContentBlock};
use serde_json::Value;

use super::untrusted::capped;
use crate::failure::Failure;
use crate::hub::{Code, HubError, TokenKind};

/// The longest hub message passed on; it could echo a name a device chose.
const MESSAGE_CAP: usize = 300;

/// Why a tool did not run to the end.
#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    /// The hub refused, or a lookup found nothing.
    #[error(transparent)]
    Failed(#[from] Failure),
    /// The tool itself refused before it asked the hub: a wrong `confirm`, a secret field,
    /// arguments that do not fit the schema.
    #[error("{0}")]
    Refused(String),
}

impl From<HubError> for ToolError {
    fn from(error: HubError) -> Self {
        Self::Failed(Failure::Hub(error))
    }
}

/// The MCP result of a tool's outcome.
#[must_use]
pub fn result(outcome: Result<Value, ToolError>, token: TokenKind) -> CallToolResult {
    match outcome {
        Ok(value) => CallToolResult::structured(value),
        Err(error) => CallToolResult::error(vec![ContentBlock::text(sentence(&error, token))]),
    }
}

/// What went wrong and what to do next, in one or two sentences.
#[must_use]
pub fn sentence(error: &ToolError, token: TokenKind) -> String {
    let failure = match error {
        ToolError::Refused(message) => return message.clone(),
        ToolError::Failed(failure) => failure,
    };
    let message = capped(&failure.to_string(), MESSAGE_CAP);
    match failure {
        Failure::Hub(error) => hub_sentence(error, &message, token),
        Failure::NotFound { .. } => format!(
            "{message} Call anima_devices_search to find the right device ID or exact name \
             (anima_device_get lists a device's checks)."
        ),
        Failure::Usage { .. } => format!("{message} Give the ID instead of the name."),
        Failure::SignedOut { .. } => SIGN_IN.to_owned(),
        _ => format!("{message} This is a problem on the operator's computer; tell the operator."),
    }
}

fn hub_sentence(error: &HubError, message: &str, token: TokenKind) -> String {
    match error.code() {
        Code::NotFound => format!(
            "The hub found no such thing ({message}). Call anima_devices_search to find the \
             right ID or exact name."
        ),
        Code::Forbidden => match token {
            TokenKind::ApiKey => "The hub refused: this API key may not do this. It may have \
                                  read scope only, or the action needs a login token. Tell the \
                                  operator; do not retry."
                .to_owned(),
            TokenKind::Login => {
                format!("The hub refused this action ({message}). Tell the operator.")
            }
        },
        Code::Unauthenticated => SIGN_IN.to_owned(),
        Code::Validation => {
            let fields: Vec<String> = error
                .fields()
                .iter()
                .map(|field| {
                    format!(
                        "{}: {}",
                        capped(&field.field, MESSAGE_CAP),
                        capped(&field.message, MESSAGE_CAP)
                    )
                })
                .collect();
            if fields.is_empty() {
                format!("The hub rejected the input ({message}). Correct it and call again.")
            } else {
                format!(
                    "The hub rejected these fields: {}. Correct them and call again.",
                    fields.join("; ")
                )
            }
        }
        Code::Conflict => format!(
            "{message} It clashes with what is there now: read the current state again first."
        ),
        Code::Unavailable => {
            format!("The hub or a part it needs is not available ({message}). Try again later.")
        }
        Code::Internal => {
            format!("The hub failed ({message}). Retry once at most, then tell the operator.")
        }
    }
}

const SIGN_IN: &str = "The hub no longer accepts this server's token. The operator must sign \
                       in again (`anima auth login`, or a new ANIMA_TOKEN) and restart the server.";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hub::FieldError;

    fn refused(code: Code, fields: Vec<FieldError>) -> ToolError {
        ToolError::from(HubError::Refused {
            code,
            message: "nope\u{1b}[2J".to_owned(),
            fields,
        })
    }

    #[test]
    fn mcp_010_each_hub_code_says_what_to_do_next() {
        let not_found = sentence(&refused(Code::NotFound, vec![]), TokenKind::Login);
        assert!(not_found.contains("anima_devices_search"), "{not_found}");
        assert!(!not_found.contains('\u{1b}'), "{not_found}");
        let key = sentence(&refused(Code::Forbidden, vec![]), TokenKind::ApiKey);
        assert!(key.contains("read scope"), "{key}");
        let signed_out = sentence(&refused(Code::Unauthenticated, vec![]), TokenKind::ApiKey);
        assert!(signed_out.contains("anima auth login"), "{signed_out}");
        let fields = vec![FieldError {
            field: "input.name".to_owned(),
            message: "is empty".to_owned(),
        }];
        let validation = sentence(&refused(Code::Validation, fields), TokenKind::Login);
        assert!(validation.contains("input.name: is empty"), "{validation}");
    }

    #[test]
    fn mcp_010_an_error_is_a_tool_result_with_is_error() {
        let shown = result(Err(ToolError::Refused("No.".to_owned())), TokenKind::Login);
        assert_eq!(shown.is_error, Some(true));
        let fine = result(Ok(serde_json::json!({ "a": 1 })), TokenKind::Login);
        assert_eq!(fine.is_error, Some(false));
        assert!(fine.structured_content.is_some());
    }
}
