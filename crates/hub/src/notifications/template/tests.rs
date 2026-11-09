use serde_json::json;

use super::{BODY_MAX, Invalid, TITLE_MAX, Template, Values, field_errors, render};

fn down() -> Values {
    Values::from_payload(&json!({
        "device": "pi-1", "check": "ping", "status": "down", "previous": "up",
        "message": "timed out", "at": "2026-11-04T09:00:00+00:00", "days": null,
    }))
}

fn shown(template: &str, values: &Values) -> String {
    Template::parse(template).unwrap().render(values)
}

#[test]
fn not_007_every_placeholder_is_replaced_by_its_value() {
    let values = Values {
        days: Some(7),
        ..down()
    };
    assert_eq!(
        shown(
            "{device}|{check}|{status}|{previous}|{message}|{at}|{days}",
            &values
        ),
        "pi-1|ping|down|up|timed out|2026-11-04T09:00:00+00:00|7"
    );
}

#[test]
fn d_21_a_missing_value_renders_as_an_empty_string() {
    assert_eq!(shown("[{days}][{message}]", &Values::default()), "[][]");
    assert_eq!(shown("expires in {days} days", &down()), "expires in  days");
}

#[test]
fn d_21_an_unknown_placeholder_is_refused_and_named() {
    for text in ["{host}", "{}", "{ device }", "{Device}", "a {nope} b"] {
        assert!(
            matches!(Template::parse(text), Err(Invalid::Unknown(_))),
            "{text}"
        );
    }
    assert_eq!(
        Template::parse("{host}").err(),
        Some(Invalid::Unknown("host".into()))
    );
}

#[test]
fn d_21_an_unbalanced_or_stray_brace_is_refused() {
    for text in ["{device", "{", "x {device {check}", "{device{"] {
        assert_eq!(
            Template::parse(text).err(),
            Some(Invalid::Unclosed),
            "{text}"
        );
    }
    for text in ["}", "device}", "{device}}x}", "{device} }"] {
        assert_eq!(Template::parse(text).err(), Some(Invalid::Stray), "{text}");
    }
}

#[test]
fn d_21_a_literal_brace_is_written_twice() {
    assert_eq!(shown("{{device}}", &down()), "{device}");
    assert_eq!(shown("{{{device}}}", &down()), "{pi-1}");
    assert_eq!(shown("a {{ b }} c", &down()), "a { b } c");
}

#[test]
fn d_21_a_value_with_braces_is_not_expanded_again() {
    let values = Values::from_payload(&json!({
        "device": "{message}", "check": "{{check}}", "message": "{device} }", "status": "{",
    }));
    assert_eq!(
        shown("{device}/{check}/{message}/{status}", &values),
        "{message}/{{check}}/{device} }/{"
    );
}

#[test]
fn not_007_values_are_cleaned_again_from_the_payload() {
    let values = Values::from_payload(&json!({
        "device": "a\r\nb\0c\u{202e}d\u{200b}e",
        "message": "x".repeat(10_000),
        "check": 7,
        "days": "3",
    }));
    assert_eq!(values.device.as_deref(), Some("abcde"));
    assert_eq!(values.message.map(|m| m.chars().count()), Some(500));
    assert_eq!(values.check, None);
    assert_eq!(values.days, None);
}

#[test]
fn not_007_the_rendered_text_is_cut_at_the_template_limits() {
    let values = Values::default();
    let rendered = render(&"t".repeat(TITLE_MAX), &"b".repeat(BODY_MAX), &values).unwrap();
    assert_eq!(rendered.title.chars().count(), TITLE_MAX);
    let long = Values {
        message: Some("m".repeat(500)),
        ..Values::default()
    };
    let rendered = render(&"{message}".repeat(10), "{message}", &long).unwrap();
    assert_eq!(rendered.title.chars().count(), TITLE_MAX);
}

#[test]
fn not_007_a_stored_template_that_does_not_parse_is_an_error_not_a_guess() {
    assert_eq!(
        render("{oops}", "x", &Values::default()).err(),
        Some(Invalid::Unknown("oops".into()))
    );
}

#[test]
fn not_007_field_errors_name_title_and_body() {
    let errors = field_errors("{x}", "ok {check}");
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].field, "template.title");
    assert!(errors[0].message.contains("{x} is not a placeholder"));
    assert!(errors[0].message.contains("{device}"));
    let errors = field_errors("ok", &format!("{}}}", "b".repeat(BODY_MAX)));
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].field, "template.body");
    assert!(errors[0].message.contains("at most 4000"));
    assert_eq!(field_errors("{device}", "{{}}").len(), 0);
}

#[test]
fn log_007_an_error_message_repeats_at_most_a_short_name_never_the_template() {
    let secret = format!("{{{}}} tail-s3cr3t", "n".repeat(200));
    let errors = field_errors(&secret, "");
    assert!(!errors[0].message.contains("tail-s3cr3t"));
    assert!(errors[0].message.len() < 250);
}
