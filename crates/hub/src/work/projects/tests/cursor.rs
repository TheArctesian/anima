//! The K7 cursors over GitHub's pages: a hub page is cut from one GitHub page, every cursor
//! gives back its place, and any other text is an `after` error, never a panic (API-006).

use proptest::prelude::*;

use crate::work::projects::forge::cursor::{GITHUB_PAGE, Place, cut, encode, page_size, start};

fn at(page: u32, offset: usize) -> Place {
    Place { page, offset }
}

proptest! {
    #[test]
    fn api_006_a_cursor_gives_back_its_place(
        page in 1_u32..=10_000,
        offset in 0..=GITHUB_PAGE,
        size in 1_usize..=100,
    ) {
        let place = at(page, offset);
        prop_assert_eq!(start(Some(&encode(place, size)), size).unwrap(), place);
    }

    #[test]
    fn api_006_any_other_text_is_an_after_error_not_a_panic(text in ".{0,40}") {
        if let Err(error) = start(Some(&text), 20) {
            prop_assert_eq!(error.field, "after");
        }
    }

    #[test]
    fn api_006_the_pages_of_one_github_page_hold_each_row_once(
        size in 1_usize..=100,
        shown in 0_usize..=100,
    ) {
        let mut place = Place::START;
        let mut rows = Vec::new();
        loop {
            let page = cut(place, size, shown, false);
            rows.extend(page.rows.clone());
            prop_assert_eq!(page.cursors.len(), page.rows.len());
            match page.end_cursor.filter(|_| page.has_next_page) {
                Some(cursor) => place = start(Some(&cursor), size).unwrap(),
                None => break,
            }
        }
        prop_assert_eq!(rows, (0..shown).collect::<Vec<_>>());
    }
}

#[test]
fn api_006_after_with_another_first_is_refused() {
    let cursor = encode(at(2, 40), 20);
    assert_eq!(start(Some(&cursor), 20).unwrap(), at(2, 40));
    assert_eq!(start(Some(&cursor), 30).unwrap_err().field, "after");
}

#[test]
fn api_006_a_cursor_past_its_bounds_is_refused() {
    for text in [
        "g1.0.0.20",
        "g1.10001.0.20",
        "g1.1.101.20",
        "g1.1.0.20.9",
        "g1.+1.0.20",
        "g1.1.0",
        "p1.1.0.20",
        "g1.0000001.0.20",
    ] {
        assert_eq!(start(Some(text), 20).unwrap_err().field, "after", "{text}");
    }
}

#[test]
fn api_006_first_is_bounded_by_the_root() {
    assert_eq!(page_size(50, 50).unwrap(), 50);
    for first in [0, -1, 51] {
        assert_eq!(page_size(first, 50).unwrap_err().field, "first");
    }
}

#[test]
fn prj_031_a_full_github_page_goes_on_to_the_next_one() {
    let first = cut(Place::START, 30, GITHUB_PAGE, true);
    assert_eq!((first.rows, first.has_next_page), (0..30, true));
    let last = cut(at(1, 90), 30, GITHUB_PAGE, true);
    assert_eq!(last.rows, 90..100, "a hub page ends where GitHub's ends");
    assert_eq!(last.end_cursor, Some(encode(at(2, 0), 30)));
    assert!(last.has_next_page);
}

#[test]
fn prj_031_an_issue_page_of_only_pull_requests_still_goes_on() {
    let page = cut(Place::START, 30, 0, true);
    assert!(page.rows.is_empty());
    assert!(page.has_next_page);
    assert_eq!(page.end_cursor, Some(encode(at(2, 0), 30)));
}

#[test]
fn prj_031_a_short_github_page_is_the_end() {
    let page = cut(at(3, 0), 30, 12, false);
    assert_eq!((page.rows, page.has_next_page), (0..12, false));
    let empty = cut(Place::START, 30, 0, false);
    assert_eq!((empty.has_next_page, empty.end_cursor), (false, None));
}
