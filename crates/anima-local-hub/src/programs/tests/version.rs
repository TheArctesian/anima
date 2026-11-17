//! An update never puts older programs over newer ones.

use crate::programs::version::is_older;

#[test]
fn versions_are_compared_as_numbers() {
    assert!(is_older("0.9.0", Some("0.10.0")));
    assert!(is_older("1.2.3", Some("1.2.4")));
    assert!(is_older("0.99.99", Some("1.0.0")));
    assert!(!is_older("0.10.0", Some("0.9.0")));
}

#[test]
fn equal_versions_are_not_older() {
    assert!(!is_older("1.2.3", Some("1.2.3")));
}

#[test]
fn an_installed_version_that_is_missing_or_not_three_numbers_blocks_nothing() {
    for installed in [
        None,
        Some(""),
        Some("garbage"),
        Some("1.2"),
        Some("1.2.3.4"),
        Some("1.2.x"),
        Some("-1.0.0"),
    ] {
        assert!(!is_older("0.0.1", installed), "{installed:?}");
    }
}

#[test]
fn a_carried_version_that_is_not_three_numbers_blocks_nothing() {
    assert!(!is_older("trial", Some("9.9.9")));
}
