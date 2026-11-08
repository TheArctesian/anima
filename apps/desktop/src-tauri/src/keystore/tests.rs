//! Tests run against an in-memory credential store installed as keyring's default builder, never
//! the real Keychain. keyring's own mock keeps nothing between two `Entry::new` calls, so it
//! cannot show that two origins are independent; this store is keyed by (service, account).

use std::{
    any::Any,
    collections::HashMap,
    sync::{Arc, Mutex, OnceLock},
};

use tauri::async_runtime::block_on;

mod builder;

use keyring::{
    Error as BackendError,
    credential::{Credential, CredentialApi, CredentialBuilderApi},
};

use super::{
    KeystoreError, access_token_clear, access_token_read, access_token_write, blocking,
    token::{RELEASE_SERVICE, SERVICE, TokenKind, clear_token, read_token, write_token},
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

/// Installs the in-memory store once for the test process and returns it, so a test can see
/// the exact (service, account) keys. Each test uses its own origins.
pub(crate) fn use_memory_store() -> &'static Store {
    static STORE: OnceLock<Store> = OnceLock::new();
    STORE.get_or_init(|| {
        let store = Store::default();
        keyring::set_default_credential_builder(Box::new(MemoryStore(Arc::clone(&store))));
        store
    })
}

const SECRET: &str = "tok-s3cret-value";
const BACKEND_TEXT: &str = "backend echoed hub-access-token and tok-s3cret-value";

#[test]
fn cli_003_token_round_trips_through_write_read_clear() {
    use_memory_store();
    let origin = "https://round-trip.anima.test";
    write_token(TokenKind::Access, origin, SECRET).unwrap();
    assert_eq!(
        read_token(TokenKind::Access, origin).unwrap().as_deref(),
        Some(SECRET)
    );
    clear_token(TokenKind::Access, origin).unwrap();
    assert_eq!(read_token(TokenKind::Access, origin).unwrap(), None);
}

#[test]
fn cli_003_missing_entry_reads_as_none() {
    use_memory_store();
    assert_eq!(
        read_token(TokenKind::Access, "https://missing.anima.test").unwrap(),
        None
    );
}

#[test]
fn cli_003_clearing_a_missing_entry_is_ok() {
    use_memory_store();
    clear_token(TokenKind::Access, "https://never-written.anima.test").unwrap();
}

#[test]
fn cli_003_empty_token_is_refused_and_nothing_is_stored() {
    use_memory_store();
    let origin = "https://empty.anima.test";
    assert_eq!(
        write_token(TokenKind::Access, origin, ""),
        Err(KeystoreError::EmptyToken)
    );
    assert_eq!(read_token(TokenKind::Access, origin).unwrap(), None);
}

#[test]
fn cli_003_tokens_for_two_origins_are_independent() {
    use_memory_store();
    let (a, b) = ("https://a.anima.test", "https://b.anima.test");
    write_token(TokenKind::Access, a, "token-for-a").unwrap();
    assert_eq!(read_token(TokenKind::Access, b).unwrap(), None);
    write_token(TokenKind::Access, b, "token-for-b").unwrap();
    clear_token(TokenKind::Access, a).unwrap();
    assert_eq!(read_token(TokenKind::Access, a).unwrap(), None);
    assert_eq!(
        read_token(TokenKind::Access, b).unwrap().as_deref(),
        Some("token-for-b")
    );
}

#[test]
fn cli_003_an_origin_that_is_not_canonical_https_is_refused_by_every_command() {
    use_memory_store();
    for origin in [
        "",
        "https://",
        "http://hub.anima.test",
        "hub.anima.test",
        "ftp://x",
        "https://hub/",
        "https://HUB",
        "https://hub ",
        "https://hub/path?q",
        "https:// ",
        "https://\n",
        " https://hub",
        "https://hub?x",
    ] {
        assert_eq!(
            read_token(TokenKind::Access, origin),
            Err(KeystoreError::BadOrigin),
            "{origin}"
        );
        assert_eq!(
            write_token(TokenKind::Access, origin, SECRET),
            Err(KeystoreError::BadOrigin),
            "{origin}"
        );
        assert_eq!(
            clear_token(TokenKind::Access, origin),
            Err(KeystoreError::BadOrigin),
            "{origin}"
        );
    }
    assert_eq!(KeystoreError::BadOrigin.code(), "origin");
}

#[test]
fn cli_003_canonical_origins_with_port_or_ip_are_accepted() {
    use_memory_store();
    for origin in [
        "https://hub.anima.ing",
        "https://hub.anima.ing:8443",
        "https://10.0.0.5",
        "https://[::1]:8443",
        "https://my_hub.lan",
    ] {
        assert_eq!(read_token(TokenKind::Access, origin), Ok(None), "{origin}");
    }
}

#[test]
fn cli_003_commands_round_trip_and_store_under_service_and_origin() {
    let store = use_memory_store();
    let origin = "https://commands.anima.test".to_owned();
    block_on(access_token_write(origin.clone(), SECRET.to_owned())).unwrap();
    let key = (SERVICE.to_owned(), origin.clone());
    assert_eq!(
        store.lock().unwrap().get(&key).map(String::as_str),
        Some(SECRET)
    );
    assert_eq!(
        block_on(access_token_read(origin.clone()))
            .unwrap()
            .as_deref(),
        Some(SECRET)
    );
    block_on(access_token_clear(origin.clone())).unwrap();
    assert!(!store.lock().unwrap().contains_key(&key));
    assert_eq!(block_on(access_token_read(origin)).unwrap(), None);
}

#[test]
fn cli_003_a_blocking_call_that_panics_is_a_failed_error_with_no_detail() {
    let result: Result<(), KeystoreError> = block_on(blocking(|| panic!("secret detail {SECRET}")));
    assert_eq!(result, Err(KeystoreError::Failed));
}

#[test]
fn cli_003_release_builds_use_the_shipped_service_name() {
    assert_eq!(RELEASE_SERVICE, "ing.anima.desktop");
    if cfg!(debug_assertions) {
        assert_eq!(SERVICE, "ing.anima.desktop.dev");
    } else {
        assert_eq!(SERVICE, RELEASE_SERVICE);
    }
}

#[cfg(target_os = "macos")]
#[test]
fn cli_003_a_cancelled_or_failed_keychain_prompt_is_access_denied() {
    for code in [-128, -25293] {
        let backend =
            keyring::macos::decode_error(security_framework::base::Error::from_code(code));
        assert_eq!(
            KeystoreError::from(backend),
            KeystoreError::AccessDenied,
            "{code}"
        );
    }
    let other = keyring::macos::decode_error(security_framework::base::Error::from_code(-25299));
    assert_eq!(KeystoreError::from(other), KeystoreError::Unavailable);
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
