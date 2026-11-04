//! Size and character limits on operator text, which reaches logs that cannot be cleaned
//! (LOG-004, DEV-010).

use super::*;
use super::{fields, new_device, ssh};

fn device_with(change: impl FnOnce(&mut NewDeviceInput)) -> Result<NewDevice, Vec<&'static str>> {
    let mut input = new_device("pve-01", "10.10.0.2");
    change(&mut input);
    validate_new_device(&input).map_err(|errors| fields(&errors))
}

/// A newline, a right-to-left override and an isolate, NUL, the line and paragraph separators,
/// the left-to-right and right-to-left marks, and the Arabic letter mark.
const UNSAFE: [&str; 9] = [
    "a\nb",
    "a\u{202E}b",
    "a\u{2067}b",
    "a\0b",
    "a\u{2028}b",
    "a\u{2029}b",
    "a\u{200E}b",
    "a\u{200F}b",
    "a\u{061C}b",
];

#[test]
fn log_004_control_and_direction_characters_are_rejected_in_every_logged_text() {
    for bad in UNSAFE {
        assert_eq!(
            device_with(|i| i.name = bad.into()).unwrap_err(),
            ["name"],
            "{bad:?}"
        );
        assert_eq!(
            device_with(|i| i.kind = bad.into()).unwrap_err(),
            ["kind"],
            "{bad:?}"
        );
        assert_eq!(
            device_with(|i| i.tags = vec![bad.into()]).unwrap_err(),
            ["tags"],
            "{bad:?}"
        );
        let user = device_with(|i| i.ssh = Some(ssh(true, None, bad))).unwrap_err();
        assert_eq!(user, ["ssh.username"], "{bad:?}");
        let site = NewSiteInput {
            name: bad.into(),
            subnet: "10.40.0.0/24".parse().unwrap(),
        };
        assert_eq!(
            fields(&validate_new_site(&site).unwrap_err()),
            ["name"],
            "{bad:?}"
        );
        let change = DeviceChangeInput {
            name: Some(bad.into()),
            ..DeviceChangeInput::default()
        };
        assert_eq!(
            fields(&validate_change(&change).unwrap_err()),
            ["name"],
            "{bad:?}"
        );
    }
    assert!(device_with(|i| i.name = "Büro ñ 日本".into()).is_ok());
}

#[test]
fn log_004_the_characters_just_outside_the_unsafe_ranges_are_accepted() {
    for ok in [
        "a\u{2027}b",
        "a\u{202F}b",
        "a\u{2065}b",
        "a\u{206A}b",
        "a\u{200D}b",
    ] {
        assert!(device_with(|i| i.name = ok.into()).is_ok(), "{ok:?}");
    }
}

#[test]
fn dev_010_a_device_has_at_most_32_tags() {
    let tags = |count: usize| (0..count).map(|n| format!("t{n}")).collect::<Vec<_>>();
    assert_eq!(device_with(|i| i.tags = tags(32)).unwrap().tags.len(), 32);
    assert_eq!(device_with(|i| i.tags = tags(33)).unwrap_err(), ["tags"]);
}

#[test]
fn dev_010_an_ssh_user_name_has_at_most_64_characters() {
    let user = |count: usize| "u".repeat(count);
    assert!(device_with(|i| i.ssh = Some(ssh(true, None, &user(64)))).is_ok());
    let over = device_with(|i| i.ssh = Some(ssh(true, None, &user(65)))).unwrap_err();
    assert_eq!(over, ["ssh.username"]);
}

#[test]
fn dev_010_a_filter_has_a_name_of_at_most_64_characters_and_at_most_32_tags() {
    let filter = |name: usize, tags: usize| DeviceFilter {
        name: Some("n".repeat(name)),
        tags: vec!["t".into(); tags],
        site: None,
    };
    assert!(validate_filter(filter(64, 32)).is_ok());
    assert_eq!(
        fields(&validate_filter(filter(65, 32)).unwrap_err()),
        ["name"]
    );
    assert_eq!(
        fields(&validate_filter(filter(64, 33)).unwrap_err()),
        ["tags"]
    );
    assert_eq!(
        fields(&validate_filter(filter(65, 33)).unwrap_err()),
        ["name", "tags"]
    );
}

#[test]
fn dev_010_the_k1_default_of_kind_is_the_rule_default() {
    use async_graphql::{EmptySubscription, Schema};

    use crate::devices::graphql::{Mutation, Query};

    let sdl = Schema::build(Query, Mutation, EmptySubscription)
        .finish()
        .sdl();
    let expected = format!("kind: String = \"{DEFAULT_KIND}\"");
    let input = sdl.split("input CreateDeviceInput").nth(1).unwrap();
    let input = input.split('}').next().unwrap();
    assert!(input.contains(&expected), "{input}");
}
