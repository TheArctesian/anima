//! The webhook rules: an https URL whose origin alone is shown, and headers that cannot frame
//! or redirect the request (CR-14 items 1, 3, 5, 6 and 7; SEC-005).

use super::{
    Draft, Kept, NONE, Prepared, Secrets, Settings, SettingsDraft, WebhookDraft, WebhookMethod,
    WebhookSecrets, WebhookSettings, accepted, draft, errors_of, fields, new_errors,
    prepare_change, prepare_new, quotes_none, refused, secret, text, webhook,
};
use crate::{api_errors::FieldError, credentials::Secret};

const URL: &str = "https://hooks.example.test/path/s3cr3t-path?token=s3cr3t-query";

fn with_headers(url: Option<&str>, headers: Option<&[(&str, &str)]>) -> Draft {
    draft(SettingsDraft::Webhook(WebhookDraft {
        url: url.map(secret),
        method: WebhookMethod::Put,
        headers: headers.map(|list| {
            list.iter()
                .map(|(name, value)| ((*name).to_owned(), secret(value)))
                .collect()
        }),
    }))
}

fn parts(prepared: &Prepared) -> (&WebhookSettings, &WebhookSecrets) {
    match (&prepared.settings, &prepared.secrets) {
        (Settings::Webhook(settings), Some(Secrets::Webhook(secrets))) => (settings, secrets),
        _ => panic!("not a webhook"),
    }
}

/// A stored webhook at `hooks.example.test` with one header.
fn stored() -> (Settings, Kept<Secrets>) {
    let settings = WebhookSettings {
        origin: "https://hooks.example.test".to_owned(),
        method: WebhookMethod::Post,
        header_names: vec!["X-Api-Key".to_owned()],
    };
    let secrets = WebhookSecrets {
        url: secret(URL),
        headers: vec![secret("stored-header-value")],
    };
    (
        Settings::Webhook(settings),
        Kept::Opened(Secrets::Webhook(secrets)),
    )
}

fn changed(draft: Draft) -> Result<Prepared, Vec<FieldError>> {
    let (before, kept) = stored();
    prepare_change(&before, kept, draft)
}

fn values(secrets: &[Secret]) -> Vec<&str> {
    secrets.iter().map(super::text).collect()
}

#[test]
fn cr_14_6_a_webhook_url_must_be_https() {
    assert_eq!(
        new_errors(webhook("http://hooks.example.test/a")),
        ["settings.webhook.url"]
    );
    assert_eq!(
        new_errors(webhook("ftp://hooks.example.test/a")),
        ["settings.webhook.url"]
    );
    assert_eq!(
        new_errors(webhook("hooks.example.test/a")),
        ["settings.webhook.url"]
    );
    assert_eq!(new_errors(webhook("https://hooks.example.test/a")), NONE);
    assert_eq!(new_errors(webhook("https://192.0.2.10:8443/a")), NONE);
}

#[test]
fn cr_14_7_a_webhook_url_with_a_user_name_or_password_is_refused() {
    let url = "https://operator:pa55word@hooks.example.test/a";
    let errors = refused(prepare_new(draft(webhook(url))));
    assert_eq!(fields(&errors), ["settings.webhook.url"]);
    quotes_none(&errors, &["operator", "pa55word"]);
    assert_eq!(
        new_errors(webhook("https://operator@hooks.example.test/a")),
        ["settings.webhook.url"]
    );
}

#[test]
fn not_002_a_new_webhook_needs_its_url() {
    assert_eq!(
        fields(&refused(prepare_new(with_headers(None, None)))),
        ["settings.webhook.url"]
    );
    assert_eq!(new_errors(webhook("")), ["settings.webhook.url"]);
}

#[test]
fn sec_005_a_webhook_shows_its_origin_and_header_names_and_never_its_url_or_values() {
    let prepared = accepted(prepare_new(with_headers(
        Some(URL),
        Some(&[("X-Api-Key", "header-s3cr3t")]),
    )));
    let stored = prepared.settings.to_stored().to_string();
    assert!(stored.contains("https://hooks.example.test"));
    assert!(stored.contains("X-Api-Key"));
    for secret in ["s3cr3t-path", "s3cr3t-query", "header-s3cr3t"] {
        assert!(!stored.contains(secret), "settings hold a secret");
    }
    let (settings, secrets) = parts(&prepared);
    assert_eq!(settings.method, WebhookMethod::Put);
    assert_eq!(text(&secrets.url), URL);
    assert_eq!(values(&secrets.headers), ["header-s3cr3t"]);
    assert_eq!(prepared.touched.set, ["url", "headers"]);
}

#[test]
fn not_002_header_names_are_tokens_that_neither_frame_nor_redirect_the_request() {
    let named = |name: &str| errors_of(with_headers(Some(URL), Some(&[(name, "v")])));
    for refused in [
        "Host",
        "content-length",
        "Transfer-Encoding",
        "Connection",
        "Upgrade",
        "Proxy-Authorization",
        "X-Forwarded-For",
        "TE",
        "",
        "Bad Name",
        "X-Api-Key:",
        "Ünicode",
        &"x".repeat(129),
    ] {
        assert_eq!(named(refused), ["settings.webhook.headers"], "{refused:?}");
    }
    for allowed in [
        "X-Api-Key",
        "Authorization",
        "x-custom_header.v2",
        &"x".repeat(128),
    ] {
        assert_eq!(named(allowed), NONE, "{allowed:?}");
    }
}

