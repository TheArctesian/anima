//! Where a new comment is looked for when it is read back (OD-E14).

use super::back::pages;

#[test]
fn a_new_comment_is_looked_for_on_the_last_page_then_the_one_before() {
    assert_eq!(pages(0), [1], "a count that has not caught up");
    assert_eq!(pages(1), [1]);
    assert_eq!(pages(100), [1]);
    assert_eq!(pages(101), [2, 1]);
    assert_eq!(pages(250), [3, 2]);
    assert_eq!(pages(u64::MAX).len(), 2, "never more than two reads");
}
