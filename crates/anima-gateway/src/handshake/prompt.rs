//! Asking the operator for a credential the hub did not send (GW-015): the gateway sends
//! `required` with the names, and the client answers each with an `argv` stream: `argv`,
//! `blob` with the value in base64, `end` (guacamole-server `src/libguac/argv.c`).
//!
//! The hub's relay lets the client answer only the names of the latest `required`, once
//! each (K4 Section 4). The gateway checks the same again.

use base64::Engine as _;
use guac_protocol::Instruction;
use zeroize::Zeroizing;

use super::opening::{LONGEST_SECRET, Secret};

/// `GUAC_PROTOCOL_STATUS_SUCCESS`.
const SUCCESS: &str = "0";
/// `GUAC_PROTOCOL_STATUS_CLIENT_BAD_REQUEST`.
const BAD_REQUEST: &str = "768";

/// A credential the operator can be asked for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Credential {
    Username,
    Password,
}

impl Credential {
    const fn name(self) -> &'static str {
        match self {
            Self::Username => "username",
            Self::Password => "password",
        }
    }
}

/// What the caller does with one instruction from the client while a prompt is open.
#[derive(Debug)]
pub enum PromptStep {
    /// The instruction is not part of an answer: handle it as any other.
    NotAnAnswer,
    /// Part of an answer: send this `ack` to the client.
    Acknowledge(Instruction),
    /// One answer is complete: send the `ack`, and use the value.
    Answered {
        credential: Credential,
        value: Secret,
        ack: Instruction,
    },
}

/// One open `required`, and the answer stream being read.
pub struct Prompt {
    wanted: Vec<Credential>,
    /// The stream index of the answer being read, what it answers, and its bytes so far.
    reading: Option<(String, Credential, Zeroizing<Vec<u8>>)>,
    /// The stream index of the last answer that was refused. What the client still sends
    /// on it is dropped, so the rest of a refused password never passes for anything else,
    /// such as clipboard text on a stream with the same index.
    refused: Option<String>,
}

impl Prompt {
    /// Asks for `wanted`, each once, and gives the `required` instruction to send.
    #[must_use]
    pub fn ask(wanted: &[Credential]) -> (Self, Instruction) {
        let mut once = Vec::new();
        for credential in wanted {
            if !once.contains(credential) {
                once.push(*credential);
            }
        }
        let prompt = Self {
            wanted: once,
            reading: None,
            refused: None,
        };
        let required = prompt.required();
        (prompt, required)
    }

    /// `required` with the names still asked for. The hub lets the client answer each name
    /// of a `required` once (K4 Section 4), so after a refused answer this is sent again.
    #[must_use]
    pub fn required(&self) -> Instruction {
        Instruction::new("required", self.wanted.iter().map(|wanted| wanted.name()))
    }

    /// Whether every credential asked for has been answered.
    #[must_use]
    pub const fn is_answered(&self) -> bool {
        self.wanted.is_empty() && self.reading.is_none()
    }

    /// Takes one instruction from the client.
    pub fn on_instruction(&mut self, instruction: &Instruction) -> PromptStep {
        let args = &instruction.args;
        let reading_this = |reading: &Option<(String, _, _)>| {
            reading
                .as_ref()
                .is_some_and(|(stream, ..)| args.first() == Some(stream))
        };
        let refused = self
            .refused
            .as_ref()
            .is_some_and(|stream| args.first() == Some(stream));
        match instruction.opcode.as_str() {
            "argv" => self.on_argv(args),
            "blob" | "end" if refused => PromptStep::Acknowledge(ack(&args[0], BAD_REQUEST)),
            "blob" if reading_this(&self.reading) => self.on_blob(args),
            "end" if reading_this(&self.reading) => self.on_end(),
            _ => PromptStep::NotAnAnswer,
        }
    }

    /// `argv,<stream>,<mimetype>,<name>`: an answer begins, when the name was asked for.
    fn on_argv(&mut self, args: &[String]) -> PromptStep {
        let (Some(stream), Some(name)) = (args.first(), args.get(2)) else {
            return PromptStep::NotAnAnswer;
        };
        let asked = self.wanted.iter().position(|wanted| wanted.name() == name);
        match (asked, &self.reading) {
            (Some(at), None) => {
                let credential = self.wanted.remove(at);
                // Its full size from the start: a buffer that grows would leave parts of
                // the value behind in memory that is freed and not zeroed.
                let bytes = Zeroizing::new(Vec::with_capacity(LONGEST_SECRET));
                self.reading = Some((stream.clone(), credential, bytes));
                self.refused = None;
                PromptStep::Acknowledge(ack(stream, SUCCESS))
            }
            _ => PromptStep::Acknowledge(ack(stream, BAD_REQUEST)),
        }
    }

    /// `blob,<stream>,<base64>`: more of the answer. One that does not decode, or makes
    /// the answer too long, ends the stream. The credential is asked for again with
    /// [`Prompt::required`].
    fn on_blob(&mut self, args: &[String]) -> PromptStep {
        let Some((stream, credential, mut bytes)) = self.reading.take() else {
            return PromptStep::NotAnAnswer;
        };
        let more = args
            .get(1)
            .map(|text| base64::engine::general_purpose::STANDARD.decode(text));
        let more = more.and_then(Result::ok).map(Zeroizing::new);
        match more {
            Some(more) if bytes.len() + more.len() <= LONGEST_SECRET => {
                bytes.extend_from_slice(&more);
                let ack = ack(&stream, SUCCESS);
                self.reading = Some((stream, credential, bytes));
                PromptStep::Acknowledge(ack)
            }
            _ => self.refuse(stream, credential),
        }
    }

    /// Ends the answer on `stream` as refused; `credential` is asked for again.
    fn refuse(&mut self, stream: String, credential: Credential) -> PromptStep {
        self.wanted.push(credential);
        let ack = ack(&stream, BAD_REQUEST);
        self.refused = Some(stream);
        PromptStep::Acknowledge(ack)
    }

    /// `end,<stream>`: the answer is complete, when it is text and not empty.
    fn on_end(&mut self) -> PromptStep {
        let Some((stream, credential, bytes)) = self.reading.take() else {
            return PromptStep::NotAnAnswer;
        };
        match std::str::from_utf8(&bytes) {
            Ok(text) if !text.is_empty() => {
                let value = Secret::new(text.to_owned());
                let ack = ack(&stream, SUCCESS);
                PromptStep::Answered {
                    credential,
                    value,
                    ack,
                }
            }
            _ => self.refuse(stream, credential),
        }
    }
}

/// `ack,<stream>,<message>,<status>`.
fn ack(stream: &str, status: &str) -> Instruction {
    let message = if status == SUCCESS { "OK" } else { "Refused" };
    Instruction::new("ack", [stream, message, status])
}
