//! The builder token and the updater key entries (BLD-023, BLD-025, BLD-045).

use tauri::async_runtime::block_on;

use super::{SECRET, use_memory_store};
use crate::keystore::{
    KeystoreError, access_token_write, builder_token_clear, builder_token_read,
    builder_token_write, read_signing_key,
    token::{BUILDER_SERVICE, SERVICE, TokenKind, read_token, write_token},
    write_signing_key,
};

#[test]
fn bld_023_the_builder_token_is_kept_apart_from_the_access_token() {
    let store = use_memory_store();
    let origin = "https://builder.anima.test".to_owned();
    block_on(access_token_write(
        origin.clone(),
        "operator-token".to_owned(),
    ))
    .unwrap();
    block_on(builder_token_write(
        origin.clone(),
        "builder-token".to_owned(),
    ))
    .unwrap();
    let key = (BUILDER_SERVICE.to_owned(), origin.clone());
    assert_eq!(
        store.lock().unwrap().get(&key).map(String::as_str),
        Some("builder-token")
    );
    assert_eq!(
        block_on(builder_token_read(origin.clone()))
            .unwrap()
            .as_deref(),
        Some("builder-token")
    );
    assert_eq!(
        read_token(TokenKind::Access, &origin).unwrap().as_deref(),
        Some("operator-token")
    );
    assert_eq!(
        read_token(TokenKind::Builder, "https://other.anima.test"),
        Ok(None)
    );
}

#[test]
fn bld_025_clearing_the_builder_token_keeps_the_access_token() {
    use_memory_store();
    let origin = "https://builder-off.anima.test".to_owned();
    write_token(TokenKind::Access, &origin, "operator-token").unwrap();
    write_token(TokenKind::Builder, &origin, "builder-token").unwrap();
    block_on(builder_token_clear(origin.clone())).unwrap();
    assert_eq!(block_on(builder_token_read(origin.clone())).unwrap(), None);
    assert_eq!(
        read_token(TokenKind::Access, &origin).unwrap().as_deref(),
        Some("operator-token")
    );
}

#[test]
fn bld_023_builder_token_commands_refuse_an_origin_that_is_not_canonical() {
    use_memory_store();
    let origin = "http://hub.anima.test".to_owned();
    assert_eq!(
        block_on(builder_token_write(origin.clone(), SECRET.to_owned())),
        Err(KeystoreError::BadOrigin)
    );
    assert_eq!(
        block_on(builder_token_read(origin)),
        Err(KeystoreError::BadOrigin)
    );
}

#[test]
fn bld_045_the_updater_key_round_trips_and_an_empty_key_is_refused() {
    let store = use_memory_store();
    assert_eq!(write_signing_key(""), Err(KeystoreError::EmptyToken));
    write_signing_key("stored-key-text").unwrap();
    assert_eq!(
        read_signing_key().unwrap().as_deref().map(String::as_str),
        Some("stored-key-text")
    );
    let key = (SERVICE.to_owned(), "updater-key".to_owned());
    assert!(store.lock().unwrap().contains_key(&key));
}
