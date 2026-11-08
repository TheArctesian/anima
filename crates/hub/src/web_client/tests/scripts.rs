//! The CSP hashes of `index.html`'s inline scripts (A-2).

use proptest::prelude::*;

use super::super::scripts::inline_script_hashes;

const FIRST: &str = "'sha256-fDYmfWjvur9AxjhDbjHe6RvExc05MbXhhrbgz2vvy/A='";
const SECOND: &str = "'sha256-VRcK3v7WYQJL3yHheBe4dsvLYjAYwFTyoPi2odncRio='";
const EMPTY: &str = "'sha256-47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU='";

#[test]
fn d_2_each_inline_script_gets_the_hash_of_its_exact_body_in_page_order() {
    let html = "<script>first()</script><p>text</p><script>second()</script>";
    assert_eq!(inline_script_hashes(html), [FIRST, SECOND]);
}

#[test]
fn d_2_a_script_with_src_gets_no_hash_in_any_spelling() {
    let html = "<script src=/a.js></script>\
                <SCRIPT type=module SRC=\"/b.js\"></SCRIPT>\
                <script defer\nsrc = '/c.js' ></script>\
                <script>first()</script>";
    assert_eq!(inline_script_hashes(html), [FIRST]);
}

#[test]
fn d_2_attributes_on_an_inline_script_do_not_change_its_hash() {
    let html = "<script type=\"module\" nonce=x data-src=\"/no.js\" data-y='a > b'>first()</script>\
                <Script\tid=boot>second()</sCrIpT >";
    assert_eq!(inline_script_hashes(html), [FIRST, SECOND]);
}

#[test]
fn d_2_an_attribute_value_that_says_src_is_not_a_src_attribute() {
    let html = "<script data-kind=\"src\" title=src>first()</script>";
    assert_eq!(inline_script_hashes(html), [FIRST]);
}

#[test]
fn d_2_other_elements_and_unclosed_scripts_give_no_hash() {
    assert_eq!(
        inline_script_hashes("<scripts>first()</scripts>"),
        Vec::<String>::new()
    );
    assert_eq!(
        inline_script_hashes("<script-x>first()</script-x>"),
        Vec::<String>::new()
    );
    assert_eq!(
        inline_script_hashes("<script>first()"),
        Vec::<String>::new()
    );
    assert_eq!(inline_script_hashes("<script"), Vec::<String>::new());
    assert_eq!(
        inline_script_hashes("<script type='x>"),
        Vec::<String>::new()
    );
}

#[test]
fn d_2_an_empty_inline_script_gets_the_hash_of_nothing() {
    assert_eq!(inline_script_hashes("<script></script>"), [EMPTY]);
}

#[test]
fn d_2_non_ascii_text_around_and_in_a_script_is_hashed_as_utf_8() {
    let html = "<p>Gerät ✓</p><script>first()</script><script>ü</script>";
    let hashes = inline_script_hashes(html);
    assert_eq!(hashes[0], FIRST);
    assert_eq!(hashes.len(), 2);
}

/// The CSP source of `text`, hashed here as the browser hashes the parsed script.
fn hash_of(text: &str) -> String {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    use sha2::{Digest as _, Sha256};
    format!("'sha256-{}'", STANDARD.encode(Sha256::digest(text)))
}

#[test]
fn d_2_a_crlf_in_a_script_is_hashed_as_the_lf_the_browser_parses() {
    let html = "<script>\r\n\tfirst();\r\n</script>";
    assert_eq!(inline_script_hashes(html), [hash_of("\n\tfirst();\n")]);
}

#[test]
fn d_2_a_lone_cr_in_a_script_is_hashed_as_lf() {
    assert_eq!(
        inline_script_hashes("<script>a()\rb()\r\r\nc()\r</script>"),
        [hash_of("a()\nb()\n\nc()\n")]
    );
}

/// Text that cannot open or close a script element or end a start tag.
fn plain() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9 ;(){}=.\"'ü]{0,40}"
}

/// Text built from the pieces the reader looks for, in any order, plus any characters.
fn tag_soup() -> impl Strategy<Value = String> {
    let piece = prop::sample::select(vec![
        "<script", "<SCRIPT", "</script", ">", "/", "src", "=", "\"", "'", " ", "\t", "x", "ü", "<",
    ]);
    let pieces = prop::collection::vec(piece, 0..40).prop_map(|pieces| pieces.concat());
    prop_oneof![pieces, any::<String>()]
}

proptest! {
    /// Any text: no panic, and never more hashes than script openings.
    #[test]
    fn d_2_any_page_reads_without_panic(html in tag_soup()) {
        let hashes = inline_script_hashes(&html);
        prop_assert!(hashes.len() <= html.to_ascii_lowercase().matches("<script").count());
        for hash in hashes {
            prop_assert!(hash.starts_with("'sha256-") && hash.ends_with('\''));
            prop_assert_eq!(hash.len(), 53);
        }
    }

    /// Every inline script among plain text and `src` scripts gets exactly one hash.
    #[test]
    fn d_2_one_hash_per_inline_script(
        parts in prop::collection::vec((plain(), plain(), any::<bool>()), 0..8),
    ) {
        let mut html = String::new();
        let mut inline = 0;
        for (text, body, loaded) in &parts {
            html.push_str(text);
            if *loaded {
                html.push_str("<script src=\"/x.js\">");
            } else {
                inline += 1;
                html.push_str("<script type=module>");
            }
            html.push_str(body);
            html.push_str("</script>");
        }
        prop_assert_eq!(inline_script_hashes(&html).len(), inline);
    }
}
