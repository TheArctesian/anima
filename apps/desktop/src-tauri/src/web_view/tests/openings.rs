//! WEB-004, D-6: `web_view_open`'s arguments are read whole and checked before any web view
//! exists; the caller must be the main web view.

use serde_json::{Value, json};

use crate::web_view::{WebViewError, caller, opening::Opening};

const DEVICE: &str = "0192f3a4-5b6c-7d8e-9fa0-b1c2d3e4f5a6";

fn arguments(url: &str, pin: Option<&str>) -> Value {
    json!({
        "tabId": "tab-1",
        "deviceId": DEVICE,
        "url": url,
        "pin": pin,
        "bounds": { "x": 0, "y": 48, "width": 1024, "height": 700 },
    })
}

fn checked(body: &Value, pinning: bool) -> Result<(), WebViewError> {
    Opening::read(body)?.check(pinning).map(|_| ())
}

#[test]
fn web_004_an_http_opening_is_checked_into_a_new_tab() {
    let new = Opening::read(&arguments("http://192.0.2.7/", None))
        .unwrap()
        .check(false)
        .unwrap();
    assert_eq!(new.id.view_label(), "web-tab-1");
    assert_eq!(new.device.folder_name(), DEVICE);
    assert_eq!(new.tab.url.as_str(), "http://192.0.2.7/");
    assert_eq!((new.tab.origin, new.tab.pin), (None, None));
    assert!(!new.tab.popped_out);
}

#[test]
fn d_6_an_https_opening_is_refused_while_no_platform_pins() {
    let body = arguments("https://192.0.2.7/", Some(&"ab".repeat(32)));
    assert_eq!(checked(&body, false), Err(WebViewError::PinningUnavailable));
    let new = Opening::read(&body).unwrap().check(true).unwrap();
    assert_eq!(
        new.tab.pin.unwrap().fingerprint.to_string(),
        "ab".repeat(32)
    );
    assert!(new.tab.origin.is_some());
}

#[test]
fn web_004_each_field_is_checked() {
    let mut bad_tab = arguments("http://192.0.2.7/", None);
    bad_tab["tabId"] = json!("a/b");
    let mut bad_device = arguments("http://192.0.2.7/", None);
    bad_device["deviceId"] = json!("../x");
    let mut bad_bounds = arguments("http://192.0.2.7/", None);
    bad_bounds["bounds"]["width"] = json!(-1);
    let cases = [
        (bad_tab, WebViewError::TabId),
        (bad_device, WebViewError::DeviceId),
        (arguments("file:///etc/hosts", None), WebViewError::Scheme),
        (
            arguments("http://192.0.2.7/", Some("zz")),
            WebViewError::Fingerprint,
        ),
        (
            arguments("http://192.0.2.7/", Some(&"ab".repeat(32))),
            WebViewError::PinScheme,
        ),
        (bad_bounds, WebViewError::Bounds),
    ];
    for (body, error) in cases {
        assert_eq!(checked(&body, true), Err(error), "{body}");
    }
}

#[test]
fn log_007_a_wrong_shape_is_arguments_whatever_the_field() {
    let mut extra = arguments("http://192.0.2.7/", None);
    extra["window"] = json!("main");
    let mut missing = arguments("http://192.0.2.7/", None);
    missing.as_object_mut().unwrap().remove("bounds");
    let mut wrong_type = arguments("http://192.0.2.7/", None);
    wrong_type["url"] = json!(7);
    for body in [
        extra,
        missing,
        wrong_type,
        json!([]),
        json!("x"),
        Value::Null,
    ] {
        assert_eq!(
            Opening::read(&body).map(|_| ()),
            Err(WebViewError::Arguments),
            "{body}"
        );
    }
}

#[test]
fn web_007_only_the_main_web_view_may_drive_web_views() {
    assert_eq!(caller("main"), Ok(()));
    for other in ["web-tab-1", "pop-tab-1", "Main", "main ", ""] {
        assert_eq!(caller(other), Err(WebViewError::Caller), "{other:?}");
    }
}
