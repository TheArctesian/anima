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
    assert_eq!(
        requests(&events)[0].session_rules,
        ["Bash(touch t1.txt)"],
        "only the allow rule is offered, not the folder or the mode"
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
        request.session_rules,
        ["Bash(mkdir d10 *)"],
        "the rule that sets the mode is not offered"
    );
    let session =
        line(&ClaudeInput::answer(&request, &Answer::AllowForSession { rule: 0 }).expect("fits"));
    let sent = &session["response"]["response"]["updatedPermissions"][0];
    assert_eq!(
        sent["destination"], "session",
        "no settings file is written"
    );
    assert_eq!(sent["rules"][0]["ruleContent"], "mkdir d10 *");
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
fn claude_account_members_of_a_control_reply_are_dropped() {
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

fn session_rules_of(suggestion: &str) -> Vec<String> {
    let text = CLAUDE_BASH.replace(
        r#"{"type":"setMode","mode":"acceptEdits","destination":"session"}"#,
        suggestion,
    );
    one_request(claude(&text)).session_rules
}

#[test]
fn thr_030_a_claude_session_rule_is_offered_in_words_and_sent_with_known_fields_only() {
    let text = CLAUDE_BASH.replace(
        r#"{"type":"setMode","mode":"acceptEdits","destination":"session"}"#,
        r#"{"type":"addRules","rules":[{"toolName":"Bash","ruleContent":"mkdir d10 *"},{"toolName":"Edit","ruleContent":"src/**","extra":1}],"behavior":"allow","destination":"projectSettings","hooks":{"x":1}}"#,
    );
    let request = one_request(claude(&text));
    assert_eq!(
        request.session_rules,
        ["Bash(mkdir d10 *)", "Bash(mkdir d10 *), Edit(src/**)"]
    );
    let session =
        line(&ClaudeInput::answer(&request, &Answer::AllowForSession { rule: 1 }).expect("fits"));
    assert_eq!(
        session["response"]["response"]["updatedPermissions"][0],
        json!({"type": "addRules", "behavior": "allow", "destination": "session",
               "rules": [{"toolName": "Bash", "ruleContent": "mkdir d10 *"},
                         {"toolName": "Edit", "ruleContent": "src/**"}]})
    );
}

#[test]
fn thr_030_a_rule_for_every_use_of_a_tool_or_for_the_settings_is_not_offered() {
    let refused = [
        r#"{"type":"addRules","rules":[{"toolName":"Bash"}],"behavior":"allow","destination":"session"}"#,
        r#"{"type":"addRules","rules":[{"toolName":"Edit","ruleContent":"**"}],"behavior":"allow","destination":"session"}"#,
        r#"{"type":"addRules","rules":[{"toolName":"Write","ruleContent":"/w/.claude/settings.local.json"}],"behavior":"allow","destination":"session"}"#,
        r#"{"type":"addRules","rules":[{"toolName":"Bash","ruleContent":"ls"}],"behavior":"deny","destination":"session"}"#,
        r#"{"type":"addDirectories","directories":["/"],"destination":"session"}"#,
    ];
    for suggestion in refused {
        assert_eq!(
            session_rules_of(suggestion),
            ["Bash(mkdir d10 *)"],
            "{suggestion}"
        );
    }
    let long = format!(
        r#"{{"type":"addRules","rules":[{{"toolName":"Bash","ruleContent":"{}"}}],"behavior":"allow","destination":"session"}}"#,
        "a".repeat(2000)
    );
    assert_eq!(
        session_rules_of(&long),
        ["Bash(mkdir d10 *)"],
        "a rule is offered whole or not at all"
    );
}

#[test]
fn thr_125_a_claude_file_change_shows_its_lines_and_no_hidden_marks() {
    let edit = r#"{"type":"control_request","request_id":"e1","request":{"subtype":"can_use_tool","tool_name":"Edit","input":{"file_path":"/w/a\u202e.txt","old_string":"one\u0007","new_string":"two\nthree"},"tool_use_id":"toolu_03"}}"#;
    let request = one_request(claude(edit));
    assert_eq!(request.subject, "/w/a.txt");
    assert_eq!(request.diff.as_deref(), Some("- one\n+ two\n+ three"));
    let write = r#"{"type":"control_request","request_id":"e2","request":{"subtype":"can_use_tool","tool_name":"Write","input":{"file_path":"/w/b.txt","content":"new"},"tool_use_id":"toolu_04"}}"#;
    assert_eq!(one_request(claude(write)).diff.as_deref(), Some("+ new"));
    let every = edit.replace(r#""new_string""#, r#""replace_all":true,"new_string""#);
    assert!(
        one_request(claude(&every))
            .diff
            .is_some_and(|diff| diff.ends_with("(each place in the file)"))
    );
    assert_eq!(one_request(claude(CLAUDE_BASH)).diff, None);
}

/// The reply of Claude Code 2.1.311 with no sign-in it could read (TH99, measured on this Mac on
/// 2026-11-21 with an empty configuration folder): a made assistant line with an error, then a
/// result with the subtype "success" and `is_error`.
const NOT_LOGGED_IN: &str = concat!(
    r#"{"type":"assistant","message":{"model":"<synthetic>","role":"assistant","content":[{"type":"text","text":"Not logged in · Please run /login"}]},"session_id":"s","error":"authentication_failed"}"#,
    "\n",
    r#"{"type":"result","subtype":"success","is_error":true,"result":"Not logged in · Please run /login","terminal_reason":"api_error","session_id":"s","permission_denials":[]}"#,
);

#[test]
fn th99_not_logged_in_ends_the_turn_as_failed_with_the_fault_of_the_sign_in() {
    let events = claude(NOT_LOGGED_IN);
    assert!(events.iter().any(|event| matches!(
        event,
        DriveEvent::TurnEnded {
            outcome: TurnOutcome::Failed,
            detail: Some(detail),
            ..
        } if detail == "authentication_failed"
    )));
    let next_turn = claude(&NOT_LOGGED_IN.replace(r#","error":"authentication_failed""#, ""));
    assert!(next_turn.iter().any(|event| matches!(
        event,
        DriveEvent::TurnEnded {
            outcome: TurnOutcome::Failed,
            detail: Some(detail),
            ..
        } if detail == "api_error"
    )));
}
