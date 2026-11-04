use std::fmt;

use static_assertions::assert_not_impl_any;

use super::secret;
use crate::credentials::{
    Error, Secret, SshSecret,
    ssh::{SshKind, context},
};
use crate::ids::DeviceId;

fn from_kinds(kinds: &[SshKind]) -> Result<SshSecret, Error> {
    SshSecret::from_parts(kinds.iter().map(|&kind| (kind, secret(kind.as_sql()))))
}

fn exposed(parts: &[(SshKind, Secret)]) -> Vec<(SshKind, &[u8])> {
    parts
        .iter()
        .map(|(kind, part)| (*kind, part.expose()))
        .collect()
}

#[test]
fn log_007_ssh_secret_has_no_format_traits() {
    assert_not_impl_any!(SshSecret: fmt::Debug, fmt::Display, serde::Serialize, Clone, PartialEq);
}

#[test]
fn sec_003_context_names_the_device_and_kind() {
    let device: DeviceId = "0190A000-0000-7000-8002-00000000000A".parse().unwrap();
    assert_eq!(
        context(device, SshKind::Passphrase),
        b"credentials.secrets:0190a000-0000-7000-8002-00000000000a:ssh_passphrase"
    );
}

#[test]
fn secret_kind_round_trips_through_its_sql_text() {
    let kinds = [SshKind::Password, SshKind::PrivateKey, SshKind::Passphrase];
    for kind in kinds {
        assert_eq!(SshKind::from_sql(kind.as_sql()).unwrap(), kind);
    }
    assert!(matches!(SshKind::from_sql("totp"), Err(Error::Malformed)));
}

#[test]
fn password_maps_to_one_password_part() {
    let parts = SshSecret::Password(secret("hunter2")).into_parts();
    assert_eq!(
        exposed(&parts),
        [(SshKind::Password, b"hunter2".as_slice())]
    );
}

#[test]
fn private_key_maps_to_key_and_optional_passphrase_parts() {
    let bare = SshSecret::PrivateKey {
        key: secret("KEY"),
        passphrase: None,
    }
    .into_parts();
    assert_eq!(exposed(&bare), [(SshKind::PrivateKey, b"KEY".as_slice())]);

    let locked = SshSecret::PrivateKey {
        key: secret("KEY"),
        passphrase: Some(secret("PASS")),
    }
    .into_parts();
    assert_eq!(
        exposed(&locked),
        [
            (SshKind::PrivateKey, b"KEY".as_slice()),
            (SshKind::Passphrase, b"PASS".as_slice()),
        ]
    );
}

#[test]
fn possible_row_sets_rebuild_the_secret() {
    assert!(matches!(
        from_kinds(&[SshKind::Password]),
        Ok(SshSecret::Password(p)) if p.expose() == b"ssh_password"
    ));
    assert!(matches!(
        from_kinds(&[SshKind::PrivateKey]),
        Ok(SshSecret::PrivateKey { key, passphrase: None }) if key.expose() == b"ssh_private_key"
    ));
    assert!(matches!(
        from_kinds(&[SshKind::Passphrase, SshKind::PrivateKey]),
        Ok(SshSecret::PrivateKey { key, passphrase: Some(p) })
            if key.expose() == b"ssh_private_key" && p.expose() == b"ssh_passphrase"
    ));
}

#[test]
fn impossible_row_sets_are_malformed() {
    use SshKind::{Passphrase, Password, PrivateKey};
    let impossible: [&[SshKind]; 7] = [
        &[],
        &[Passphrase],
        &[Password, PrivateKey],
        &[Password, Passphrase],
        &[Password, PrivateKey, Passphrase],
        &[Password, Password],
        &[PrivateKey, PrivateKey],
    ];
    for kinds in impossible {
        assert!(
            matches!(from_kinds(kinds), Err(Error::Malformed)),
            "{kinds:?}"
        );
    }
}

#[test]
fn log_007_database_error_message_is_generic() {
    let message = Error::Database(sqlx::Error::Protocol("SELECT secret".into())).to_string();
    assert_eq!(message, "the credential store is unavailable");
}
