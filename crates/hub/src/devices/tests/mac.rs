//! The hardware address of a device (RA-031, D-6).

use crate::devices::mac::{MacAddress, MacAddressError};

fn mac(text: &str) -> Result<MacAddress, MacAddressError> {
    text.parse()
}

#[test]
fn ra_031_mac_accepts_colons_or_hyphens_in_any_case_and_renders_lower_case() {
    for text in [
        "AA:bb:0C:dd:EE:f2",
        "aa-BB-0c-DD-ee-F2",
        "aa:bb:0c:dd:ee:f2",
    ] {
        assert_eq!(
            mac(text).unwrap().to_string(),
            "aa:bb:0c:dd:ee:f2",
            "{text}"
        );
    }
    assert_eq!(
        mac("aa:bb:0c:dd:ee:f2").unwrap().octets(),
        [0xaa, 0xbb, 0x0c, 0xdd, 0xee, 0xf2]
    );
}

#[test]
fn ra_031_mac_refuses_text_that_is_not_six_hex_pairs() {
    for text in [
        "",
        "aabbccddeeff",
        "aa:bb:cc:dd:ee",
        "aa:bb:cc:dd:ee:f2:00",
        "aa:bb-cc:dd:ee:f2",
        "aa:bb:cc:dd:ee:fg",
        "a:bb:cc:dd:ee:f2",
        "aaa:bb:cc:dd:ee:f2",
        "aa:bb:cc:dd:ee:+2",
        " aa:bb:cc:dd:ee:f2",
        "aa:bb:cc:dd:ee:f2:",
    ] {
        assert_eq!(mac(text), Err(MacAddressError::Malformed), "{text:?}");
    }
}

#[test]
fn ra_031_mac_refuses_the_all_zero_and_group_addresses() {
    assert_eq!(mac("00:00:00:00:00:00"), Err(MacAddressError::AllZero));
    assert_eq!(mac("ff:ff:ff:ff:ff:ff"), Err(MacAddressError::Group));
    assert_eq!(mac("01:00:5e:00:00:fb"), Err(MacAddressError::Group));
    assert!(mac("02:00:5e:00:00:fb").is_ok());
}

#[test]
fn log_007_sec_009_mac_debug_holds_no_octet() {
    let address = mac("aa:bb:0c:dd:ee:f2").unwrap();
    for shown in [format!("{address:?}"), format!("{address:#?}")] {
        assert_eq!(shown, "MacAddress(..)");
        for octet in ["170", "187", "12", "221", "238", "242", "aa:bb", "0c:dd"] {
            assert!(!shown.contains(octet), "{shown} holds {octet}");
        }
    }
}
