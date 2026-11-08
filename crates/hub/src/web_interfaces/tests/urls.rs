//! WEB-001 and D-4, D-6: the URL and label rules and the origin comparison.

use crate::web_interfaces::{
    OpenIn, WebInterfaceInput,
    rules::{self, same_origin, validate},
};

fn input(label: Option<&str>, url: &str) -> WebInterfaceInput {
    WebInterfaceInput {
        label: label.map(str::to_owned),
        url: url.to_owned(),
        open_in: OpenIn::AnimaTab,
    }
}

#[test]
fn web_001_accepts_http_and_https_addresses_and_stores_them_trimmed() {
    for (typed, stored) in [
        ("http://ha.home.lan:8123/", "http://ha.home.lan:8123/"),
        ("  https://nas.lan/ui?tab=1  ", "https://nas.lan/ui?tab=1"),
        ("HTTP://10.0.0.5", "HTTP://10.0.0.5"),
        ("https://[fd00::1]:8443/x", "https://[fd00::1]:8443/x"),
    ] {
        let new = validate(&input(None, typed)).unwrap();
        assert_eq!(new.url, stored, "{typed}");
    }
}

#[test]
fn web_001_refuses_every_bad_url_shape_on_the_url_field() {
    let long = format!("https://a.lan/{}", "x".repeat(2048));
    for bad in [
        "",
        "   ",
        "nas.lan",
        "ftp://nas.lan/",
        "javascript:alert(1)",
        "file:///etc/passwd",
        "http:example.com",
        "http://",
        "https:///path",
        "http://user@nas.lan/",
        "http://user:pw@nas.lan/",
        "http://:pw@nas.lan/",
        "https://nas.lan/#top",
        "https://nas.lan/#",
        "https://nas.lan/a b",
        "https://nas.lan/\nx",
        "https://nas.\tlan/",
        "http://@nas.lan/",
        "http://:@nas.lan/",
        long.as_str(),
    ] {
        let errors = validate(&input(None, bad)).err().unwrap_or_default();
        assert_eq!(errors.len(), 1, "{bad:?}");
        assert_eq!(errors[0].field, "url", "{bad:?}");
        assert!(
            !errors[0].message.contains("pw@"),
            "no input in the message"
        );
    }
}

#[test]
fn web_001_refuses_invisible_format_characters_in_a_url() {
    for c in [
        '\u{200b}',
        '\u{202e}',
        '\u{feff}',
        '\u{2060}',
        '\u{ad}',
        '\u{e0041}',
    ] {
        let url = format!("https://nas{c}.lan/");
        let errors = validate(&input(None, &url)).err().unwrap_or_default();
        assert_eq!(errors.len(), 1, "{c:?}");
        assert_eq!(errors[0].field, "url");
    }
}

#[test]
fn web_001_the_label_defaults_to_the_host_when_empty_or_missing() {
    for label in [None, Some(""), Some("   ")] {
        let new = validate(&input(label, "http://ha.home.lan:8123/x")).unwrap();
        assert_eq!(new.label, "ha.home.lan");
    }
    let new = validate(&input(Some("  Home Assistant "), "http://ha.lan/")).unwrap();
    assert_eq!(new.label, "Home Assistant");
}

#[test]
fn web_001_refuses_a_long_label_and_control_or_direction_characters() {
    for bad in [
        "x".repeat(101),
        "a\nb".to_owned(),
        "a\u{202e}b".to_owned(),
        "a\u{0}b".to_owned(),
    ] {
        let errors = validate(&input(Some(&bad), "http://ha.lan/"))
            .err()
            .unwrap();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field, "label");
    }
    assert!(validate(&input(Some(&"x".repeat(100)), "http://ha.lan/")).is_ok());
}

#[test]
fn q2_11_a_label_with_a_line_or_paragraph_separator_is_refused() {
    for c in ['\u{2028}', '\u{2029}'] {
        let label = format!("a{c}b");
        let errors = validate(&input(Some(&label), "http://ha.lan/"))
            .err()
            .unwrap();
        assert_eq!(errors.len(), 1, "{c:?}");
        assert_eq!(errors[0].field, "label");
    }
}

#[test]
fn web_001_reports_the_url_and_the_label_together() {
    let errors = validate(&input(Some("a\nb"), "ftp://x")).err().unwrap();
    let fields: Vec<_> = errors.iter().map(|e| e.field).collect();
    assert_eq!(fields, ["url", "label"]);
}

#[test]
fn d_6_origin_is_scheme_host_and_port() {
    assert!(same_origin("https://nas.lan/a", "https://nas.lan/b?x=1"));
    assert!(same_origin("http://nas.lan/", "http://NAS.lan:80/x"));
    assert!(!same_origin("https://nas.lan/", "http://nas.lan/"));
    assert!(!same_origin("https://nas.lan/", "https://nas.lan:8443/"));
    assert!(!same_origin("https://nas.lan/", "https://nas2.lan/"));
    assert!(!same_origin("not a url", "not a url"));
}

#[test]
fn web_008_only_https_addresses_can_take_a_pin() {
    assert!(rules::is_https("https://nas.lan/"));
    assert!(!rules::is_https("http://nas.lan/"));
}
