use crate::credentials::{PasswordKind, password::context, ssh::SshKind};
use crate::ids::DeviceId;

const DEVICE: &str = "0190A000-0000-7000-8002-00000000000A";
const OTHER: &str = "0190a000-0000-7000-8002-00000000000b";

fn device(text: &str) -> DeviceId {
    text.parse().unwrap()
}

#[test]
fn sec_003_context_names_the_device_and_password_kind() {
    assert_eq!(
        context(device(DEVICE), PasswordKind::Rdp),
        b"credentials.secrets:0190a000-0000-7000-8002-00000000000a:rdp_password"
    );
    assert_eq!(
        context(device(DEVICE), PasswordKind::Vnc),
        b"credentials.secrets:0190a000-0000-7000-8002-00000000000a:vnc_password"
    );
}

#[test]
fn sec_003_contexts_differ_per_kind_and_device() {
    let contexts = [
        context(device(DEVICE), PasswordKind::Rdp),
        context(device(DEVICE), PasswordKind::Vnc),
        context(device(OTHER), PasswordKind::Rdp),
        context(device(OTHER), PasswordKind::Vnc),
    ];
    for (index, first) in contexts.iter().enumerate() {
        for second in &contexts[index + 1..] {
            assert_ne!(first, second);
        }
    }
}

#[test]
fn password_kinds_have_their_own_sql_text() {
    assert_eq!(PasswordKind::Rdp.as_sql(), "rdp_password");
    assert_eq!(PasswordKind::Vnc.as_sql(), "vnc_password");
    for kind in [PasswordKind::Rdp, PasswordKind::Vnc] {
        assert!(
            !SshKind::SQL_KINDS.contains(&kind.as_sql()),
            "{kind:?} must not be an SSH kind"
        );
    }
}

#[test]
fn ssh_kind_list_round_trips_through_its_sql_text() {
    for text in SshKind::SQL_KINDS {
        assert_eq!(SshKind::from_sql(text).unwrap().as_sql(), text);
    }
    assert_eq!(SshKind::SQL_KINDS.len(), 3);
}
