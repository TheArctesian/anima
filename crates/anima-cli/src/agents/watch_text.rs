//! One update of a thread as one line of text for `anima agents watch`. Pure. The words of the
//! program are shown as text: `stream::line` cleans each line of control characters.

use agent_threads::control::{Change, RunState};
use agent_threads::drive::{DriveEvent, RequestKind};

/// The most characters of a reply, a command or a result on one line.
const WIDTH: usize = 200;

fn short(text: &str) -> String {
    let one: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if one.chars().count() > WIDTH {
        format!("{}…", one.chars().take(WIDTH).collect::<String>())
    } else {
        one
    }
}

const fn state_words(state: RunState) -> &'static str {
    match state {
        RunState::Starting => "starting",
        RunState::Working => "working",
        RunState::NeedsYou => "needs you",
        RunState::Idle => "your turn",
        RunState::Stopped => "stopped",
        RunState::Unknown => "unknown",
    }
}

const fn kind_words(kind: RequestKind) -> &'static str {
    match kind {
        RequestKind::Command => "run a command",
        RequestKind::FileChange => "change a file",
        RequestKind::Tool => "use a tool",
        RequestKind::Question => "answer a question",
    }
}

/// The line of a change, or `None` for a change that says nothing to a person (a piece of a
/// reply that the whole reply repeats).
#[must_use]
pub fn text(change: &Change) -> Option<String> {
    Some(match change {
        Change::Prompt { text, source } => format!("prompt ({source}): {}", short(text)),
        Change::Asked { request, detail } => format!(
            "NEEDS YOU: may it {}? {} (request {request})",
            kind_words(detail.kind),
            short(&detail.subject)
        ),
        Change::Closed { request, why } => format!("request {request} closed: {why}"),
        Change::State { state, row } => match &row.ended {
            Some(why) if *state == RunState::Stopped => format!("stopped: {why}"),
            _ => format!("state: {}", state_words(*state)),
        },
        Change::Said { event } => said(event)?,
    })
}

fn said(event: &DriveEvent) -> Option<String> {
    Some(match event {
        DriveEvent::Session { id, model, .. } => {
            format!(
                "session {id} ({})",
                model.as_deref().unwrap_or("model not named")
            )
        }
        DriveEvent::Message { text, .. } => format!("reply: {}", short(text)),
        DriveEvent::ToolCall { name, input, .. } => format!("tool {name}: {}", short(input)),
        DriveEvent::ToolOutput { output, failed, .. } => {
            let word = if *failed { "failed" } else { "result" };
            format!("{word}: {}", short(output))
        }
        DriveEvent::TurnEnded {
            outcome, detail, ..
        } => {
            let outcome = serde_json::to_value(outcome).ok()?;
            match detail {
                Some(detail) => format!("turn ended: {} ({})", outcome.as_str()?, short(detail)),
                None => format!("turn ended: {}", outcome.as_str()?),
            }
        }
        DriveEvent::Notice { text } => format!("notice: {}", short(text)),
        DriveEvent::Unknown {
            what,
            stops_turn: true,
        } => {
            format!("the runner does not know {what}; the turn was stopped")
        }
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_request_says_that_it_needs_the_person_with_its_id() {
        let detail: agent_threads::drive::Request = serde_json::from_value(serde_json::json!({
            "id": "r", "kind": "command", "tool": "Bash", "subject": "rm -rf\nbuild",
            "diff": null, "session_rules": [], "questions": []
        }))
        .expect("a request");
        let request = uuid::Uuid::nil();
        let line = text(&Change::Asked { request, detail }).expect("a line");
        assert_eq!(
            line,
            format!("NEEDS YOU: may it run a command? rm -rf build (request {request})")
        );
    }
}
