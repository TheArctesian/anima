//! The version parser and order (BLD-002), with property tests over arbitrary text.

use proptest::prelude::*;

use crate::builds::version::Version;

#[test]
fn bld_002_versions_order_part_by_part() {
    let parse = |text: &str| text.parse::<Version>().unwrap();
    assert!(parse("0.1.10") > parse("0.1.9"));
    assert!(parse("1.0.0") > parse("0.99.99"));
    assert_eq!(parse("0.1.12").next_patch(), Some(parse("0.1.13")));
    assert_eq!(parse("0.1.999999999").next_patch(), None);
}

#[test]
fn bld_002_only_three_plain_numbers_are_a_version() {
    for bad in [
        "",
        "0",
        "0.1",
        "0.1.2.3",
        "01.1.2",
        "0.01.2",
        "0.1.02",
        "+1.2.3",
        "-1.2.3",
        "1.2.3 ",
        " 1.2.3",
        "1.2.3-beta",
        "1..3",
        "1.2.",
        "a.b.c",
        "1234567890.0.0",
        "1.2.٣",
    ] {
        assert!(bad.parse::<Version>().is_err(), "{bad:?}");
    }
}

proptest! {
    #[test]
    fn bld_002_a_version_reads_back_its_own_text(
        major in 0_u32..=999_999_999,
        minor in 0_u32..=999_999_999,
        patch in 0_u32..=999_999_999,
    ) {
        let version = Version::new(major, minor, patch);
        prop_assert_eq!(version.to_string().parse::<Version>(), Ok(version));
    }

    #[test]
    fn bld_002_any_accepted_text_is_the_canonical_spelling(text in "\\PC{0,24}") {
        if let Ok(version) = text.parse::<Version>() {
            prop_assert_eq!(version.to_string(), text);
        }
    }

    #[test]
    fn bld_002_the_order_is_the_order_of_the_parts(
        a in (0_u32..50, 0_u32..50, 0_u32..50),
        b in (0_u32..50, 0_u32..50, 0_u32..50),
    ) {
        let (left, right) = (Version::new(a.0, a.1, a.2), Version::new(b.0, b.1, b.2));
        prop_assert_eq!(left.cmp(&right), a.cmp(&b));
    }
}
