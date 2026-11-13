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

mod question;

use super::definition::Definition;
use super::outcome::OPERATOR_ONLY;
use question::{MAX_LENGTH, MAX_VALUES, TooLarge, question};

/// How long the server waits for the person's answer.
pub const ASK_TIMEOUT: Duration = Duration::from_secs(120);

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
    /// The arguments are too many or too long to show in full, so the person was not asked.
    TooLarge { values: usize },
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
    let question = question(definition, arguments)
        .map_err(|TooLarge { values }| Refusal::TooLarge { values })?;
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
        Refusal::TooLarge { values } => {
            return format!(
                "Not run, and the hub was not contacted: the operator must see every argument \
                 of {tool} to approve it, and these {values} values are too many or too long to \
                 show in full (at most {MAX_VALUES} values and a question of {MAX_LENGTH} \
                 characters). Send a smaller change: fewer or shorter values in one call."
            );
        }
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
         call, and {why}. Do not retry on your own: tell the operator. {OPERATOR_ONLY}"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn mcp_013_f4_the_refusal_sends_the_agent_to_the_operator_without_a_command() {
        for refusal in [
            Refusal::NoAsking,
            Refusal::Declined,
            Refusal::NoAnswer,
            Refusal::TooLarge { values: 50 },
        ] {
            let text = sentence(&refusal, "anima_check_delete");
            assert!(!text.contains("anima mcp"), "{text}");
            assert!(!text.contains('`'), "{text}");
        }
        let text = sentence(&Refusal::NoAsking, "anima_check_delete");
        assert!(text.contains("ask the operator"), "{text}");
        assert!(text.contains("must not change Anima's settings"), "{text}");
        assert!(text.contains("elicitation"), "{text}");
        assert!(sentence(&Refusal::NoAnswer, "x").contains("120 seconds"));
    }
}
