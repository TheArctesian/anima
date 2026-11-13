use chrono::{TimeZone as _, Utc};
use source_exchange::{Acquisition, Minting};

use super::*;

fn app_key() -> source_exchange::AppKey {
    let Acquisition::TokenPull(Minting::AppKey(key)) = DECLARATION.acquisition else {
        panic!("GitHub is a token pull with an App key");
    };
    key
}

#[test]
fn src_002_the_installation_token_is_minted_with_a_post_to_the_installation() {
    assert_eq!(
        (app_key().mint_path)("4321"),
        "/app/installations/4321/access_tokens"
    );
}

#[test]
fn src_002_the_mint_reply_gives_the_token_and_its_expiry() {
    let minted = (app_key().parse_minted)(&fixture("installation_token.json"))
        .ok()
        .unwrap();

    assert_eq!(minted.token, "ghs_exampleexampleexampleexampleexample");
    assert_eq!(
        minted.expires_at,
        Utc.with_ymd_and_hms(2026, 11, 13, 16, 42, 0).unwrap()
    );
}

#[test]
fn src_002_an_unreadable_mint_reply_never_quotes_the_reply() {
    let body = br#"{"token":"ghs_secret_value","expires_at":"not a date"}"#;

    let Err(problem) = (app_key().parse_minted)(body) else {
        panic!("the reply is not valid");
    };

    assert_eq!(problem.code, "token_unreadable");
    assert!(!problem.message.contains("ghs_secret_value"));
    assert!(!problem.subject.contains("ghs_secret_value"));
}

#[test]
fn src_021_and_src_010_the_declaration_polls_slowly_and_says_why_it_may_write() {
    assert!(DECLARATION.poll_interval_is_allowed());
    assert_eq!(DECLARATION.key, "github");
    assert!(DECLARATION.write_reason.unwrap().contains("PRJ-017"));
}
