//! The e-mail rules: a host and port, mailboxes without line breaks, at least one recipient,
//! and the CR-14 update rules for the password.

use std::str::FromStr as _;

use super::{
    Kept, NONE, Prepared, Secrets, Settings, SettingsDraft, accepted, draft, fields, new_errors,
    prepare_change, prepare_new, quotes_none, refused, secret, text,
};
use crate::{
    api_errors::FieldError,
    fleet::notifications::{
        channels::{EmailDraft, EmailSecrets, EmailSettings},
        egress::SmtpSecurity,
    },
};

/// A valid draft; `edit` changes it before it is judged.
fn email(edit: impl FnOnce(&mut EmailDraft)) -> SettingsDraft {
    let mut draft = EmailDraft {
        host: "smtp.example.test".to_owned(),
        port: 587,
        security: SmtpSecurity::StartTls,
        username: Some("hub".to_owned()),
        password: Some(secret("smtp-s3cr3t")),
        from: "Anima <hub@example.test>".to_owned(),
        to: vec!["ops@example.test".to_owned()],
    };
    edit(&mut draft);
    SettingsDraft::Email(draft)
}

fn parts(prepared: &Prepared) -> (&EmailSettings, Option<&EmailSecrets>) {
    match (&prepared.settings, &prepared.secrets) {
        (Settings::Email(settings), Some(Secrets::Email(secrets))) => (settings, Some(secrets)),
        (Settings::Email(settings), None) => (settings, None),
        _ => panic!("not e-mail"),
    }
}

/// A stored e-mail channel on `smtp.example.test:587` with a password.
fn changed(next: SettingsDraft) -> Result<Prepared, Vec<FieldError>> {
    let before = Settings::Email(EmailSettings {
        host: "smtp.example.test".to_owned(),
        port: 587,
        security: SmtpSecurity::StartTls,
        username: Some("hub".to_owned()),
        from: "hub@example.test".to_owned(),
        to: vec!["ops@example.test".to_owned()],
        password_stored: true,
    });
    let stored = Kept::Opened(Secrets::Email(EmailSecrets {
        password: secret("stored-password"),
    }));
    prepare_change(&before, stored, draft(next))
}

#[test]
fn not_002_an_smtp_port_is_from_1_to_65535() {
    for port in [0, -1, 65536] {
        assert_eq!(
            new_errors(email(|d| d.port = port)),
            ["settings.email.port"]
        );
    }
    for port in [1, 25, 465, 65535] {
        assert_eq!(new_errors(email(|d| d.port = port)), NONE);
    }
}

#[test]
fn not_002_an_smtp_host_is_a_name_or_address_and_never_a_url() {
    for host in [
        "",
        "smtp://smtp.example.test",
        "smtp.example.test:25",
        "-bad.example",
        "a b",
    ] {
        assert_eq!(
            new_errors(email(|d| d.host = host.to_owned())),
            ["settings.email.host"]
        );
    }
    for host in ["smtp.example.test", "192.0.2.25", "2001:db8::25", "mail"] {
        assert_eq!(
            new_errors(email(|d| d.host = host.to_owned())),
            NONE,
            "{host}"
        );
    }
}

#[test]
fn log_007_an_address_with_a_line_break_is_refused_without_quoting_it() {
    let injected = "hub@example.test\r\nBcc: spy@example.test";
    let errors = refused(prepare_new(draft(email(|d| d.from = injected.to_owned()))));
    assert_eq!(fields(&errors), ["settings.email.from"]);
    quotes_none(&errors, &["spy@example.test"]);
    let errors = refused(prepare_new(draft(email(|d| {
        d.to = vec![injected.to_owned()];
    }))));
    assert_eq!(fields(&errors), ["settings.email.to"]);
    // lettre reads a mailbox with a trailing line break as valid, so only the hub's own
    // control-character check keeps it out of the message headers.
    let trailing = "ops@example.test\r\n";
    assert!(lettre::message::Mailbox::from_str(trailing).is_ok());
    assert_eq!(
        new_errors(email(|d| d.from = trailing.to_owned())),
        ["settings.email.from"]
    );
    assert_eq!(
        new_errors(email(|d| d.to = vec![trailing.to_owned()])),
        ["settings.email.to"]
    );
    let folded = "Anima\r\n <hub@example.test>";
    assert_eq!(
        new_errors(email(|d| d.from = folded.to_owned())),
        ["settings.email.from"]
    );
}

