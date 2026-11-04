//! Tests run against an in-memory credential store installed as keyring's default builder, never
//! the real Keychain. keyring's own mock keeps nothing between two `Entry::new` calls, so it
//! cannot show that two origins are independent; this store is keyed by (service, account).

use std::{
    any::Any,
    collections::HashMap,
    sync::{Arc, Mutex, Once},
};

use keyring::{
    Error as BackendError,
    credential::{Credential, CredentialApi, CredentialBuilderApi},
};

use super::{
    KeystoreError,
    token::{clear_token, read_token, write_token},
};

type Store = Arc<Mutex<HashMap<(String, String), String>>>;

struct MemoryCredential {
    key: (String, String),
    store: Store,
}

impl CredentialApi for MemoryCredential {
    fn set_secret(&self, secret: &[u8]) -> keyring::Result<()> {
        let text = String::from_utf8_lossy(secret).into_owned();
        self.store.lock().unwrap().insert(self.key.clone(), text);
        Ok(())
    }

    fn get_secret(&self) -> keyring::Result<Vec<u8>> {
        let store = self.store.lock().unwrap();
        store
            .get(&self.key)
            .map(|text| text.clone().into_bytes())
            .ok_or(BackendError::NoEntry)
    }

    fn delete_credential(&self) -> keyring::Result<()> {
        let removed = self.store.lock().unwrap().remove(&self.key);
        removed.map(|_| ()).ok_or(BackendError::NoEntry)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

struct MemoryStore(Store);

impl CredentialBuilderApi for MemoryStore {
    fn build(
        &self,
        _: Option<&str>,
        service: &str,
        user: &str,
    ) -> keyring::Result<Box<Credential>> {
        Ok(Box::new(MemoryCredential {
            key: (service.to_owned(), user.to_owned()),
            store: Arc::clone(&self.0),
        }))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Installs the in-memory store once for the test process. Each test uses its own origins.
fn use_memory_store() {
    static INSTALL: Once = Once::new();
    INSTALL.call_once(|| {
        let store = MemoryStore(Arc::default());
        keyring::set_default_credential_builder(Box::new(store));
    });
}

const SECRET: &str = "tok-s3cret-value";
const BACKEND_TEXT: &str = "backend echoed hub-access-token and tok-s3cret-value";

#[test]
fn cli_003_token_round_trips_through_write_read_clear() {
    use_memory_store();
    let origin = "https://round-trip.anima.test";
    write_token(origin, SECRET).unwrap();
    assert_eq!(read_token(origin).unwrap().as_deref(), Some(SECRET));
    clear_token(origin).unwrap();
    assert_eq!(read_token(origin).unwrap(), None);
}

#[test]
fn cli_003_missing_entry_reads_as_none() {
    use_memory_store();
    assert_eq!(read_token("https://missing.anima.test").unwrap(), None);
}

#[test]
fn cli_003_clearing_a_missing_entry_is_ok() {
    use_memory_store();
    clear_token("https://never-written.anima.test").unwrap();
}

#[test]
fn cli_003_empty_token_is_refused_and_nothing_is_stored() {
    use_memory_store();
    let origin = "https://empty.anima.test";
    assert_eq!(write_token(origin, ""), Err(KeystoreError::EmptyToken));
    assert_eq!(read_token(origin).unwrap(), None);
}

#[test]
fn sec_002_tokens_for_two_origins_are_independent() {
    use_memory_store();
    let (a, b) = ("https://a.anima.test", "https://b.anima.test");
    write_token(a, "token-for-a").unwrap();
    assert_eq!(read_token(b).unwrap(), None);
    write_token(b, "token-for-b").unwrap();
    clear_token(a).unwrap();
    assert_eq!(read_token(a).unwrap(), None);
    assert_eq!(read_token(b).unwrap().as_deref(), Some("token-for-b"));
}

#[test]
fn sec_002_an_origin_that_is_not_https_is_refused_by_every_command() {
    use_memory_store();
    for origin in [
        "",
        "https://",
        "http://hub.anima.test",
        "hub.anima.test",
        "ftp://x",
    ] {
        assert_eq!(
            read_token(origin),
            Err(KeystoreError::BadOrigin),
            "{origin}"
        );
        assert_eq!(
            write_token(origin, SECRET),
            Err(KeystoreError::BadOrigin),
            "{origin}"
        );
        assert_eq!(
            clear_token(origin),
            Err(KeystoreError::BadOrigin),
            "{origin}"
        );
    }
    assert_eq!(KeystoreError::BadOrigin.code(), "origin");
}

#[test]
fn log_007_each_backend_error_maps_to_its_code() {
    let cases = [
        (
            BackendError::PlatformFailure(BACKEND_TEXT.into()),
            "unavailable",
        ),
        (BackendError::NoStorageAccess(BACKEND_TEXT.into()), "denied"),
        (
            BackendError::Invalid("attr".into(), BACKEND_TEXT.into()),
            "failed",
        ),
        (
            BackendError::BadEncoding(SECRET.as_bytes().to_vec()),
            "failed",
        ),
    ];
    for (backend, code) in cases {
        assert_eq!(KeystoreError::from(backend).code(), code);
    }
    assert_eq!(KeystoreError::EmptyToken.code(), "empty");
}

#[test]
fn log_007_serialised_and_displayed_errors_hold_no_token_or_backend_text() {
    let errors = [
        KeystoreError::from(BackendError::PlatformFailure(BACKEND_TEXT.into())),
        KeystoreError::from(BackendError::NoStorageAccess(BACKEND_TEXT.into())),
        KeystoreError::from(BackendError::BadEncoding(SECRET.as_bytes().to_vec())),
        KeystoreError::EmptyToken,
        KeystoreError::BadOrigin,
    ];
    for error in errors {
        let json = serde_json::to_string(&error).unwrap();
        let shown = format!("{error} {error:?}");
        for text in [json.as_str(), shown.as_str()] {
            assert!(!text.contains(SECRET), "token leaked: {text}");
            assert!(!text.contains("echoed"), "backend text leaked: {text}");
            assert!(
                !text.contains("hub-access-token"),
                "attribute leaked: {text}"
            );
        }
    }
}

#[test]
fn log_007_error_serialises_as_code_and_fixed_message() {
    let json = serde_json::to_value(KeystoreError::AccessDenied).unwrap();
    assert_eq!(
        json,
        serde_json::json!({ "code": "denied", "message": "the OS key store refused access" })
    );
}
