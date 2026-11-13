//! The person's approval of one call of a tool in state `ask` (MCP-013): an MCP elicitation
//! request in form mode with one yes-or-no field. Anything but a clear yes refuses the call
//! before the hub is asked: a client without elicitation, a decline, a cancel, an error, or no
//! answer within [`ASK_TIMEOUT`].

use std::time::Duration;

use rmcp::model::JsonObject;
use rmcp::service::ElicitationError;
use rmcp::{Peer, RoleServer};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;

use super::definition::Definition;
use super::untrusted::{capped, cleaned};

/// How long the server waits for the person's answer.
pub const ASK_TIMEOUT: Duration = Duration::from_secs(120);

/// The most characters of arguments the question shows.
const ARGUMENTS_CAP: usize = 1_000;

/// The form the person fills in.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct Approval {
    /// Run this one call now.
    approve: bool,
}

rmcp::elicit_safe!(Approval);

/// Why a call was not approved.
#[derive(Debug, PartialEq, Eq)]
pub enum Refusal {
    /// The client did not declare form elicitation, so it cannot ask the person.
    NoAsking,
    /// The person said no or closed the question.
    Declined,
    /// No answer in time, or the request failed.
    NoAnswer,
}

/// Asks the person at the client to approve one call; `Ok` only for a clear yes. Stops
/// waiting when `cancelled` ends (the agent cancelled the call).
///
/// # Errors
/// [`Refusal`] saying why the call must not run.
pub async fn ask(
    peer: &Peer<RoleServer>,
    cancelled: impl Future<Output = ()>,
    definition: &Definition,
    arguments: Option<&JsonObject>,
) -> Result<(), Refusal> {
    let question = question(definition, arguments);
    let asked = peer.elicit_with_timeout::<Approval>(question, Some(ASK_TIMEOUT));
    tokio::select! {
        answer = asked => verdict(&answer),
        () = cancelled => Err(Refusal::Declined),
    }
}

/// The answer as a decision.
///
/// # Errors
/// [`Refusal`] for anything but `approve: true`.
pub fn verdict(answer: &Result<Option<Approval>, ElicitationError>) -> Result<(), Refusal> {
    match answer {
        Ok(Some(Approval { approve: true })) => Ok(()),
        Ok(_) | Err(ElicitationError::UserDeclined | ElicitationError::UserCancelled) => {
            Err(Refusal::Declined)
        }
        Err(ElicitationError::CapabilityNotSupported) => Err(Refusal::NoAsking),
        Err(_) => Err(Refusal::NoAnswer),
    }
}

/// The tool result's sentence for `refusal` (MCP-010).
#[must_use]
pub fn sentence(refusal: &Refusal, tool: &str) -> String {
    let why = match refusal {
        Refusal::NoAsking => "this MCP client cannot ask the operator (it did not declare \
                              the elicitation capability)"
            .to_owned(),
        Refusal::Declined => "the operator did not approve it".to_owned(),
        Refusal::NoAnswer => format!(
            "the operator did not answer within {} seconds",
            ASK_TIMEOUT.as_secs()
        ),
    };
    format!(
        "Not run, and the hub was not contacted: {tool} needs the operator's approval for each \
         call, and {why}. Do not retry on your own. The operator must approve the call when \
         asked, or allow the tool with `anima mcp permissions set --allow {tool}`."
    )
}

/// What the person is asked: the tool, what it does, and the arguments the agent gave,
/// cleaned and capped. A key the tool's schema does not name is shown by name only.
#[must_use]
pub fn question(definition: &Definition, arguments: Option<&JsonObject>) -> String {
    let known = definition
        .input
        .get("properties")
        .and_then(Value::as_object);
    let mut shown = serde_json::Map::new();
    for (key, value) in arguments.into_iter().flatten() {
        let value = if known.is_some_and(|names| names.contains_key(key)) {
            cleaned(value.clone())
        } else {
            Value::String("(not a field of this tool; the call will be refused)".to_owned())
        };
        shown.insert(capped(key, 100), value);
    }
    let arguments = capped(&Value::Object(shown).to_string(), ARGUMENTS_CAP);
    format!(
        "An agent asks to run the Anima tool {}: {} Arguments from the agent: {arguments}. \
         Approve this one call?",
        definition.name,
        capped(definition.description, 300)
    )
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::mcp::catalogue::Access;
    use crate::mcp::definition::Effect;

    #[derive(JsonSchema)]
    #[expect(dead_code, reason = "only the schema of the type is used")]
    struct Input {
        /// The check.
        check: String,
    }

    fn definition() -> Definition {
        Definition::new::<Input>(
            "anima_check_delete",
            Access::Write,
            Effect::Removes,
            "Deletes a check.",
        )
    }

    #[test]
    fn mcp_013_only_a_clear_yes_approves() {
        assert_eq!(verdict(&Ok(Some(Approval { approve: true }))), Ok(()));
        let no = verdict(&Ok(Some(Approval { approve: false })));
        assert_eq!(no, Err(Refusal::Declined));
        assert_eq!(verdict(&Ok(None)), Err(Refusal::Declined));
        let declined = verdict(&Err(ElicitationError::UserDeclined));
        assert_eq!(declined, Err(Refusal::Declined));
        let cancelled = verdict(&Err(ElicitationError::UserCancelled));
        assert_eq!(cancelled, Err(Refusal::Declined));
        let no_asking = verdict(&Err(ElicitationError::CapabilityNotSupported));
        assert_eq!(no_asking, Err(Refusal::NoAsking));
        let timeout = rmcp::ServiceError::Timeout {
            timeout: ASK_TIMEOUT,
        };
        let late = verdict(&Err(ElicitationError::Service(timeout)));
        assert_eq!(late, Err(Refusal::NoAnswer));
        assert_eq!(
            verdict(&Err(ElicitationError::NoContent)),
            Err(Refusal::NoAnswer)
        );
    }

    #[test]
    fn mcp_013_the_refusal_says_how_the_operator_allows_the_tool() {
        let text = sentence(&Refusal::NoAsking, "anima_check_delete");
        assert!(
            text.contains("anima mcp permissions set --allow anima_check_delete"),
            "{text}"
        );
        assert!(text.contains("elicitation"), "{text}");
        assert!(sentence(&Refusal::NoAnswer, "x").contains("120 seconds"));
    }

    #[test]
    fn mcp_013_the_question_shows_cleaned_capped_arguments_and_hides_unknown_values() {
        let arguments = json!({
            "check": format!("web\u{1b}[2J{}", "x".repeat(2_000)),
            "password": "hunter2",
        });
        let Value::Object(arguments) = arguments else {
            unreachable!("an object")
        };
        let text = question(&definition(), Some(&arguments));
        assert!(text.starts_with("An agent asks to run the Anima tool anima_check_delete"));
        assert!(!text.contains('\u{1b}'), "{text}");
        assert!(!text.contains("hunter2"), "{text}");
        assert!(text.contains("password"), "{text}");
        assert!(text.chars().count() < 1_600, "{}", text.len());
    }
}
