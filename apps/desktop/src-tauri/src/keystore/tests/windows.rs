//! Uses an isolated trial credential in the real Windows store, without replacing the test mock.
use keyring::{credential::CredentialApi, windows::WinCredential};

#[test]
fn windows_credential_survives_reopen_and_is_removed() {
    let account = format!("trial-{}", std::process::id());
    let entry = WinCredential::new_with_target(None, "ing.anima.windows-test", &account).unwrap();
    entry.set_password("anima-test-value").unwrap();
    let reopened =
        WinCredential::new_with_target(None, "ing.anima.windows-test", &account).unwrap();
    let read = reopened.get_password();
    reopened.delete_credential().unwrap();
    assert_eq!(read.unwrap(), "anima-test-value");
    assert!(matches!(
        reopened.get_password(),
        Err(keyring::Error::NoEntry)
    ));
}
