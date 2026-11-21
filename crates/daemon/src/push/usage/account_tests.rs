//! The accounts of a batch (AIS-022 as R-313 changed it): only a key, a plan and limit windows
//! leave the machine, never the identifier or the name of an account.

use agent_threads::drive::AccountFacts;
use base64::Engine as _;
use daemon_core::agent_usage::account::account_key;
use daemon_core::agent_usage::{Source, UsageState};
use serde_json::json;

use super::UsageBacklog;
use crate::sampling::agent_usage::{ProgramAccount, UsageReport};

const SALT: [u8; 32] = [7; 32];
const ID: &str = "user-7f3a9c-ID";
const NAME: &str = "person-7f3a9c@example.com";

fn codex(facts: serde_json::Value) -> ProgramAccount {
    ProgramAccount {
        source: Source::Codex,
        facts: serde_json::from_value::<AccountFacts>(facts).expect("facts"),
    }
}

fn signed_in() -> ProgramAccount {
    codex(json!({
        "id": ID, "name": NAME, "plan": "pro",
        "limits": [{ "window": "300 min", "used_percent": 35, "resets_at": 1_795_000_000, "reached": false }]
    }))
}

fn report(accounts: Option<Vec<ProgramAccount>>) -> UsageReport {
    UsageReport {
        state: UsageState::Reading,
        session_days: Vec::new(),
        accounts,
    }
}

fn salt_text() -> String {
    base64::engine::general_purpose::STANDARD.encode(SALT)
}

#[test]
fn ais_022_with_no_salt_an_empty_list_asks_for_it_then_the_key_goes() {
    let mut backlog = UsageBacklog::default();
    backlog.take(report(Some(vec![signed_in()])));
    let asking = backlog.outgoing().expect("a batch");
    assert_eq!(asking.accounts, Some(Vec::new()), "no salt: an empty list");
    backlog.done();
    backlog.take_salt(Some(&salt_text()));
    let sent = backlog.outgoing().expect("the accounts");
    let accounts = sent.accounts.expect("the accounts member");
    assert_eq!(accounts.len(), 1);
    assert_eq!(accounts[0].source, "codex");
    assert_eq!(accounts[0].key, account_key(&SALT, Source::Codex, ID));
    assert_eq!(accounts[0].plan.as_deref(), Some("pro"));
    assert_eq!(accounts[0].limits[0].window, "300 min");
    assert_eq!(accounts[0].limits[0].used_percent, Some(35));
}

#[test]
fn ais_022_the_identifier_and_the_name_of_an_account_never_leave() {
    let mut backlog = UsageBacklog::default();
    backlog.take_salt(Some(&salt_text()));
    backlog.take(report(Some(vec![signed_in()])));
    let sent = backlog.outgoing().expect("a batch");
    let text = serde_json::to_string(&sent).expect("json");
    assert!(!text.contains("7f3a9c"), "{text}");
    assert!(!text.contains("example.com"), "{text}");
}

#[test]
fn ais_022_accounts_go_again_only_when_they_change() {
    let mut backlog = UsageBacklog::default();
    backlog.take_salt(Some(&salt_text()));
    backlog.take(report(Some(vec![signed_in()])));
    assert!(backlog.outgoing().is_some());
    backlog.done();
    backlog.take(report(Some(vec![signed_in()])));
    assert_eq!(backlog.outgoing(), None, "nothing new");
    backlog.take(report(None));
    assert_eq!(
        backlog.outgoing(),
        None,
        "no runner: the sent accounts stay"
    );
    let mut fuller = signed_in();
    fuller.facts.limits[0].used_percent = Some(90);
    backlog.take(report(Some(vec![fuller])));
    let sent = backlog.outgoing().expect("the change");
    assert_eq!(
        sent.accounts.expect("accounts")[0].limits[0].used_percent,
        Some(90)
    );
}

#[test]
fn ais_022_a_program_with_no_identifier_or_signed_out_sends_no_account() {
    let mut backlog = UsageBacklog::default();
    backlog.take_salt(Some(&salt_text()));
    let claude = ProgramAccount {
        source: Source::ClaudeCode,
        facts: serde_json::from_value(json!({
            "limits": [{ "window": "five_hour", "used_percent": 50, "resets_at": null, "reached": false }]
        }))
        .expect("facts"),
    };
    let out = codex(json!({ "limits": [], "signed_out": true }));
    backlog.take(report(Some(vec![claude, out])));
    let sent = backlog.outgoing().expect("a batch");
    assert_eq!(sent.accounts, Some(Vec::new()));
}

#[test]
fn ais_022_a_salt_that_does_not_read_is_ignored() {
    let mut backlog = UsageBacklog::default();
    backlog.take_salt(Some("not base64!"));
    backlog.take_salt(Some(
        &base64::engine::general_purpose::STANDARD.encode([1_u8; 8]),
    ));
    backlog.take(report(Some(vec![signed_in()])));
    assert_eq!(
        backlog.outgoing().expect("a batch").accounts,
        Some(Vec::new())
    );
}
