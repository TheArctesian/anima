//! The protocol of Claude Code: TH10 Section 2b and the TH18 runs.

use serde_json::json;

use super::*;

#[test]
fn a_claude_turn_in_full_access_asks_only_the_question() {
    let events = claude(CLAUDE_FULL);
    assert!(!stops(&events));
    assert!(events.iter().any(|event| matches!(event, DriveEvent::Session { mode: Some(mode), .. } if mode == "bypassPermissions")));
    let asked = requests(&events);
    assert_eq!(asked.len(), 1);
    assert_eq!(asked[0].kind, RequestKind::Question);
    assert_eq!(asked[0].questions[0].choices, ["Red", "Blue"]);
    let refused = events.iter().find_map(|event| match event {
        DriveEvent::ToolOutput {
            failed: true,
            decided_by,
            ..
        } => decided_by.clone(),
        _ => None,
    });
    assert_eq!(
        refused.as_deref(),
        Some("config"),
        "the deny rule of the project answered"
    );
    assert!(matches!(
        events.last(),
        Some(DriveEvent::State {
            state: ProgramState::Idle
        })
    ));
    assert!(events.iter().any(|event| matches!(
        event,
        DriveEvent::TurnEnded {
            outcome: TurnOutcome::Completed,
            refused: 1,
            ..
        }
    )));
    assert!(
        events
            .iter()
            .any(|event| matches!(event, DriveEvent::TextDelta { .. }))
    );
}

#[test]
fn a_claude_turn_that_asks_gives_each_request_with_its_subject() {
    let events = claude(CLAUDE_ASK);
    assert!(!stops(&events));
    let asked: Vec<(RequestKind, &str)> = requests(&events)
        .iter()
        .map(|request| (request.kind, request.subject.as_str()))
        .collect();
    assert_eq!(asked[0], (RequestKind::Command, "touch t1.txt"));
    assert_eq!(asked[1], (RequestKind::FileChange, "/w/note.txt"));
    assert_eq!(asked[3].0, RequestKind::Question);
    assert!(
        !requests(&events)[0].offered.is_empty(),
        "Claude Code offered session rules"
    );
}

