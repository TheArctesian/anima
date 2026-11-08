use reqwest::header::HeaderValue;
use serde_json::{Value, json};

use super::{apprise, email, ntfy, webhook};
use crate::notifications::{
    NotificationKind,
    template::{Rendered, Values, render},
};

fn rendered(title: &str, body: &str) -> Rendered {
    Rendered {
        title: title.to_owned(),
        body: body.to_owned(),
    }
}

fn values() -> Values {
    Values::from_payload(&json!({
        "device": "pi-1", "check": "ping", "status": "down", "previous": "up",
        "message": "timed out", "at": "2026-11-04T09:00:00+00:00",
    }))
}

const HOSTILE: &str = "a\r\nBcc: x@example.test\0b\u{202e}c\u{200b}d\u{85}e\u{2028}f";

#[test]
fn not_007_the_webhook_body_has_a_fixed_json_shape() {
    let body = webhook(
        &rendered("pi-1: ping is down", "ping went down"),
        NotificationKind::Down,
        &values(),
    );
    assert_eq!(
        body,
        json!({
            "kind": "down", "title": "pi-1: ping is down", "body": "ping went down",
            "status": "down", "previous": "up", "device": "pi-1", "check": "ping",
            "message": "timed out", "at": "2026-11-04T09:00:00+00:00", "days": null,
        })
    );
}

#[test]
fn not_007_a_webhook_value_with_quotes_and_braces_stays_one_json_string() {
    let values = Values::from_payload(&json!({
        "device": "\"},\"kind\":\"up\",\"x\":{\"", "message": "{message} \\ \n",
    }));
    let rendered = render("{device}", "{message}", &values).unwrap();
    let body = webhook(&rendered, NotificationKind::Down, &values);
    let text = serde_json::to_string(&body).unwrap();
    let back: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(back, body);
    assert_eq!(back["kind"], "down");
    assert_eq!(back["device"], "\"},\"kind\":\"up\",\"x\":{\"");
    assert_eq!(back.as_object().unwrap().len(), 10);
}

#[test]
fn not_007_a_webhook_with_days_carries_a_whole_number() {
    let values = Values {
        days: Some(7),
        ..values()
    };
    let body = webhook(
        &rendered("t", "b"),
        NotificationKind::CertificateExpiry,
        &values,
    );
    assert_eq!(body["days"], 7);
    assert_eq!(body["kind"], "certificate_expiry");
}

#[test]
fn not_007_ntfy_title_never_holds_a_control_character() {
    let sent = ntfy(&rendered(HOSTILE, "body"), 4).unwrap();
    let bytes = sent.title.as_bytes();
    assert!(bytes.iter().all(|b| *b >= 0x20 && *b != 0x7f));
    assert!(HeaderValue::from_bytes(bytes).is_ok());
    let text = std::str::from_utf8(bytes).unwrap();
    assert!(!text.contains(['\r', '\n', '\0', '\u{202e}', '\u{200b}', '\u{2028}']));
    assert_eq!(text.chars().filter(|c| c.is_control()).count(), 0);
}

#[test]
fn not_007_ntfy_keeps_non_ascii_text_and_the_channels_priority() {
    let sent = ntfy(
        &rendered("Pi \u{e9}t\u{e9} \u{1f6a8}", "line 1\r\nline 2\r3"),
        5,
    )
    .unwrap();
    assert_eq!(
        sent.title.as_bytes(),
        "Pi \u{e9}t\u{e9} \u{1f6a8}".as_bytes()
    );
    assert_eq!(sent.body, "line 1\nline 2\n3");
    assert_eq!(sent.priority, 5);
    assert_eq!(ntfy(&rendered("t", "b"), 0).unwrap().priority, 1);
    assert_eq!(ntfy(&rendered("t", "b"), 9).unwrap().priority, 5);
}

#[test]
fn not_007_a_very_long_title_is_still_a_header() {
    let long = rendered(&"\u{e9}".repeat(500), &"b".repeat(4000));
    let sent = ntfy(&long, 3).unwrap();
    assert_eq!(sent.title.len(), 1000);
    assert_eq!(sent.body.chars().count(), 4000);
}

#[test]
fn not_007_an_email_subject_has_no_line_break_and_the_body_is_plain_text() {
    let sent = email(&rendered(HOSTILE, "one\r\ntwo\0\u{202e}\tthree"));
    assert!(!sent.subject.contains(['\r', '\n', '\0']));
    assert!(sent.subject.starts_with("a  Bcc: x@example.test"));
    assert_eq!(sent.body, "one\ntwo\tthree");
}

#[test]
fn not_007_an_email_subject_built_with_hostile_values_stays_on_one_line() {
    let values = Values::from_payload(&json!({
        "device": "pi\r\nBcc: evil@example.test", "check": "c\u{2028}d",
    }));
    let rendered = render("{device}: {check}", "{device}", &values).unwrap();
    let sent = email(&rendered);
    assert_eq!(sent.subject, "piBcc: evil@example.test: cd");
    assert_eq!(sent.subject.lines().count(), 1);
}

#[test]
fn not_007_apprise_body_has_title_body_type_and_the_channels_tags() {
    let tags = vec!["ops".to_owned(), "pager".to_owned()];
    let body = apprise(&rendered("t", "b"), NotificationKind::Down, &tags);
    assert_eq!(
        Value::Object(body),
        json!({ "title": "t", "body": "b", "type": "failure", "format": "text", "tag": "ops,pager" })
    );
    let none = apprise(&rendered("t", "b"), NotificationKind::Up, &[]);
    assert!(!none.contains_key("tag"));
    assert_eq!(none["type"], "success");
}

#[test]
fn sec_005_apprise_body_never_carries_an_attachment_or_a_url() {
    let values = Values::from_payload(&json!({
        "message": "https://evil.example.test/x.png",
    }));
    for kind in [
        NotificationKind::Down,
        NotificationKind::Up,
        NotificationKind::StillDown,
        NotificationKind::CertificateExpiry,
        NotificationKind::DomainExpiry,
        NotificationKind::Test,
    ] {
        let rendered = render("{message}", "{message}", &values).unwrap();
        let body = apprise(&rendered, kind, &["a".to_owned()]);
        for key in body.keys() {
            let key = key.to_lowercase();
            assert!(!key.contains("attach") && !key.contains("url"), "{key}");
        }
    }
}

#[test]
fn not_007_every_encoder_cuts_hostile_text_to_plain_text() {
    let long = rendered(&format!("{HOSTILE}{}", "x".repeat(600)), HOSTILE);
    let body = webhook(&long, NotificationKind::Test, &Values::default());
    for key in ["title", "body"] {
        let text = body[key].as_str().unwrap();
        assert!(
            !text.contains(['\0', '\u{202e}', '\u{200b}', '\u{2028}']),
            "{key}"
        );
    }
    assert!(!body["title"].as_str().unwrap().contains(['\r', '\n']));
}
