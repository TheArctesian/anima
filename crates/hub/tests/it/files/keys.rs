//! A data key is never the credential key (VLT-003, SEC-004).

use anima_hub::engine::sealing_keys::{SealingKeyError, SealingKeys};
use base64::{Engine as _, engine::general_purpose::STANDARD};

use crate::hub::test_vault;

#[test]
fn vlt_003_start_refuses_a_data_key_file_that_holds_the_credential_key() {
    // `test_vault` is the key of 32 bytes 7; a data key file with the same bytes is refused.
    let vault = test_vault();
    let folder = tempfile::tempdir().unwrap();
    std::fs::write(
        folder.path().join("engine.key"),
        STANDARD.encode([7_u8; 32]),
    )
    .unwrap();

    let refused = SealingKeys::open(Some(folder.path()), vault.key_id().as_bytes());

    assert!(
        matches!(&refused, Err(SealingKeyError::CredentialKey { name }) if name == "engine"),
        "{refused:?}"
    );
    let shown = format!("{:?}", refused.unwrap_err());
    assert!(!shown.contains(&STANDARD.encode([7_u8; 32])), "{shown}");
}

#[test]
fn vlt_003_a_data_key_made_at_first_use_is_not_the_credential_key() {
    let vault = test_vault();
    let folder = tempfile::tempdir().unwrap();
    let keys = SealingKeys::open(Some(folder.path()), vault.key_id().as_bytes()).unwrap();

    let key = keys.key("library").unwrap();

    assert_ne!(key.id(), vault.key_id());
    assert!(SealingKeys::open(Some(folder.path()), vault.key_id().as_bytes()).is_ok());
}