#[test]
fn a_claude_interrupt_is_read_from_the_result_and_not_the_exit_code() {
    let events = claude(
        r#"{"type":"result","subtype":"error_during_execution","is_error":true,"result":null,"terminal_reason":"aborted_tools","stop_reason":"tool_use"}"#,
    );
    assert_eq!(ends(&events), [TurnOutcome::Interrupted]);
    let failed =
        claude(r#"{"type":"result","subtype":"error_max_turns","terminal_reason":"max_turns"}"#);
    assert_eq!(ends(&failed), [TurnOutcome::Failed]);
}

const CLAUDE_BASH: &str = r#"{"type":"control_request","request_id":"b01d8ead","request":{"subtype":"can_use_tool","tool_name":"Bash","display_name":"Bash","input":{"command":"mkdir d10","description":"Create directory d10"},"permission_suggestions":[{"type":"addRules","rules":[{"toolName":"Bash","ruleContent":"mkdir d10 *"}],"behavior":"allow","destination":"localSettings"},{"type":"setMode","mode":"acceptEdits","destination":"session"}],"tool_use_id":"toolu_01"}}"#;

#[test]
fn a_claude_answer_sends_back_the_input_and_only_an_offered_rule() {
    let request = one_request(claude(CLAUDE_BASH));
    assert_eq!(request.id, ProgramRequestId::Text("b01d8ead".into()));
    let allow = line(&ClaudeInput::answer(&request, &Answer::Allow).expect("fits"));
    assert_eq!(allow["response"]["request_id"], "b01d8ead");
    assert_eq!(
        allow["response"]["response"]["updatedInput"]["command"],
        "mkdir d10"
    );
    assert_eq!(
        request.offered.len(),
        1,
        "the rule for the settings file is not offered"
    );
    let session =
        line(&ClaudeInput::answer(&request, &Answer::AllowForSession { rule: 0 }).expect("fits"));
    assert_eq!(
        session["response"]["response"]["updatedPermissions"][0]["mode"],
        "acceptEdits"
    );
    assert!(ClaudeInput::answer(&request, &Answer::AllowForSession { rule: 1 }).is_err());
    let choose = Answer::Choose {
        answers: vec![("Red?".into(), "Red".into())],
    };
    assert!(
        ClaudeInput::answer(&request, &choose).is_err(),
        "no choices for an approval"
    );
    let deny = line(
        &ClaudeInput::answer(
            &request,
            &Answer::Deny {
                message: "no".into(),
            },
        )
        .expect("fits"),
    );
    assert_eq!(
        deny["response"]["response"],
        json!({"behavior": "deny", "message": "no"})
    );
}

#[test]
fn a_claude_question_is_answered_with_choices_keyed_by_its_text() {
    let question = r#"{"type":"control_request","request_id":"bd9a","request":{"subtype":"can_use_tool","tool_name":"AskUserQuestion","input":{"questions":[{"question":"Do you prefer red or blue?","header":"Color","options":[{"label":"Red"},{"label":"Blue"}],"multiSelect":false}]},"tool_use_id":"toolu_02","requires_user_interaction":true}}"#;
    let request = one_request(claude(question));
    assert!(
        ClaudeInput::answer(&request, &Answer::Allow).is_err(),
        "a question needs choices"
    );
    let wrong = Answer::Choose {
        answers: vec![("Something else?".into(), "Red".into())],
    };
    assert!(ClaudeInput::answer(&request, &wrong).is_err());
    let answer = Answer::Choose {
        answers: vec![("Do you prefer red or blue?".into(), "Red".into())],
    };
    let sent = line(&ClaudeInput::answer(&request, &answer).expect("fits"));
    let input = &sent["response"]["response"]["updatedInput"];
    assert_eq!(input["answers"]["Do you prefer red or blue?"], "Red");
    assert_eq!(input["questions"][0]["header"], "Color");
}

#[test]
fn a_claude_line_of_an_unknown_kind_stops_the_turn() {
    assert!(stops(&claude(r#"{"type":"something_new"}"#)));
    assert!(stops(&claude(
        r#"{"type":"control_request","request_id":"x","request":{"subtype":"elicit"}}"#
    )));
    assert!(stops(&claude("not json")));
    assert!(
        !stops(&claude(r#"{"type":"system","subtype":"hook_started"}"#)),
        "an unknown system kind is shown, not fatal"
    );
    let refusal = line(&ClaudeInput::refuse(
        &ProgramRequestId::Text("x".into()),
        "unknown",
    ));
    assert_eq!(refusal["response"]["subtype"], "error");
}

#[test]
fn claude_account_members_are_dropped() {
    assert_eq!(
        claude(r#"{"type":"rate_limit_event","rate_limit_info":{"status":"allowed"}}"#),
        []
    );
    let reply = claude(
        r#"{"type":"control_response","response":{"subtype":"success","request_id":"init-1","response":{"account":{"email":"a@b"}}}}"#,
    );
    assert_eq!(
        reply,
        [DriveEvent::Reply {
            id: ProgramRequestId::Text("init-1".into()),
            ok: true,
            result: Value::Null
        }]
    );
}

#[test]
fn a_claude_session_is_named_again_only_when_its_id_changes() {
    let init = |id: &str| {
        format!(
            r#"{{"type":"system","subtype":"init","session_id":"{id}","permissionMode":"default"}}"#
        )
    };
    let events = claude(&[init("a"), init("a"), init("b")].join("\n"));
    let ids: Vec<&str> = events
        .iter()
        .filter_map(|event| match event {
            DriveEvent::Session { id, .. } => Some(id.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(ids, ["a", "b"]);
}
