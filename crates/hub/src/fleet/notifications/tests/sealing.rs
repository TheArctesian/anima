//! Sealed channel secrets open only for their channel, kind, target and header names
//! (SEC-003, SEC-005, D-17). The key and every secret are synthetic.

use std::io::Write as _;

use base64::{Engine as _, engine::general_purpose::STANDARD};
use uuid::Uuid;

use crate::{
    fleet::{
        credentials::{Secret, Vault},
        notifications::{
            Error,
            channels::{
                Kept, NtfySecrets, NtfySettings, Secrets, Settings, WebhookMethod, WebhookSecrets,
                WebhookSettings,
            },
            sealing,
        },
    },
    ids::ChannelId,
};

fn vault(byte: u8) -> Vault {
    let mut key_file = tempfile::NamedTempFile::new().unwrap();
    writeln!(key_file, "{}", STANDARD.encode([byte; 32])).unwrap();
    Vault::from_key_file(key_file.path()).unwrap()
}

fn secret(text: &str) -> Secret {
    Secret::from(text.to_owned())
}

fn channel() -> ChannelId {
    ChannelId::from(Uuid::now_v7())
}

fn webhook(origin: &str, names: &[&str]) -> Settings {
    Settings::Webhook(WebhookSettings {
        origin: origin.to_owned(),
        method: WebhookMethod::Post,
        header_names: names.iter().map(|name| (*name).to_owned()).collect(),
    })
}

fn webhook_secrets() -> Secrets {
    Secrets::Webhook(WebhookSecrets {
        url: secret("https://hooks.example.test/s3cr3t-path"),
        headers: vec![secret("header-s3cr3t")],
    })
}

const ORIGIN: &str = "https://hooks.example.test";

#[test]
fn sec_003_channel_secrets_round_trip_and_are_not_stored_in_clear() {
    let (vault, id, settings) = (vault(7), channel(), webhook(ORIGIN, &["X-Key"]));
    let sealed = sealing::seal(&vault, id, &settings, &webhook_secrets()).unwrap();
    let clear = String::from_utf8_lossy(&sealed);
    assert!(!clear.contains("s3cr3t"));
    let Secrets::Webhook(opened) = sealing::open(&vault, id, &settings, &sealed).unwrap() else {
        panic!("not a webhook");
    };
    assert_eq!(
        opened.url.expose(),
        b"https://hooks.example.test/s3cr3t-path"
    );
    assert_eq!(opened.headers.len(), 1);
    assert_eq!(opened.headers[0].expose(), b"header-s3cr3t");
}

#[test]
fn sec_003_secrets_open_only_for_their_channel_and_key() {
    let (keys, id, settings) = (vault(7), channel(), webhook(ORIGIN, &["X-Key"]));
    let sealed = sealing::seal(&keys, id, &settings, &webhook_secrets()).unwrap();
    let other = sealing::open(&keys, channel(), &settings, &sealed);
    assert!(matches!(other, Err(Error::Credentials(_))));
    let rekeyed = sealing::open(&vault(9), id, &settings, &sealed);
    assert!(matches!(rekeyed, Err(Error::Credentials(_))));
}

#[test]
fn sec_005_secrets_do_not_open_for_another_target_or_header_names() {
    let (vault, id) = (vault(7), channel());
    let settings = webhook(ORIGIN, &["X-Key"]);
    let sealed = sealing::seal(&vault, id, &settings, &webhook_secrets()).unwrap();
    for moved in [
        webhook("https://elsewhere.example.test", &["X-Key"]),
        webhook(ORIGIN, &["Authorization"]),
        webhook(ORIGIN, &[]),
    ] {
        let opened = sealing::open(&vault, id, &moved, &sealed);
        assert!(matches!(opened, Err(Error::SecretsUnbound)));
    }
    let ntfy = Settings::Ntfy(NtfySettings {
        server: ORIGIN.to_owned(),
        priority: 3,
        token_stored: false,
    });
    assert!(matches!(
        sealing::open(&vault, id, &ntfy, &sealed),
        Err(Error::SecretsUnbound)
    ));
}

#[test]
fn sec_005_secrets_must_have_the_shape_the_settings_say() {
    let (vault, id) = (vault(7), channel());
    let tokenless = Settings::Ntfy(NtfySettings {
        server: ORIGIN.to_owned(),
        priority: 3,
        token_stored: false,
    });
    let with_token = Secrets::Ntfy(NtfySecrets {
        topic: secret("alerts"),
        token: Some(secret("tk_1")),
    });
    let sealed = sealing::seal(&vault, id, &tokenless, &with_token).unwrap();
    assert!(matches!(
        sealing::open(&vault, id, &tokenless, &sealed),
        Err(Error::SecretsUnbound)
    ));
}

#[test]
fn sec_003_an_update_finds_stored_secrets_opened_absent_or_unopened() {
    let (vault, id, settings) = (vault(7), channel(), webhook(ORIGIN, &["X-Key"]));
    let sealed = sealing::seal(&vault, id, &settings, &webhook_secrets()).unwrap();
    assert!(matches!(
        sealing::kept(&vault, id, &settings, None),
        Kept::Nothing
    ));
    assert!(matches!(
        sealing::kept(&vault, id, &settings, Some(&sealed)),
        Kept::Opened(_)
    ));
    let garbage = [1_u8, 2, 3];
    assert!(matches!(
        sealing::kept(&vault, id, &settings, Some(&garbage)),
        Kept::Unopened
    ));
    let moved = webhook("https://elsewhere.example.test", &["X-Key"]);
    assert!(matches!(
        sealing::kept(&vault, id, &moved, Some(&sealed)),
        Kept::Unopened
    ));
}
