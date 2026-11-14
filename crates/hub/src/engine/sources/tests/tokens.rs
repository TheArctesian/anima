//! PRJ-018: a token minted while its account was being removed is not kept. The removal's
//! `forget` and a mint's `keep` meet under the one lock of [`Tokens`].

use chrono::{TimeZone as _, Utc};
use source_exchange::Minted;
use uuid::Uuid;

use crate::{engine::sources::Tokens, ids::SourceAccountId};

fn minted() -> Minted {
    Minted {
        token: format!("t-{}", Uuid::now_v7().simple()),
        expires_at: Utc.with_ymd_and_hms(2099, 1, 1, 0, 0, 0).unwrap(),
    }
}

fn now() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 13, 9, 0, 0).unwrap()
}

#[test]
fn prj_018_a_keep_after_the_accounts_forget_stores_no_token() {
    let tokens = Tokens::default();
    let account = SourceAccountId::from(Uuid::now_v7());
    let ticket = tokens.ticket();
    // The mint is in flight; the account is removed and its token forgotten.
    tokens.forget(account);

    drop(tokens.keep(account, minted(), ticket));

    assert!(tokens.fresh(account, now()).is_none());
}

#[test]
fn prj_018_a_keep_with_a_ticket_taken_after_the_forget_stores_the_token() {
    let tokens = Tokens::default();
    let account = SourceAccountId::from(Uuid::now_v7());
    tokens.forget(account);
    let ticket = tokens.ticket();

    drop(tokens.keep(account, minted(), ticket));

    assert!(tokens.fresh(account, now()).is_some());
}
