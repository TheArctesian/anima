use std::fmt;

use base64::{Engine as _, engine::general_purpose::STANDARD};
use static_assertions::assert_not_impl_any;

use super::{test_key, vault_from};
use crate::credentials::Secret;

#[test]
fn log_007_secret_has_no_format_traits() {
    assert_not_impl_any!(Secret: fmt::Debug, fmt::Display, serde::Serialize, Clone, PartialEq);
}

#[test]
fn log_007_vault_debug_shows_only_the_key_id() {
    let key = test_key();
    let printed = format!("{:?}", vault_from(key));
    assert_eq!(printed, r#"Vault { key_id: "2407973bf9f2a329", .. }"#);
    assert!(!printed.contains(&STANDARD.encode(key)));
}
