//! Checking the time zone, the name and the targets of a window typed in the form (MW-001,
//! MW-003).

use super::validation::{DEVICE, fields, target, weekday_form};
use crate::fleet::maintenance::{
    TargetType,
    model::{Target, TargetForm, WindowForm},
    rules::validate,
};

#[test]
fn mw_001_a_time_zone_must_be_an_iana_name() {
    for bad in ["", "Mars/Olympus_Mons", "NZDT+13", "pacific auckland"] {
        let form = WindowForm {
            timezone: bad.into(),
            ..weekday_form()
        };
        assert_eq!(fields(&form), ["timezone"], "{bad:?}");
    }
    for good in [
        "UTC",
        "Pacific/Auckland",
        "America/New_York",
        "Europe/London",
    ] {
        let form = WindowForm {
            timezone: good.into(),
            ..weekday_form()
        };
        assert!(validate(&form).is_ok(), "{good}");
    }
}

#[test]
fn mw_003_a_window_has_one_to_one_hundred_targets() {
    let none = WindowForm {
        targets: vec![],
        ..weekday_form()
    };
    assert_eq!(fields(&none), ["targets"]);

    let tag_forms = |count: usize| -> Vec<TargetForm> {
        (0..count)
            .map(|n| target(TargetType::Tag, &format!("tag-{n}")))
            .collect()
    };
    let hundred = WindowForm {
        targets: tag_forms(100),
        ..weekday_form()
    };
    assert_eq!(validate(&hundred).unwrap().targets.len(), 100);
    let too_many = WindowForm {
        targets: tag_forms(101),
        ..weekday_form()
    };
    assert_eq!(fields(&too_many), ["targets"]);
}

#[test]
fn mw_003_device_and_check_targets_must_be_ids() {
    for kind in [TargetType::Device, TargetType::Check] {
        for bad in ["pve-01", "", "0190a000"] {
            let form = WindowForm {
                targets: vec![target(kind, bad)],
                ..weekday_form()
            };
            assert_eq!(fields(&form), ["targets"], "{kind:?} {bad:?}");
        }
    }
    let upper = WindowForm {
        targets: vec![target(TargetType::Device, &DEVICE.to_uppercase())],
        ..weekday_form()
    };
    let checked = validate(&upper).unwrap();
    assert_eq!(checked.targets[0].value(), DEVICE);
}

#[test]
fn mw_003_a_tag_follows_the_device_tag_rules() {
    let long = "x".repeat(33);
    for bad in ["", "   ", "line\nbreak", "bidi\u{202E}tag", long.as_str()] {
        let form = WindowForm {
            targets: vec![target(TargetType::Tag, bad)],
            ..weekday_form()
        };
        assert_eq!(fields(&form), ["targets"], "{bad:?}");
    }
    let padded = WindowForm {
        targets: vec![
            target(TargetType::Tag, "  infra "),
            target(TargetType::Tag, "x".repeat(32).as_str()),
        ],
        ..weekday_form()
    };
    assert_eq!(
        validate(&padded).unwrap().targets[0],
        Target::Tag("infra".into())
    );
}

#[test]
fn mw_003_a_repeated_target_is_kept_once_in_first_seen_order() {
    let form = WindowForm {
        targets: vec![
            target(TargetType::Tag, "b"),
            target(TargetType::Tag, "a"),
            target(TargetType::Tag, " b "),
        ],
        ..weekday_form()
    };
    assert_eq!(
        validate(&form).unwrap().targets,
        vec![Target::Tag("b".into()), Target::Tag("a".into())]
    );
}

#[test]
fn mw_001_a_name_is_one_to_100_plain_characters() {
    let long = "n".repeat(101);
    for bad in [
        "",
        "   ",
        "tab\there",
        "rtl\u{202E}name",
        "sep\u{2028}arator",
        long.as_str(),
    ] {
        let form = WindowForm {
            name: bad.into(),
            ..weekday_form()
        };
        assert_eq!(fields(&form), ["name"], "{bad:?}");
    }
    let edge = WindowForm {
        name: format!("  {}  ", "n".repeat(100)),
        ..weekday_form()
    };
    assert_eq!(validate(&edge).unwrap().name.len(), 100);
}
