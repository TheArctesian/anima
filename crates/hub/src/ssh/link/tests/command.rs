//! The command line (CHK-012, D-3): static words and a MAC rendered by the hub, joined by
//! single spaces.

use proptest::prelude::*;

use crate::{devices::MacAddress, ssh::link::FixedCommand};

fn mac(text: &str) -> MacAddress {
    text.parse().unwrap()
}

#[test]
fn chk_012_a_program_alone_is_its_own_line() {
    assert_eq!(FixedCommand::new("uptime").line(), "uptime");
}

#[test]
fn chk_012_static_words_follow_the_program_in_order_split_by_one_space() {
    let command = FixedCommand::new("systemctl")
        .arg("list-units")
        .arg("--type=service")
        .arg("--no-pager");
    assert_eq!(
        command.line(),
        "systemctl list-units --type=service --no-pager"
    );
}

#[test]
fn d_3_a_mac_renders_lower_case_with_colons_whatever_form_it_was_typed_in() {
    let typed = mac("0A-1B-2C-3D-4E-5F");
    assert_eq!(
        FixedCommand::new("wakeonlan").mac(typed).line(),
        "wakeonlan 0a:1b:2c:3d:4e:5f"
    );
}

#[test]
fn d_3_static_words_and_a_mac_keep_their_order() {
    let command = FixedCommand::new("etherwake")
        .arg("-i")
        .arg("eth0")
        .mac(mac("aa:bb:cc:dd:ee:f0"));
    assert_eq!(command.line(), "etherwake -i eth0 aa:bb:cc:dd:ee:f0");
}

proptest! {
    /// D-3: the MAC word is the six parsed bytes as two hex digits each, so it is always 17
    /// characters of `0-9a-f:` and nothing a shell reads specially.
    #[test]
    fn d_3_the_mac_word_is_rendered_from_its_bytes(bytes in any::<[u8; 6]>()) {
        let mut bytes = bytes;
        bytes[0] &= 0xfe; // unicast
        prop_assume!(bytes != [0; 6]);
        let typed = bytes.map(|b| format!("{b:02X}")).join("-");
        let line = FixedCommand::new("wakeonlan").mac(mac(&typed)).line();
        let word = line.strip_prefix("wakeonlan ").unwrap();
        let expected = bytes.map(|b| format!("{b:02x}")).join(":");
        prop_assert_eq!(word, expected.as_str());
        prop_assert!(word.bytes().all(|b| b.is_ascii_hexdigit() || b == b':'));
    }
}
