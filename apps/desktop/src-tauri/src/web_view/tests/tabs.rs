//! D-3: tab IDs become web view labels; the open tabs are bounded and keep each pin.

use crate::web_view::{
    Fingerprint, PinnedOrigin, Tabs, WebUrl, WebViewError,
    tabs::{TABS_MAX, Tab, TabId},
};

fn id(text: &str) -> TabId {
    TabId::parse(text).unwrap()
}

fn tab(url: &str) -> Tab {
    let url = WebUrl::parse(url).unwrap();
    let origin = url
        .is_https()
        .then(|| PinnedOrigin::parse(url.as_str()).unwrap());
    Tab {
        url,
        origin,
        pin: None,
        popped_out: false,
    }
}

fn fingerprint() -> Fingerprint {
    "cd".repeat(32).parse().unwrap()
}

#[test]
fn d_3_a_tab_id_names_the_web_view_and_its_pop_out_window() {
    let tab = id("tab_01-A");
    assert_eq!(tab.view_label(), "web-tab_01-A");
    assert_eq!(tab.window_label(), "pop-tab_01-A");
    assert!(TabId::parse(&"x".repeat(128)).is_ok());
}

#[test]
fn d_3_a_tab_id_is_1_to_128_letters_digits_underscores_or_hyphens() {
    for bad in [
        String::new(),
        "x".repeat(129),
        "main".to_owned() + " ",
        "a/b".to_owned(),
        "a:b".to_owned(),
        "a.b".to_owned(),
        "é".to_owned(),
        "a\0".to_owned(),
    ] {
        assert_eq!(TabId::parse(&bad), Err(WebViewError::TabId), "{bad:?}");
    }
}

#[test]
fn d_3_no_tab_id_can_make_the_main_label() {
    assert_ne!(id("main").view_label(), "main");
    assert_ne!(id("main").window_label(), "main");
}

#[test]
fn web_004_a_tab_is_reserved_once() {
    let tabs = Tabs::default();
    tabs.reserve(id("a"), tab("http://192.0.2.7/")).unwrap();
    assert_eq!(
        tabs.reserve(id("a"), tab("http://192.0.2.8/")),
        Err(WebViewError::TabOpen)
    );
    assert_eq!(tabs.get(&id("a")).unwrap(), tab("http://192.0.2.7/"));
    assert!(tabs.remove(&id("a")).is_some());
    assert_eq!(tabs.get(&id("a")), Err(WebViewError::UnknownTab));
    assert!(tabs.remove(&id("a")).is_none());
}

#[test]
fn web_004_the_open_tabs_are_bounded() {
    let tabs = Tabs::default();
    for index in 0..TABS_MAX {
        tabs.reserve(id(&format!("t{index}")), tab("http://192.0.2.7/"))
            .unwrap();
    }
    assert_eq!(
        tabs.reserve(id("one-more"), tab("http://192.0.2.7/")),
        Err(WebViewError::TabLimit)
    );
    tabs.remove(&id("t0"));
    assert!(
        tabs.reserve(id("one-more"), tab("http://192.0.2.7/"))
            .is_ok()
    );
}

#[test]
fn web_008_a_pin_belongs_to_the_tab_s_https_origin() {
    let tabs = Tabs::default();
    tabs.reserve(id("s"), tab("https://router.lan:8443/admin"))
        .unwrap();
    let pinned = tabs.set_pin(&id("s"), fingerprint()).unwrap();
    let pin = pinned.pin.unwrap();
    assert_eq!(pin.fingerprint, fingerprint());
    assert_eq!(
        pin.origin,
        PinnedOrigin::parse("https://router.lan:8443/").unwrap()
    );
    assert_eq!(tabs.get(&id("s")).unwrap().pin, Some(pin));
}

#[test]
fn web_008_an_http_tab_takes_no_pin() {
    let tabs = Tabs::default();
    tabs.reserve(id("p"), tab("http://192.0.2.7/")).unwrap();
    assert_eq!(
        tabs.set_pin(&id("p"), fingerprint()),
        Err(WebViewError::PinScheme)
    );
    assert_eq!(
        tabs.set_pin(&id("missing"), fingerprint()),
        Err(WebViewError::UnknownTab)
    );
}

#[test]
fn web_004_popping_out_is_marked_once() {
    let tabs = Tabs::default();
    let generation = tabs.reserve(id("w"), tab("http://192.0.2.7/")).unwrap();
    assert_eq!(tabs.pop_out(&id("w")), Ok(Some(generation)));
    assert_eq!(tabs.pop_out(&id("w")), Ok(None));
    assert!(tabs.get(&id("w")).unwrap().popped_out);
    assert_eq!(tabs.pop_out(&id("x")), Err(WebViewError::UnknownTab));
}

#[test]
fn web_004_a_pop_out_that_did_not_happen_is_taken_back_for_its_own_reservation_only() {
    let tabs = Tabs::default();
    let old = tabs.reserve(id("w"), tab("http://192.0.2.7/")).unwrap();
    tabs.pop_out(&id("w")).unwrap();
    tabs.undo_pop_out(&id("w"), old);
    assert!(!tabs.get(&id("w")).unwrap().popped_out);
    tabs.remove(&id("w"));
    let new = tabs.reserve(id("w"), tab("http://192.0.2.7/")).unwrap();
    tabs.pop_out(&id("w")).unwrap();
    tabs.undo_pop_out(&id("w"), old);
    assert!(tabs.get(&id("w")).unwrap().popped_out);
    tabs.undo_pop_out(&id("w"), new);
    assert!(!tabs.get(&id("w")).unwrap().popped_out);
}

#[test]
fn web_004_a_tab_opened_again_is_a_new_reservation_the_old_one_cannot_end() {
    let tabs = Tabs::default();
    let old = tabs.reserve(id("r"), tab("http://192.0.2.7/")).unwrap();
    assert!(tabs.holds(&id("r"), old));
    tabs.remove(&id("r"));
    assert!(!tabs.holds(&id("r"), old), "closed while its open waited");
    let new = tabs.reserve(id("r"), tab("http://192.0.2.8/")).unwrap();
    assert_ne!(old, new);
    assert!(!tabs.holds(&id("r"), old), "closed and opened again");
    assert!(tabs.holds(&id("r"), new));
    assert_eq!(tabs.release(&id("r"), old), None);
    assert_eq!(tabs.get(&id("r")).unwrap(), tab("http://192.0.2.8/"));
    assert_eq!(tabs.release(&id("r"), new), Some(tab("http://192.0.2.8/")));
    assert_eq!(tabs.get(&id("r")), Err(WebViewError::UnknownTab));
    assert!(!tabs.holds(&id("missing"), new));
}

/// Every character in every position: a tab ID is accepted only if each character is one
/// of `A-Z a-z 0-9 _ -`, so no label can hold another.
#[test]
fn d_3_every_character_outside_the_tab_id_set_is_refused_in_any_position() {
    let probes = (0..=0x2FF_u32)
        .chain([0x2028, 0x202E, 0xFEFF, 0xFF0F, 0x1F600])
        .filter_map(char::from_u32);
    for probe in probes {
        let allowed = probe.is_ascii_alphanumeric() || probe == '_' || probe == '-';
        for text in [
            format!("{probe}"),
            format!("a{probe}"),
            format!("{probe}b"),
            format!("a{probe}b"),
        ] {
            let parsed = TabId::parse(&text);
            assert_eq!(parsed.is_ok(), allowed, "{text:?}");
            if let Ok(id) = parsed {
                assert!(
                    id.view_label()
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
                );
            }
        }
    }
}
