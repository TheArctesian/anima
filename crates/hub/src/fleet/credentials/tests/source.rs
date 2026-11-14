//! A source secret's binding to its account (SRC-008, SEC-003).

use super::{other_vault, secret, test_key, vault_from};
use crate::fleet::credentials::{
    Error, Sealed,
    source::{context, open, seal},
    store::StoredSourceSecret,
};
use crate::ids::SourceAccountId;

const ACCOUNT: &str = "0190A000-0000-7000-8020-00000000000A";
const OTHER: &str = "0190a000-0000-7000-8020-00000000000b";

fn account(text: &str) -> SourceAccountId {
    text.parse().unwrap()
}

fn row(sealed: &Sealed) -> StoredSourceSecret {
    StoredSourceSecret {
        nonce: sealed.nonce().to_vec(),
        ciphertext: sealed.ciphertext().to_vec(),
        key_id: sealed.key_id().to_owned(),
    }
}

#[test]
fn src_008_context_names_the_account_in_lowercase() {
    assert_eq!(
        context(account(ACCOUNT)),
        b"credentials.source_secrets:0190a000-0000-7000-8020-00000000000a"
    );
}

#[test]
fn src_008_secret_opens_for_its_account() {
    let vault = vault_from(test_key());
    let sealed = seal(&vault, account(ACCOUNT), &secret("app key")).unwrap();
    let opened = open(&vault, row(&sealed), account(ACCOUNT)).unwrap();
    assert_eq!(opened.expose(), b"app key");
}

#[test]
fn src_008_secret_does_not_open_for_another_account() {
    let vault = vault_from(test_key());
    let sealed = seal(&vault, account(ACCOUNT), &secret("app key")).unwrap();
    assert!(matches!(
        open(&vault, row(&sealed), account(OTHER)),
        Err(Error::Decrypt)
    ));
}

#[test]
fn sec_003_secret_does_not_open_with_another_key() {
    let sealed = seal(
        &vault_from(test_key()),
        account(ACCOUNT),
        &secret("app key"),
    )
    .unwrap();
    assert!(matches!(
        open(&other_vault(), row(&sealed), account(ACCOUNT)),
        Err(Error::WrongKey)
    ));
}

#[test]
fn sec_003_short_nonce_is_malformed() {
    let vault = vault_from(test_key());
    let sealed = seal(&vault, account(ACCOUNT), &secret("app key")).unwrap();
    let mut stored = row(&sealed);
    stored.nonce.pop();
    assert!(matches!(
        open(&vault, stored, account(ACCOUNT)),
        Err(Error::Malformed)
    ));
}