#[test]
fn not_002_every_address_is_one_mailbox() {
    for bad in ["", "not an address", "a@example.test, b@example.test", "<>"] {
        assert_eq!(
            new_errors(email(|d| d.from = bad.to_owned())),
            ["settings.email.from"]
        );
    }
    let named = "Ops Team <ops@example.test>";
    assert_eq!(new_errors(email(|d| d.to = vec![named.to_owned()])), NONE);
}

#[test]
fn not_002_an_email_channel_has_1_to_20_recipients() {
    assert_eq!(
        new_errors(email(|d| d.to = Vec::new())),
        ["settings.email.to"]
    );
    let many = |n: usize| {
        (0..n)
            .map(|i| format!("ops{i}@example.test"))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        new_errors(email(|d| d.to = many(21))),
        ["settings.email.to"]
    );
    assert_eq!(new_errors(email(|d| d.to = many(20))), NONE);
}

#[test]
fn not_002_a_password_needs_a_user_name_and_has_no_control_characters() {
    assert_eq!(
        new_errors(email(|d| d.username = None)),
        ["settings.email.username"]
    );
    assert_eq!(
        new_errors(email(|d| d.username = Some(String::new()))),
        ["settings.email.username"]
    );
    let nul = email(|d| d.password = Some(secret("a\0b")));
    assert_eq!(new_errors(nul), ["settings.email.password"]);
    let open = accepted(prepare_new(draft(email(|d| {
        d.username = None;
        d.password = Some(secret(""));
    }))));
    assert!(parts(&open).1.is_none() && !parts(&open).0.password_stored);
    assert_eq!(open.touched.set, NONE);
}

#[test]
fn sec_005_the_email_settings_name_the_security_and_never_the_password() {
    let prepared = accepted(prepare_new(draft(email(|d| {
        d.security = SmtpSecurity::Implicit;
    }))));
    let stored = prepared.settings.to_stored();
    assert_eq!(stored["security"], "tls");
    assert!(!stored.to_string().contains("smtp-s3cr3t"));
    assert_eq!(
        parts(&prepared).1.map(|s| text(&s.password)),
        Some("smtp-s3cr3t")
    );
    assert_eq!(prepared.touched.set, ["password"]);
    let starttls = accepted(prepare_new(draft(email(|_| {}))));
    assert_eq!(starttls.settings.to_stored()["security"], "starttls");
}

#[test]
fn cr_14_1_a_null_password_on_the_same_server_keeps_the_stored_one() {
    let prepared = accepted(changed(email(|d| {
        d.password = None;
        d.to = vec!["new@example.test".to_owned()];
    })));
    assert_eq!(
        parts(&prepared).1.map(|s| text(&s.password)),
        Some("stored-password")
    );
    assert_eq!(prepared.touched.set, NONE);
}

#[test]
fn cr_14_1_a_new_host_or_port_needs_the_password_again_or_empty() {
    let moved = |host: &str, port: i32, password: Option<&str>| {
        changed(email(|d| {
            d.host = host.to_owned();
            d.port = port;
            d.password = password.map(secret);
        }))
    };
    let errors = refused(moved("other.example.test", 587, None));
    assert_eq!(fields(&errors), ["settings.email.password"]);
    let errors = refused(moved("smtp.example.test", 465, None));
    assert_eq!(fields(&errors), ["settings.email.password"]);
    let cleared = accepted(moved("other.example.test", 587, Some("")));
    assert!(!parts(&cleared).0.password_stored);
    assert_eq!(cleared.touched.cleared, ["password"]);
    let again = accepted(moved("other.example.test", 587, Some("fresh")));
    assert_eq!(parts(&again).1.map(|s| text(&s.password)), Some("fresh"));
}
