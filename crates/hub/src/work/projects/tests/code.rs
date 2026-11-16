//! Where a ref and a file are (PRJ-010, PRJ-033): a file's page keeps each name one part of
//! the address, and only 40 hex digits name a commit.

use crate::work::projects::code::address::{file, is_commit};

#[test]
fn prj_010_a_file_page_keeps_each_name_one_part_of_the_address() {
    assert_eq!(
        file(
            "https://github.com/octo/widgets",
            "feature/a#b",
            "docs/a b?#%.md"
        ),
        "https://github.com/octo/widgets/blob/feature/a%23b/docs/a%20b%3F%23%25.md"
    );
    assert_eq!(
        file("https://github.com/o/r/", "main", "\u{e9}.txt"),
        "https://github.com/o/r/blob/main/%C3%A9.txt"
    );
}

#[test]
fn prj_033_only_forty_hex_digits_name_a_commit() {
    assert!(is_commit(&"a1".repeat(20)));
    assert!(!is_commit(&"a1".repeat(19)));
    assert!(!is_commit("main"));
    assert!(!is_commit(&"g".repeat(40)));
}
