//! The command line (CHK-012, D-3, HOST-001): static words, and a MAC, hex digits and numbers
//! rendered by the hub, joined by single spaces.

use proptest::prelude::*;

use crate::fleet::{
    devices::MacAddress,
    ssh::link::{FixedCommand, HexDigits},
};

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

#[test]
fn host_001_hex_digits_and_a_number_render_in_order_after_static_words() {
    let id = HexDigits::parse("0123456789abcdef").unwrap();
    let command = FixedCommand::new("docker")
        .arg("logs")
        .arg("--tail")
        .number(200)
        .hex(&id)
        .arg("2>&1");
    assert_eq!(
        command.line(),
        "docker logs --tail 200 0123456789abcdef 2>&1"
    );
}

#[test]
fn host_001_hex_digits_take_only_one_to_sixty_four_lower_case_hex_digits() {
    for refused in [
        "",
        "ABCDEF",
        "abc def",
        "abc;rm",
        "$(id)",
        "0x12",
        "abcdefg",
        "é",
        &"a".repeat(65),
    ] {
        assert_eq!(HexDigits::parse(refused), None, "{refused:?}");
    }
    for taken in ["0", "f", &"a".repeat(64)] {
        assert!(HexDigits::parse(taken).is_some(), "{taken:?}");
    }
}

proptest! {
    /// HOST-001: whatever text is offered, a word made of hex digits is that text exactly, and
    /// it is only `0-9a-f`.
    #[test]
    fn host_001_hex_digits_render_as_the_text_they_were_read_from(text in "[0-9a-f]{0,66}|.{0,70}") {
        let parsed = HexDigits::parse(&text);
        let all_hex = text.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'));
        prop_assert_eq!(parsed.is_some(), all_hex && (1..=64).contains(&text.len()));
        if let Some(digits) = parsed {
            let line = FixedCommand::new("docker").hex(&digits).line();
            prop_assert_eq!(line, format!("docker {text}"));
        }
    }

    /// HOST-001: a number renders as its decimal digits only.
    #[test]
    fn host_001_a_number_renders_in_decimal(number in any::<u32>()) {
        let line = FixedCommand::new("docker").number(number).line();
        prop_assert_eq!(line, format!("docker {number}"));
    }

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
