//! The shapes of RED-002 from the fixture (RED-004), the provider keys and the text of many
//! lines. The rules of event text are pinned by the hub's `log_007` tests.

use std::collections::BTreeSet;

use crate::{redact_document, redact_text};

const SHAPES: &str = include_str!("fixtures/secret-shapes.txt");

/// The shapes RED-002 names. Each must have a line in the fixture.
const REQUIRED: [&str; 12] = [
    "anthropic",
    "openai",
    "github",
    "aws",
    "google",
    "slack",
    "stripe",
    "jwt",
    "bearer",
    "private-key",
    "url-password",
    "password-assignment",
];

/// One fixture line: the shape, the text with the secret in it, and the secret.
fn shapes() -> Vec<(String, String, String)> {
    SHAPES
        .lines()
        .filter(|line| !line.starts_with('#') && !line.trim().is_empty())
        .map(|line| {
            let line = line.replace('¦', "").replace("\\n", "\n");
            let (shape, marked) = line.split_once(": ").expect("`<shape>: <text>`");
            let (before, rest) = marked.split_once('«').expect("a secret starts at «");
            let (secret, after) = rest.split_once('»').expect("a secret ends at »");
            (
                shape.to_owned(),
                format!("{before}{secret}{after}"),
                secret.to_owned(),
            )
        })
        .collect()
}

#[test]
fn red_004_the_fixture_has_each_shape_of_red_002() {
    let found: BTreeSet<String> = shapes().into_iter().map(|(shape, _, _)| shape).collect();

    for shape in REQUIRED {
        assert!(found.contains(shape), "no fixture line for {shape}");
    }
}

#[test]
fn red_002_no_secret_of_the_fixture_stays_in_the_text() {
    for (shape, text, secret) in shapes() {
        let redacted = redact_document(&text);

        assert!(!redacted.contains(&secret), "{shape}: {redacted}");
        assert!(redacted.contains("[redacted]"), "{shape}: {redacted}");
        assert_eq!(redact_document(&redacted), redacted, "{shape}: not stable");
    }
}

#[test]
fn red_002_a_provider_key_is_masked_with_no_word_before_it() {
    let key = ["sk-ant-", "api03-FAKEfake0123456789FAKEfake"].concat();

    assert_eq!(
        redact_text(&format!("ran with {key}, then stopped")),
        "ran with [redacted], then stopped"
    );
    assert_eq!(redact_text(&format!("({key})")), "([redacted])");
}

#[test]
fn red_002_text_that_only_looks_like_a_provider_key_stays() {
    for text in [
        "sk-learn-is-a-fine-library-for-this",
        "the risk-0123456789012345678901 ticket",
        "AKIA is the prefix of an access key id",
        "AKIAFAKE0123",
        "xoxb-short",
        "commit 3059c851a0f4e2b7c6d5e4f3a2b1c0d9e8f7a6b5",
        "ghp_lowercaseonlywithnodigitsorcapitalsinit",
    ] {
        assert_eq!(redact_text(text), text);
    }
}

#[test]
fn red_001_a_document_loses_only_the_lines_of_a_private_key() {
    let text = [
        "first line",
        "-----BEGIN OPENSSH PRIVATE KEY-----",
        "b3BlbnNzaC1rZXk=",
        "-----END OPENSSH PRIVATE KEY-----",
        "password=hunter2 on the last line",
    ]
    .join("\r\n");

    assert_eq!(
        redact_document(&text),
        "first line\r\n[redacted]\r\npassword=[redacted] on the last line"
    );
}

#[test]
fn red_001_a_putty_key_file_in_a_document_is_one_redacted_line() {
    let text = "note\nPuTTY-User-Key-File-3: ssh-ed25519\nPrivate-Lines: 1\nAAAAfake\nPrivate-MAC: 00ff\nafter\n";

    assert_eq!(redact_document(text), "note\n[redacted]\nafter\n");
}

#[test]
fn red_003_a_private_key_cut_by_a_clip_takes_the_rest_of_the_document() {
    let text = "before\n-----BEGIN PRIVATE KEY-----\nMIIEfake\nmore of the key";

    assert_eq!(redact_document(text), "before\n[redacted]\n");
}

#[test]
fn red_001_a_document_of_one_line_is_redacted_as_event_text() {
    for text in [
        "host=db password=hunter2 user=anima",
        "curl -H 'Authorization: Bearer x' https://hub",
        "nothing secret here",
        "",
    ] {
        assert_eq!(redact_document(text), redact_text(text));
    }
}