#[test]
fn log_007_a_header_value_with_a_line_break_is_refused_without_quoting_it() {
    for value in ["a\r\nX-Injected: 1", "a\nX-Injected: 1", "a\0X-Injected"] {
        let errors = refused(prepare_new(with_headers(
            Some(URL),
            Some(&[("X-Key", value)]),
        )));
        assert_eq!(fields(&errors), ["settings.webhook.headers"]);
        quotes_none(&errors, &["X-Injected"]);
    }
    let tabbed = with_headers(Some(URL), Some(&[("X-Key", "a\tb")]));
    assert_eq!(errors_of(tabbed), NONE);
}

#[test]
fn not_002_headers_are_limited_in_count_size_and_repetition() {
    let many: Vec<(String, &str)> = (0..21).map(|i| (format!("X-H{i}"), "v")).collect();
    let many: Vec<(&str, &str)> = many.iter().map(|(n, v)| (n.as_str(), *v)).collect();
    assert_eq!(
        errors_of(with_headers(Some(URL), Some(&many))),
        ["settings.webhook.headers"]
    );
    assert_eq!(errors_of(with_headers(Some(URL), Some(&many[..20]))), NONE);
    let long = "v".repeat(4097);
    let big = with_headers(Some(URL), Some(&[("X-Key", &long)]));
    assert_eq!(errors_of(big), ["settings.webhook.headers"]);
    let twice = with_headers(Some(URL), Some(&[("X-Key", "a"), ("x-key", "b")]));
    assert_eq!(errors_of(twice), ["settings.webhook.headers"]);
}

#[test]
fn cr_14_1_a_null_url_and_headers_keep_the_stored_ones() {
    let prepared = accepted(changed(with_headers(None, None)));
    let (settings, secrets) = parts(&prepared);
    assert_eq!(text(&secrets.url), URL);
    assert_eq!(settings.header_names, ["X-Api-Key"]);
    assert_eq!(values(&secrets.headers), ["stored-header-value"]);
    assert_eq!(prepared.touched.set, NONE);
    assert_eq!(prepared.touched.cleared, NONE);
}

#[test]
fn cr_14_1_a_new_url_at_the_same_origin_keeps_the_headers() {
    let next = "https://hooks.example.test/other";
    let prepared = accepted(changed(with_headers(Some(next), None)));
    let (_, secrets) = parts(&prepared);
    assert_eq!(text(&secrets.url), next);
    assert_eq!(values(&secrets.headers), ["stored-header-value"]);
    assert_eq!(prepared.touched.set, ["url"]);
}

#[test]
fn cr_14_1_a_new_origin_needs_every_header_again_or_none() {
    let elsewhere = "https://elsewhere.example.test/a";
    let errors = refused(changed(with_headers(Some(elsewhere), None)));
    assert_eq!(fields(&errors), ["settings.webhook.headers"]);
    let cleared = accepted(changed(with_headers(Some(elsewhere), Some(&[]))));
    let (settings, secrets) = parts(&cleared);
    assert_eq!(settings.origin, "https://elsewhere.example.test");
    assert_eq!(settings.header_names, NONE);
    assert_eq!(values(&secrets.headers), NONE);
    assert_eq!(cleared.touched.cleared, ["headers"]);
    let again = accepted(changed(with_headers(
        Some(elsewhere),
        Some(&[("X-Api-Key", "new")]),
    )));
    assert_eq!(values(&parts(&again).1.headers), ["new"]);
}

#[test]
fn cr_14_3_headers_null_keeps_empty_removes_and_a_list_replaces() {
    let replaced = accepted(changed(with_headers(None, Some(&[("X-Other", "fresh")]))));
    let (settings, secrets) = parts(&replaced);
    assert_eq!(settings.header_names, ["X-Other"]);
    assert_eq!(values(&secrets.headers), ["fresh"]);
    assert_eq!(text(&secrets.url), URL);
    assert_eq!(replaced.touched.set, ["headers"]);
    let removed = accepted(changed(with_headers(None, Some(&[]))));
    assert_eq!(parts(&removed).0.header_names, NONE);
    assert_eq!(removed.touched.cleared, ["headers"]);
}

#[test]
fn cr_14_5_a_kept_secret_that_does_not_open_asks_for_every_secret_again() {
    let (before, _) = stored();
    let errors = refused(prepare_change(
        &before,
        Kept::Unopened,
        with_headers(None, None),
    ));
    assert_eq!(fields(&errors), ["settings"]);
    assert_eq!(
        errors[0].message,
        "The stored secrets cannot be opened. Enter every secret again."
    );
    let errors = refused(prepare_change(
        &before,
        Kept::Nothing,
        with_headers(None, Some(&[])),
    ));
    assert_eq!(fields(&errors), ["settings"]);
    let fresh = with_headers(Some(URL), Some(&[("X-Api-Key", "again")]));
    assert!(prepare_change(&before, Kept::Unopened, fresh).is_ok());
}
