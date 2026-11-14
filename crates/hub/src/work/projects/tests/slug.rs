//! The slug of a new project: the clash cases, and a property over any name GitHub could send.

use proptest::prelude::*;

use crate::work::projects::slug::{for_repository, stem};

fn taken(slugs: &[&str]) -> Vec<String> {
    slugs.iter().map(|slug| (*slug).to_owned()).collect()
}

#[test]
fn pro_001_the_slug_is_the_repository_name_in_lower_case_with_dashes() {
    assert_eq!(for_repository("Arctesian/Anima", &[]), "anima");
    assert_eq!(for_repository("me/My_Site.io", &[]), "my-site-io");
    assert_eq!(for_repository("me/--odd__name--", &[]), "odd-name");
}

#[test]
fn pro_001_a_clash_adds_the_owner_then_a_number() {
    assert_eq!(for_repository("bob/site", &taken(&["site"])), "site-bob");
    assert_eq!(
        for_repository("bob/site", &taken(&["site", "site-bob"])),
        "site-bob-2"
    );
    assert_eq!(
        for_repository("bob/site", &taken(&["site", "site-bob", "site-bob-2"])),
        "site-bob-3"
    );
}

#[test]
fn pro_001_a_name_with_no_letter_or_digit_is_project() {
    assert_eq!(for_repository("me/___", &[]), "project");
    assert_eq!(for_repository("___/___", &taken(&["project"])), "project-2");
    assert_eq!(stem("me/___"), "project");
}

#[test]
fn pro_001_text_beyond_ascii_letters_and_digits_becomes_dashes() {
    assert_eq!(for_repository("me/café-ü", &[]), "caf");
    assert_eq!(for_repository("me/a\u{202e}b", &[]), "a-b");
}

proptest! {
    #[test]
    fn pro_001_every_slug_is_plain_short_free_and_starts_with_its_stem(
        full_name in ".{0,400}",
        taken in proptest::collection::vec("[a-z0-9-]{1,12}", 0..8),
    ) {
        let slug = for_repository(&full_name, &taken);
        prop_assert!(slug.len() <= 120, "{slug}");
        prop_assert!(!slug.is_empty());
        prop_assert!(slug.split('-').all(|part| !part.is_empty()
            && part.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())), "{slug}");
        prop_assert!(!taken.contains(&slug));
        prop_assert!(slug.starts_with(&stem(&full_name)));
    }
}
