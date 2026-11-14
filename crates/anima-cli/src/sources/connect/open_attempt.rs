//! `anima source connect github --state` and `--cancel`: what the hub holds of the one attempt.

use serde_json::Value;

use super::attempt::Attempt;
use super::documents::{
    GithubConnectCancel, GithubConnectState, github_connect_cancel, github_connect_state,
};
use super::lines::{NONE_OPEN, ending_words, state_text};
use crate::failure::Failure;
use crate::hub::Hub;
use crate::invocation::Invocation;

/// Prints the attempt, or that none is in progress.
///
/// # Errors
/// The hub's refusal.
pub async fn show(hub: &Hub, invocation: &Invocation) -> Result<(), Failure> {
    let current = current(hub).await?;
    match current {
        Some((attempt, value)) => print(invocation, &value, &state_text(&attempt)),
        None => print(invocation, &Value::Null, NONE_OPEN),
    }
}

/// Cancels the open attempt and prints it; says so when none is open.
///
/// # Errors
/// The hub's refusal (`CONFLICT` when it connected in the meantime).
pub async fn cancel(hub: &Hub, invocation: &Invocation) -> Result<(), Failure> {
    let open = current(hub)
        .await?
        .filter(|(attempt, _)| !attempt.phase.ended());
    let Some((attempt, _)) = open else {
        return print(invocation, &Value::Null, NONE_OPEN);
    };
    let answer = hub
        .run::<GithubConnectCancel>(github_connect_cancel::Variables { id: attempt.id })
        .await?;
    let (cancelled, value) = Attempt::read(&answer.cancel_github_connect)?;
    print(invocation, &value, &ending_words(&cancelled))
}

async fn current(hub: &Hub) -> Result<Option<(Attempt, Value)>, Failure> {
    let answer = hub
        .run::<GithubConnectState>(github_connect_state::Variables)
        .await?;
    answer
        .github_connect
        .as_ref()
        .map(Attempt::read)
        .transpose()
}

/// `value` with `--json`, else `words`, on standard output.
fn print(invocation: &Invocation, value: &Value, words: &str) -> Result<(), Failure> {
    if invocation.output.json() {
        invocation.output.raw(value)
    } else {
        invocation
            .output
            .line(&crate::output::text::clean_lines(words))
    }
}
