use crate::fleet::credentials::{PasswordKind, password::context};
use crate::ids::DeviceId;

const DEVICE: &str = "0190A000-0000-7000-8002-00000000000A";

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
