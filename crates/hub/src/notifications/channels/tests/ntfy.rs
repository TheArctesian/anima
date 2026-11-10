//! The ntfy rules: an https server origin, a topic that cannot leave the URL path, a priority
//! from 1 to 5, and the CR-14 update rules for the topic and token.

use super::{
    Kept, NONE, NtfyDraft, Prepared, Secrets, Settings, SettingsDraft, accepted, draft, fields,
    new_errors, prepare_change, prepare_new, refused, secret, text,
};
use crate::{
    api_errors::FieldError,
    notifications::channels::{NtfySecrets, NtfySettings},
};

const SERVER: &str = "https://ntfy.example.test";

fn ntfy(server: &str, topic: Option<&str>, token: Option<&str>, priority: i32) -> SettingsDraft {
    SettingsDraft::Ntfy(NtfyDraft {
        server: server.to_owned(),
        topic: topic.map(secret),
        token: token.map(secret),
        priority,
    })
}

fn parts(prepared: &Prepared) -> (&NtfySettings, &NtfySecrets) {
    match (&prepared.settings, &prepared.secrets) {
        (Settings::Ntfy(settings), Some(Secrets::Ntfy(secrets))) => (settings, secrets),
        _ => panic!("not ntfy"),
    }
}

/// A stored ntfy channel on `SERVER` with a topic and a token.
fn changed(next: SettingsDraft) -> Result<Prepared, Vec<FieldError>> {
    let before = Settings::Ntfy(NtfySettings {
        server: SERVER.to_owned(),
        priority: 3,
        token_stored: true,
    });
    let stored = Kept::Opened(Secrets::Ntfy(NtfySecrets {
        topic: secret("stored-topic"),
        token: Some(secret("tk_stored")),
    }));
    prepare_change(&before, stored, draft(next))
}

#[test]
fn cr_14_7_an_ntfy_server_is_an_https_origin_only() {
    for server in [
        "http://ntfy.example.test",
        "https://ntfy.example.test/alerts",
        "https://ntfy.example.test/?x=1",
        "https://ntfy.example.test/#frag",
        "https://ntfy.example.test#frag",
        "https://user:pw@ntfy.example.test",
        "https://:pw@ntfy.example.test",
        "ntfy.example.test",
        "https://ntfy.exa\tmple.test",
        "https://ntfy.exa\nmple.test",
        "https://ntfy.example.test\r\n",
    ] {
        let topic = Some("alerts");
        assert_eq!(
            new_errors(ntfy(server, topic, None, 3)),
            ["settings.ntfy.server"],
            "{server:?}"
        );
    }
    let prepared = accepted(prepare_new(draft(ntfy(
        "https://NTFY.example.test:8443/",
        Some("a"),
        None,
        3,
    ))));
    assert_eq!(parts(&prepared).0.server, "https://ntfy.example.test:8443");
}

#[test]
fn not_002_an_ntfy_topic_stays_in_the_url_path() {
    for topic in [
        "",
        "../admin",
        "a/b",
        "a b",
        "a?b",
        "a%2fb",
        &"t".repeat(65),
    ] {
        let errors = new_errors(ntfy(SERVER, Some(topic), None, 3));
        assert_eq!(errors, ["settings.ntfy.topic"], "{topic:?}");
    }
    assert_eq!(
        new_errors(ntfy(SERVER, None, None, 3)),
        ["settings.ntfy.topic"]
    );
    assert_eq!(
        new_errors(ntfy(SERVER, Some("Alerts_home-1"), None, 3)),
        NONE
    );
    assert_eq!(
        new_errors(ntfy(SERVER, Some(&"t".repeat(64)), None, 3)),
        NONE
    );
}

#[test]
fn not_002_an_ntfy_priority_is_from_1_to_5() {
    for priority in [0, 6, -1, 256] {
        assert_eq!(
            new_errors(ntfy(SERVER, Some("a"), None, priority)),
            ["settings.ntfy.priority"]
        );
    }
    for priority in 1..=5 {
        assert_eq!(new_errors(ntfy(SERVER, Some("a"), None, priority)), NONE);
    }
}

#[test]
fn not_002_an_ntfy_token_is_optional_and_visible_ascii() {
    let none = accepted(prepare_new(draft(ntfy(SERVER, Some("a"), Some(""), 3))));
    assert!(!parts(&none).0.token_stored);
    assert_eq!(none.touched.set, ["topic"]);
    for token in ["tk with space", "tk\r\nX: 1", &"t".repeat(257)] {
        let errors = refused(prepare_new(draft(ntfy(SERVER, Some("a"), Some(token), 3))));
        assert_eq!(fields(&errors), ["settings.ntfy.token"]);
    }
    let some = accepted(prepare_new(draft(ntfy(
        SERVER,
        Some("a"),
        Some("tk_123"),
        3,
    ))));
    assert!(parts(&some).0.token_stored);
    assert_eq!(some.touched.set, ["topic", "token"]);
}

#[test]
fn cr_14_1_null_ntfy_secrets_on_the_same_server_keep_the_stored_ones() {
    let prepared = accepted(changed(ntfy(SERVER, None, None, 5)));
    let (settings, secrets) = parts(&prepared);
    assert_eq!(settings.priority, 5);
    assert_eq!(text(&secrets.topic), "stored-topic");
    assert_eq!(secrets.token.as_ref().map(text), Some("tk_stored"));
    assert_eq!(prepared.touched.set, NONE);
}

#[test]
fn not_002_an_empty_ntfy_token_clears_it() {
    let prepared = accepted(changed(ntfy(SERVER, None, Some(""), 3)));
    let (settings, secrets) = parts(&prepared);
    assert!(!settings.token_stored && secrets.token.is_none());
    assert_eq!(text(&secrets.topic), "stored-topic");
    assert_eq!(prepared.touched.cleared, ["token"]);
}

#[test]
fn cr_14_1_a_new_ntfy_server_needs_the_topic_and_token_again() {
    let other = "https://other.example.test";
    let errors = refused(changed(ntfy(other, None, None, 3)));
    assert_eq!(
        fields(&errors),
        ["settings.ntfy.topic", "settings.ntfy.token"]
    );
    let errors = refused(changed(ntfy(other, Some("t"), None, 3)));
    assert_eq!(fields(&errors), ["settings.ntfy.token"]);
    let cleared = accepted(changed(ntfy(other, Some("t"), Some(""), 3)));
    assert!(!parts(&cleared).0.token_stored);
    let again = accepted(changed(ntfy(other, Some("t"), Some("tk_new"), 3)));
    assert_eq!(parts(&again).1.token.as_ref().map(text), Some("tk_new"));
}
