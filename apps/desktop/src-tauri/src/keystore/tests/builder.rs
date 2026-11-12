//! The builder token entries (BLD-023, BLD-025).

use tauri::async_runtime::block_on;

use super::{SECRET, use_memory_store};
use crate::keystore::{
    KeystoreError, access_token_write, builder_clear as builder_token_clear,
    builder_read as builder_token_read, builder_write as builder_token_write, from_main,
    token::{BUILDER_SERVICE, TokenKind, read_token, write_token},
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
fn c_5_only_the_main_web_view_may_use_the_builder_token() {
    assert_eq!(from_main("main"), Ok(()));
    for other in ["web-tab-1", "pop-tab-1", "Main", "main ", ""] {
        assert_eq!(from_main(other), Err(KeystoreError::Caller), "{other:?}");
    }
}
