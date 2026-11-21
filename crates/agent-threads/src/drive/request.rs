//! A request of a program to the person, and the answer to it (THR-030, THR-032, THR-117).

use serde::{Deserialize, Serialize};

/// The ID that the program gave a request: a string (Claude Code `request_id`) or the JSON-RPC
/// `id` of a server request (Codex, a number in TH10).
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ProgramRequestId {
    Number(i64),
    Text(String),
}

impl ProgramRequestId {
    /// The ID as JSON, as the program wrote it.
    #[must_use]
    pub fn json(&self) -> serde_json::Value {
        match self {
            Self::Number(number) => serde_json::Value::from(*number),
            Self::Text(text) => serde_json::Value::from(text.as_str()),
        }
    }

    pub(super) fn from_json(value: &serde_json::Value) -> Option<Self> {
        match value {
            serde_json::Value::Number(number) => number.as_i64().map(Self::Number),
            serde_json::Value::String(text) => Some(Self::Text(text.clone())),
            _ => None,
        }
    }
}

/// What a request asks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RequestKind {
    /// May the program run a command.
    Command,
    /// May the program change a file.
    FileChange,
    /// May the program use a different tool.
    Tool,
    /// The agent asks the person a question with choices.
    Question,
}

/// One question of an agent, with its choices.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Question {
    /// The key of the answer: the text of the question (Claude Code) or its `id` (Codex).
    pub key: String,
    pub header: Option<String>,
    pub text: String,
    pub choices: Vec<String>,
    pub many: bool,
}

/// A request of the program, as the program made it (THR-032: the view shows what the request
/// names, from the request itself).
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Request {
    pub id: ProgramRequestId,
    pub kind: RequestKind,
    /// The tool name (Claude Code) or the item ID (Codex).
    pub tool: String,
    /// The command, the path or the input of the tool, as text.
    pub subject: String,
    /// The diff of a file change, when the program gave one.
    pub diff: Option<String>,
    /// The input of the tool as the program sent it; an allow sends it back unchanged.
    #[serde(skip)]
    pub input: serde_json::Value,
    /// The rules that the program offered for "allow for the session", as text for the view.
    pub session_rules: Vec<String>,
    /// The same rules as the program wrote them; only one of these may be sent back (THR-030).
    #[serde(skip)]
    pub offered: Vec<serde_json::Value>,
    pub questions: Vec<Question>,
}

/// The answer of the person to a [`Request`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "answer", rename_all = "snake_case")]
pub enum Answer {
    Allow,
    /// Allow, and send the rule at this index of [`Request::offered`].
    AllowForSession {
        rule: usize,
    },
    Deny {
        message: String,
    },
    /// The answers to the questions, as (key, choice) pairs.
    Choose {
        answers: Vec<(String, String)>,
    },
}

impl Answer {
    /// The answer fits the request: a question gets choices that name its questions, an
    /// approval gets no choices, and a session rule is one that the request offered.
    pub(super) fn check(&self, request: &Request) -> Result<(), super::DriveError> {
        use super::DriveError::AnswerDoesNotFit;
        let question = request.kind == RequestKind::Question;
        match self {
            Self::Choose { answers } => {
                if !question {
                    return Err(AnswerDoesNotFit("choices for an approval"));
                }
                let known = |key: &String| request.questions.iter().any(|q| &q.key == key);
                if answers.is_empty() || !answers.iter().all(|(key, _)| known(key)) {
                    return Err(AnswerDoesNotFit(
                        "a choice for a question the request has not",
                    ));
                }
                Ok(())
            }
            Self::AllowForSession { rule } if *rule >= request.offered.len() => {
                Err(AnswerDoesNotFit("a rule the request did not offer"))
            }
            Self::Allow | Self::AllowForSession { .. } if question => {
                Err(AnswerDoesNotFit("an approval for a question"))
            }
            _ => Ok(()),
        }
    }
}
