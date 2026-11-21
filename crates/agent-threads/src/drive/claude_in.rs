//! The lines that the runner writes to the standard input of `claude -p --input-format
//! stream-json` (TH10 Section 2b.1, TH18 Section 2.3). Each is one line of JSON with no line end.

use serde_json::{Value, json};

use super::DriveError;
use super::request::{Answer, ProgramRequestId, Request};

/// The writer of the input lines of Claude Code.
pub struct ClaudeInput;

impl ClaudeInput {
    /// A prompt of the person. A prompt sent during a turn is queued by the program (TH10 2.2).
    #[must_use]
    pub fn prompt(text: &str) -> String {
        json!({"type": "user", "message": {"role": "user", "content": text}}).to_string()
    }

    /// Stops the turn and keeps the process (TH10 2b.1).
    #[must_use]
    pub fn interrupt(id: &str) -> String {
        json!({"type": "control_request", "request_id": id, "request": {"subtype": "interrupt"}})
            .to_string()
    }

    /// The answer to a request. "Allow for the session" sends back one rule that the request
    /// itself offered, never a rule of the runner (THR-030).
    ///
    /// # Errors
    /// [`DriveError::AnswerDoesNotFit`] when the answer does not fit the request.
    pub fn answer(request: &Request, answer: &Answer) -> Result<String, DriveError> {
        answer.check(request)?;
        let response = match answer {
            Answer::Allow => json!({"behavior": "allow", "updatedInput": request.input}),
            Answer::AllowForSession { rule } => json!({
                "behavior": "allow",
                "updatedInput": request.input,
                "updatedPermissions": [request.offered[*rule]],
            }),
            Answer::Deny { message } => json!({"behavior": "deny", "message": message}),
            Answer::Choose { answers } => {
                let mut input = match &request.input {
                    Value::Object(_) => request.input.clone(),
                    _ => Value::Object(serde_json::Map::new()),
                };
                let chosen: serde_json::Map<String, Value> = answers
                    .iter()
                    .map(|(key, choice)| (key.clone(), Value::from(choice.as_str())))
                    .collect();
                input["answers"] = Value::Object(chosen);
                json!({"behavior": "allow", "updatedInput": input})
            }
        };
        Ok(success(&request.id, &response))
    }

    /// The refusal of a control request of a kind the runner does not know (THR-046).
    #[must_use]
    pub fn refuse(id: &ProgramRequestId, why: &str) -> String {
        json!({
            "type": "control_response",
            "response": {"subtype": "error", "request_id": id.json(), "error": why},
        })
        .to_string()
    }
}

fn success(id: &ProgramRequestId, response: &Value) -> String {
    json!({
        "type": "control_response",
        "response": {"subtype": "success", "request_id": id.json(), "response": response},
    })
    .to_string()
}
