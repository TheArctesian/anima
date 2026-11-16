//! PRJ-031: a list reads its next page from the `Link` header and stops at the last.

use crate::live::paging::next_page;

const FIRST: &str = "<https://api.github.com/repositories/500/pulls?per_page=100&page=2>; rel=\"next\", \
     <https://api.github.com/repositories/500/pulls?per_page=100&page=5>; rel=\"last\"";
const MIDDLE: &str = "<https://api.github.com/repositories/500/pulls?per_page=100&page=1>; rel=\"prev\", \
     <https://api.github.com/repositories/500/pulls?page=4&per_page=100>; rel=\"next\"";
const LAST: &str = "<https://api.github.com/repositories/500/pulls?per_page=100&page=4>; rel=\"prev\", \
     <https://api.github.com/repositories/500/pulls?per_page=100&page=1>; rel=\"first\"";

#[test]
fn prj_031_the_next_page_is_the_page_number_of_the_next_link() {
    assert_eq!(next_page(FIRST), Some(2));
    assert_eq!(next_page(MIDDLE), Some(4));
}

#[test]
fn prj_031_the_last_page_has_no_next_page() {
    assert_eq!(next_page(LAST), None);
    assert_eq!(next_page(""), None);
}

#[test]
fn prj_031_only_the_page_number_is_taken_from_a_link_to_another_host() {
    let hostile = "<https://evil.example/steal?page=7&token=x>; rel=\"next\"";

    assert_eq!(next_page(hostile), Some(7));
}

#[test]
fn prj_031_a_link_without_a_usable_page_number_ends_the_listing() {
    for link in [
        "<https://api.github.com/x?per_page=100>; rel=\"next\"",
        "<https://api.github.com/x?page=abc>; rel=\"next\"",
        "<https://api.github.com/x?page=0>; rel=\"next\"",
        "<https://api.github.com/x?per_page=100&xpage=3>; rel=\"next\"",
        "https://api.github.com/x?page=3; rel=\"next\"",
        "garbage",
    ] {
        assert_eq!(next_page(link), None, "{link}");
    }
}
