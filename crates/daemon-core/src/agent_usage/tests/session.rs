//! A session made of several files: a sub-agent's use adds to its session's, and what is sent
//! keeps the K8 limits (AIS-018, K8 Section 4).

use super::tally;
use crate::agent_usage::{MAX_MODELS, OTHER_MODEL, Session, Source};
use crate::wire::BatchSessionDay;

fn reply(model: &str, output: u64) -> String {
    format!(
        r#"{{"type":"assistant","timestamp":"2026-11-08T09:00:00Z","message":{{"model":"{model}","content":[{{"type":"tool_use"}}],"usage":{{"output_tokens":{output}}}}}}}"#
    )
}

#[test]
fn ais_018_a_sub_agent_file_adds_to_its_session() {
    let main = tally(Source::ClaudeCode, &reply("claude-opus-5-5", 10));
    let agent = tally(Source::ClaudeCode, &reply("claude-opus-5-5", 5));
    let mut session = Session::new(
        Source::ClaudeCode,
        "s1",
        Some("github.com/me/app".to_owned()),
    );
    session.add(&main);
    session.add(&agent);
    let days = session.session_days();
    assert_eq!(days.len(), 1);
    assert_eq!(days[0].tool_calls, 2);
    assert_eq!(days[0].models[0].tokens.output, 15);
    assert_eq!(days[0].repository.as_deref(), Some("github.com/me/app"));
    assert!(BatchSessionDay::from(&days[0]).fits());
}

#[test]
fn ais_018_a_day_of_more_than_16_models_sends_the_smallest_as_other() {
    let lines: Vec<String> = (1..=20_u64)
        .map(|rank| reply(&format!("model-{rank:02}"), rank * 100))
        .collect();
    let tally = tally(Source::ClaudeCode, &lines.join("\n"));
    let mut session = Session::new(Source::ClaudeCode, "s1", None);
    session.add(&tally);
    let day = &session.session_days()[0];
    assert_eq!(day.models.len(), MAX_MODELS);
    let other = day
        .models
        .iter()
        .find(|one| one.model == OTHER_MODEL)
        .expect("the smallest share one name");
    assert_eq!(other.tokens.output, 100 + 200 + 300 + 400 + 500);
    let total: u64 = day.models.iter().map(|one| one.tokens.output).sum();
    assert_eq!(total, (1..=20).sum::<u64>() * 100, "no token is lost");
    assert!(BatchSessionDay::from(day).fits());
}

#[test]
fn ais_018_a_session_whose_id_or_repository_breaks_the_rule_is_not_sent_as_such() {
    let tally = tally(Source::ClaudeCode, &reply("m", 1));
    let mut bad_id = Session::new(Source::ClaudeCode, "../etc/passwd", None);
    bad_id.add(&tally);
    assert_eq!(bad_id.session_days().len(), 0);
    let mut bad_repository =
        Session::new(Source::ClaudeCode, "s1", Some("/Users/me/app".to_owned()));
    bad_repository.add(&tally);
    assert_eq!(bad_repository.session_days()[0].repository, None);
}

#[test]
fn ais_018_a_session_with_no_time_sends_nothing() {
    let tally = tally(
        Source::ClaudeCode,
        r#"{"type":"assistant","message":{"model":"m"}}"#,
    );
    let mut session = Session::new(Source::ClaudeCode, "s1", None);
    session.add(&tally);
    assert_eq!(session.session_days().len(), 0);
}
