//! The operator is asked for a missing credential, and only the asked names are taken
//! (GW-015).

use anima_gateway::handshake::{Credential, Prompt, PromptStep};
use base64::Engine as _;
use guac_protocol::Instruction;

fn base64(bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

fn argv(stream: &str, name: &str) -> Instruction {
    Instruction::new("argv", [stream, "text/plain", name])
}

fn blob(stream: &str, bytes: &[u8]) -> Instruction {
    Instruction::new("blob", [stream.to_owned(), base64(bytes)])
}

fn end(stream: &str) -> Instruction {
    Instruction::new("end", [stream])
}

/// The status of the `ack` the step sends, and the answer when one is complete.
fn outcome(step: PromptStep) -> (Option<String>, Option<(Credential, String)>) {
    match step {
        PromptStep::NotAnAnswer => (None, None),
        PromptStep::Acknowledge(ack) => (Some(ack.args[2].clone()), None),
        PromptStep::Answered {
            credential,
            value,
            ack,
        } => (
            Some(ack.args[2].clone()),
            Some((credential, value.expose().to_owned())),
        ),
    }
}

#[test]
fn required_names_what_is_missing() {
    let (_, required) = Prompt::ask(&[Credential::Username, Credential::Password]);
    assert_eq!(
        (required.opcode.as_str(), required.args),
        (
            "required",
            vec!["username".to_owned(), "password".to_owned()]
        )
    );
}

#[test]
fn an_answer_arrives_over_several_blobs() {
    let (mut prompt, _) = Prompt::ask(&[Credential::Password]);
    assert_eq!(
        outcome(prompt.on_instruction(&argv("3", "password")))
            .0
            .as_deref(),
        Some("0")
    );
    assert_eq!(
        outcome(prompt.on_instruction(&blob("3", b"correct ")))
            .0
            .as_deref(),
        Some("0")
    );
    prompt.on_instruction(&blob("3", b"horse"));
    assert!(!prompt.is_answered());
    let (status, answer) = outcome(prompt.on_instruction(&end("3")));
    assert_eq!(status.as_deref(), Some("0"));
    assert_eq!(
        answer,
        Some((Credential::Password, "correct horse".to_owned()))
    );
    assert!(prompt.is_answered());
}

#[test]
fn a_name_that_was_not_asked_for_is_refused() {
    let (mut prompt, _) = Prompt::ask(&[Credential::Password]);
    for name in ["hostname", "username", "port", ""] {
        assert_eq!(
            outcome(prompt.on_instruction(&argv("1", name)))
                .0
                .as_deref(),
            Some("768"),
            "{name}"
        );
    }
    assert!(matches!(
        prompt.on_instruction(&blob("1", b"x")),
        PromptStep::NotAnAnswer
    ));
    assert!(!prompt.is_answered());
}

#[test]
fn each_name_is_answered_once() {
    let (mut prompt, _) = Prompt::ask(&[Credential::Password]);
    prompt.on_instruction(&argv("1", "password"));
    prompt.on_instruction(&blob("1", b"first"));
    assert!(outcome(prompt.on_instruction(&end("1"))).1.is_some());
    assert_eq!(
        outcome(prompt.on_instruction(&argv("2", "password")))
            .0
            .as_deref(),
        Some("768")
    );
}

#[test]
fn an_answer_that_is_too_long_or_not_text_leaves_the_name_asked_for() {
    let (mut prompt, _) = Prompt::ask(&[Credential::Password]);
    prompt.on_instruction(&argv("1", "password"));
    assert_eq!(
        outcome(prompt.on_instruction(&blob("1", &[b'p'; 4097])))
            .0
            .as_deref(),
        Some("768")
    );
    assert!(matches!(
        prompt.on_instruction(&end("1")),
        PromptStep::NotAnAnswer
    ));
    assert!(!prompt.is_answered());

    prompt.on_instruction(&argv("2", "password"));
    prompt.on_instruction(&blob("2", &[0xff, 0xfe]));
    let (status, answer) = outcome(prompt.on_instruction(&end("2")));
    assert_eq!((status.as_deref(), answer), (Some("768"), None));

    prompt.on_instruction(&argv("3", "password"));
    prompt.on_instruction(&Instruction::new("blob", ["3", "not base64 !"]));
    assert!(!prompt.is_answered());

    prompt.on_instruction(&argv("4", "password"));
    prompt.on_instruction(&blob("4", b"right"));
    assert_eq!(
        outcome(prompt.on_instruction(&end("4"))).1,
        Some((Credential::Password, "right".to_owned()))
    );
}

#[test]
fn other_instructions_and_other_streams_are_not_answers() {
    let (mut prompt, _) = Prompt::ask(&[Credential::Password]);
    prompt.on_instruction(&argv("1", "password"));
    for other in [
        blob("2", b"clipboard"),
        end("2"),
        Instruction::new("key", ["65", "1"]),
    ] {
        assert!(matches!(
            prompt.on_instruction(&other),
            PromptStep::NotAnAnswer
        ));
    }
}
