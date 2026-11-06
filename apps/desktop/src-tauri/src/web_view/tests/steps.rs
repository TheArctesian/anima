//! WEB-005: the toolbar's back, forward and reload.

use crate::web_view::{WebViewError, history::Step};

#[test]
fn web_005_back_and_forward_are_fixed_history_scripts() {
    assert_eq!(
        Step::parse("BACK").unwrap().script(),
        Some("history.back()")
    );
    assert_eq!(
        Step::parse("FORWARD").unwrap().script(),
        Some("history.forward()")
    );
}

#[test]
fn web_005_reload_is_the_web_view_s_own() {
    assert_eq!(Step::parse("RELOAD"), Ok(Step::Reload));
    assert_eq!(Step::Reload.script(), None);
}

#[test]
fn web_005_any_other_step_is_refused() {
    for bad in ["back", "Back", "", "BACK ", "STOP", "history.go(-2)"] {
        assert_eq!(Step::parse(bad), Err(WebViewError::Step), "{bad:?}");
    }
}
