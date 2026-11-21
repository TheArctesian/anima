//! The `accounts` member of `agentUsage` (K8 Section 4; AIS-022, R-NEW-accountgroup): a key, a
//! plan and limit windows only; held to fixed alphabets and sizes; absent keeps what the hub has.

use super::*;
use crate::agent_usage::account::account_key;
use crate::wire::{BatchAccount, BatchLimit};

const KEY: &str = "3f9a0c55e1d24b7aa0b1c2d3e4f5061728394a5b6c7d8e9f00112233445566ff";

fn account(source: &str) -> BatchAccount {
    BatchAccount {
        source: source.to_owned(),
        key: KEY.to_owned(),
        plan: Some("Pro".to_owned()),
        limits: vec![BatchLimit {
            window: "five_hour".to_owned(),
            used_percent: Some(42),
            resets_at: None,
            reached: false,
        }],
    }
}

fn with_accounts(accounts: Vec<BatchAccount>) -> impl FnOnce(&mut Batch) {
    with(move |usage| usage.accounts = Some(accounts))
}

#[test]
fn ais_022_no_accounts_member_keeps_what_the_hub_has() {
    let usage = checked(with(|_| {})).unwrap().agent_usage.unwrap();
    assert_eq!(usage.accounts, None);
}

#[test]
fn ais_022_an_account_reaches_the_hub_with_its_key_plan_and_windows() {
    let usage = checked(with_accounts(vec![account("codex")]))
        .unwrap()
        .agent_usage
        .unwrap();
    let accounts = usage.accounts.unwrap();
    assert_eq!(accounts[0].source, Source::Codex);
    assert_eq!(accounts[0].key, KEY);
    assert_eq!(accounts[0].plan.as_deref(), Some("Pro"));
    assert_eq!(accounts[0].limits[0].used_percent, Some(42));
}

#[test]
fn ais_022_an_empty_list_asks_for_the_salt_and_an_unknown_source_is_dropped() {
    let usage = checked(with_accounts(vec![account("gemini")]))
        .unwrap()
        .agent_usage
        .unwrap();
    assert_eq!(usage.accounts, Some(Vec::new()));
}

#[test]
fn ais_022_an_account_that_could_name_a_person_is_refused() {
    let named = |change: fn(&mut BatchAccount)| {
        let mut one = account("codex");
        change(&mut one);
        with_accounts(vec![one])
    };
    assert_malformed(
        "an address as the key",
        named(|one| one.key = "ann@example.com".to_owned()),
    );
    assert_malformed(
        "an upper-case key",
        named(|one| one.key = KEY.to_uppercase()),
    );
    assert_malformed(
        "an address as the plan",
        named(|one| one.plan = Some("ann@example.com".to_owned())),
    );
    assert_malformed(
        "a share over 100",
        named(|one| one.limits[0].used_percent = Some(101)),
    );
    assert_malformed(
        "a window in capitals",
        named(|one| one.limits[0].window = "WEEK".to_owned()),
    );
    assert_malformed(
        "two of one source",
        with_accounts(vec![account("codex"), account("codex")]),
    );
}

#[test]
fn ais_022_the_key_is_the_same_for_one_hub_and_different_for_another() {
    let hub = [7_u8; 32];
    let other = [8_u8; 32];
    let id = "user-0192f5c4-acct";
    let key = account_key(&hub, Source::Codex, id);
    assert_eq!(
        key,
        account_key(&hub, Source::Codex, id),
        "two machines of one hub"
    );
    assert_ne!(
        key,
        account_key(&other, Source::Codex, id),
        "a different hub"
    );
    assert_ne!(
        key,
        account_key(&hub, Source::ClaudeCode, id),
        "a different program"
    );
    assert!(crate::agent_usage::account::is_key(&key));
    assert!(!key.contains(id), "the key does not hold the identifier");
}
