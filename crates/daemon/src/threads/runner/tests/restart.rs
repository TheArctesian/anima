//! A thread after a restart of the runner (THR-030).

use super::*;

#[test]
fn thr_030_a_restored_thread_keeps_the_id_of_its_open_request() {
    let mut thread = held_claude();
    let request = asked(&thread.line(CLAUDE_ASK, &mut counter()))[0];
    let saved = serde_json::to_string(&thread.saved()).expect("saved");
    let mut again = Thread::restore(serde_json::from_str(&saved).expect("reads"), None);
    assert_eq!(
        again.row.state,
        RunState::NeedsYou,
        "not Starting after a restart"
    );
    let line = again
        .answer(request, &Answer::Allow)
        .expect("the request is still open");
    assert_eq!(
        json(&line)["response"]["response"]["updatedInput"]["command"],
        "ls"
    );
}

#[test]
fn thr_030_an_answer_that_went_before_a_restart_is_never_given_twice() {
    let mut thread = held_claude();
    let request = asked(&thread.line(CLAUDE_ASK, &mut counter()))[0];
    thread
        .answer(request, &Answer::Allow)
        .expect("the first answer");
    let saved = serde_json::to_string(&thread.saved()).expect("saved");
    let mut again = Thread::restore(serde_json::from_str(&saved).expect("reads"), None);
    assert_eq!(
        again.answer(request, &Answer::Allow),
        Err(Refusal::RequestGone),
        "the answer may have reached the program"
    );
    let steps = again.settle_answering();
    assert!(
        steps
            .iter()
            .any(|step| matches!(step, Step::Change(Change::Closed { .. })))
    );
    assert_eq!(again.row.open.len(), 0);
}

#[test]
fn th11_a_line_that_changes_nothing_of_the_record_keeps_its_key() {
    let mut thread = held_claude();
    let key = thread.record_key();
    let _ = thread.line(CLAUDE_ASK, &mut counter());
    assert_ne!(
        thread.record_key(),
        key,
        "an open request changes the record"
    );
}
