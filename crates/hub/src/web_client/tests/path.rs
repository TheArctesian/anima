//! What a request path asks for, in its plain and encoded spellings (K7).

use proptest::prelude::*;

use super::super::path::{Wanted, wanted};

#[test]
fn sec_001_dot_segments_are_hidden_in_every_spelling() {
    for path in [
        "/.env",
        "/a/.git/config",
        "/..",
        "/../outside.txt",
        "/a/./b",
        "/%2e%2e/outside.txt",
        "/%2E%2e%2Foutside.txt",
        "/..%2foutside.txt",
        "/%2eenv",
        "/a/..\\outside.txt",
        "/a%5c..%5coutside.txt",
        "/a%5C%2e%2e%5Coutside.txt",
        "/_app/../outside.txt",
        "/_app/immutable/%2e%2e/x.js",
        "/a%00b",
    ] {
        assert_eq!(wanted(path), Wanted::Hidden, "{path}");
    }
}

#[test]
fn d_2_root_and_index_html_are_the_index() {
    assert_eq!(wanted("/"), Wanted::Index);
    assert_eq!(wanted("/index.html"), Wanted::Index);
    assert_eq!(wanted("/index%2ehtml"), Wanted::Index);
}

#[test]
fn d_2_app_paths_are_assets_in_any_encoding() {
    assert_eq!(
        wanted("/_app/version.json"),
        Wanted::Asset { immutable: false }
    );
    assert_eq!(
        wanted("/_app/immutable/a.js"),
        Wanted::Asset { immutable: true }
    );
    assert_eq!(
        wanted("/%5fapp/immutable/a.js"),
        Wanted::Asset { immutable: true }
    );
    assert_eq!(
        wanted("/_app%2Fimmutable%2fa.js"),
        Wanted::Asset { immutable: true }
    );
}

#[test]
fn d_2_other_paths_are_pages() {
    for path in [
        "/devices",
        "/devices/0190a000",
        "/robots.txt",
        "/_app",
        "/a.b/c",
        "/%zz",
        "/%",
    ] {
        assert_eq!(wanted(path), Wanted::Page, "{path}");
    }
}

proptest! {
    /// Any path: no panic; a path whose plain segments start with `.` is always hidden.
    #[test]
    fn sec_001_any_path_is_classified_and_dot_segments_stay_hidden(
        before in "[a-z/]{0,10}",
        after in "[a-z/]{0,10}",
        dot in prop::sample::select(vec![".", "%2e", "%2E"]),
        separator in prop::sample::select(vec!["/", "\\", "%2f", "%5c", "%5C"]),
        anything in any::<String>(),
    ) {
        let _ = wanted(&anything);
        let path = format!("/{before}{separator}{dot}{after}");
        prop_assert_eq!(wanted(&path), Wanted::Hidden);
    }
}
