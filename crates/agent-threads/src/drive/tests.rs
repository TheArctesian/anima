//! Tests of the drive protocols, with the lines of the TH10 and TH18 measurements.
//! The three files in `tests/fixtures/` are whole runs of TH18, with the paths shortened.

use serde_json::Value;

use super::*;

mod claude;
mod codex;
mod launch;

const CLAUDE_FULL: &str = include_str!("tests/fixtures/claude-full-access.jsonl");
const CLAUDE_ASK: &str = include_str!("tests/fixtures/claude-ask.jsonl");
const CODEX_FULL: &str = include_str!("tests/fixtures/codex-full-access.jsonl");

fn claude(text: &str) -> Vec<DriveEvent> {
    let mut decoder = ClaudeDecoder::new();
    text.lines()
        .flat_map(|line| decoder.read(line.as_bytes()))
        .collect()
}

fn codex(text: &str) -> Vec<DriveEvent> {
    let mut decoder = CodexDecoder::new();
    text.lines()
        .flat_map(|line| decoder.read(line.as_bytes()))
        .collect()
}

fn requests(events: &[DriveEvent]) -> Vec<&Request> {
    events
        .iter()
        .filter_map(|event| match event {
            DriveEvent::Request(request) => Some(request),
            _ => None,
        })
        .collect()
}

fn ends(events: &[DriveEvent]) -> Vec<TurnOutcome> {
    events
        .iter()
        .filter_map(|event| match event {
            DriveEvent::TurnEnded { outcome, .. } => Some(*outcome),
            _ => None,
        })
        .collect()
}

fn stops(events: &[DriveEvent]) -> bool {
    events.iter().any(|event| {
        matches!(
            event,
            DriveEvent::Unknown {
                stops_turn: true,
                ..
            }
        )
    })
}

fn line(text: &str) -> Value {
    serde_json::from_str(text).expect("one line of JSON")
}

fn one_request(events: Vec<DriveEvent>) -> Request {
    match events.into_iter().next() {
        Some(DriveEvent::Request(request)) => request,
        other => panic!("expected a request, got {other:?}"),
    }
}
