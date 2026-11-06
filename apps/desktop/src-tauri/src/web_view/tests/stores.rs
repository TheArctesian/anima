//! D-2, WEB-006: one store per device, named by its `DeviceId`.

use crate::web_view::{
    WebViewError,
    storage::{DeviceId, STORE_FOLDER},
};

const DEVICE: &str = "0192f3a4-5b6c-7d8e-9fa0-b1c2d3e4f5a6";

#[test]
fn web_006_the_macos_store_is_the_device_uuid_s_16_bytes() {
    let device = DeviceId::parse(DEVICE).unwrap();
    assert_eq!(
        device.store_identifier(),
        [
            0x01, 0x92, 0xf3, 0xa4, 0x5b, 0x6c, 0x7d, 0x8e, 0x9f, 0xa0, 0xb1, 0xc2, 0xd3, 0xe4,
            0xf5, 0xa6
        ]
    );
}

#[test]
fn web_006_the_linux_store_is_a_folder_named_by_the_lower_case_uuid() {
    let device = DeviceId::parse(&DEVICE.to_ascii_uppercase()).unwrap();
    assert_eq!(device.folder_name(), DEVICE);
    assert_eq!(STORE_FOLDER, "web");
}

#[test]
fn web_006_two_devices_have_two_stores() {
    let other = DeviceId::parse("0192f3a4-5b6c-7d8e-9fa0-b1c2d3e4f5a7").unwrap();
    let device = DeviceId::parse(DEVICE).unwrap();
    assert_ne!(device.store_identifier(), other.store_identifier());
    assert_ne!(device.folder_name(), other.folder_name());
}

#[test]
fn web_006_a_device_id_is_the_hyphenated_uuid_and_nothing_else() {
    for bad in [
        "",
        "../../etc",
        "0192f3a45b6c7d8e9fa0b1c2d3e4f5a6",
        "{0192f3a4-5b6c-7d8e-9fa0-b1c2d3e4f5a6}",
        "urn:uuid:0192f3a4-5b6c-7d8e-9fa0-b1c2d3e4f5a6",
        "0192f3a4-5b6c-7d8e-9fa0-b1c2d3e4f5a",
        "0192f3a4-5b6c-7d8e-9fa0-b1c2d3e4f5a6/..",
        "0192f3a4_5b6c_7d8e_9fa0_b1c2d3e4f5a6",
    ] {
        assert_eq!(DeviceId::parse(bad), Err(WebViewError::DeviceId), "{bad:?}");
    }
}

/// A one-character change anywhere in the ID, to any non-hex character, is refused, and a
/// change to another hex digit is another store: the folder name is always hex and hyphens.
#[test]
fn web_006_every_one_character_change_is_refused_or_another_device() {
    let original = DeviceId::parse(DEVICE).unwrap();
    for position in 0..DEVICE.len() {
        for probe in ['/', '.', '\\', ' ', '\0', 'g', 'G', '-', '0', 'f', '%', 'é'] {
            let mut text: Vec<char> = DEVICE.chars().collect();
            text[position] = probe;
            let text: String = text.into_iter().collect();
            match DeviceId::parse(&text) {
                Ok(device) => {
                    let name = device.folder_name();
                    assert!(
                        name.bytes().all(|b| b.is_ascii_hexdigit() || b == b'-'),
                        "{text:?}"
                    );
                    assert_eq!(device == original, text == DEVICE, "{text:?}");
                }
                Err(error) => assert_eq!(error, WebViewError::DeviceId),
            }
        }
    }
}
