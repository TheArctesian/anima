//! The channel rules, by kind (NOT-002, NOT-003, SEC-005, CR-14, N-4). Every value here is
//! synthetic.

mod apprise;
mod apprise_url;
mod email;
mod ntfy;
mod webhook;

use super::{
    Draft, Kept, NtfyDraft, Prepared, Secrets, Settings, SettingsDraft, Template, WebhookDraft,
    WebhookMethod, WebhookSecrets, WebhookSettings, prepare_change, prepare_new,
};
use crate::{api_errors::FieldError, credentials::Secret};

/// No field refused, no secret field touched: what an empty list is compared with.
const NONE: [&str; 0] = [];

fn secret(text: &str) -> Secret {
    Secret::from(text.to_owned())
}

fn text(secret: &Secret) -> &str {
    std::str::from_utf8(secret.expose()).unwrap()
}

fn draft(settings: SettingsDraft) -> Draft {
    Draft {
        name: "Ops alerts".to_owned(),
        on_by_default: false,
        settings,
        template: None,
    }
}

/// The prepared channel; a refusal fails the test with the fields it names.
fn accepted(result: Result<Prepared, Vec<FieldError>>) -> Prepared {
    match result {
        Ok(prepared) => prepared,
        Err(errors) => panic!("refused: {:?}", fields(&errors)),
    }
}

fn refused(result: Result<Prepared, Vec<FieldError>>) -> Vec<FieldError> {
    match result {
        Ok(_) => panic!("accepted"),
        Err(errors) => errors,
    }
}

fn fields(errors: &[FieldError]) -> Vec<&'static str> {
    errors.iter().map(|error| error.field).collect()
}

/// The fields a new channel is refused on; empty when accepted.
fn errors_of(draft: Draft) -> Vec<&'static str> {
    prepare_new(draft)
        .err()
        .map_or_else(Vec::new, |errors| fields(&errors))
}

/// The fields a new channel with these settings is refused on; empty when accepted.
fn new_errors(settings: SettingsDraft) -> Vec<&'static str> {
    errors_of(draft(settings))
}

/// No refusal quotes any of `values` (LOG-007).
fn quotes_none(errors: &[FieldError], values: &[&str]) {
    for error in errors {
        for value in values {
            assert!(
                !error.message.contains(value),
                "{} quotes input",
                error.field
            );
        }
    }
}

fn webhook(url: &str) -> SettingsDraft {
    SettingsDraft::Webhook(WebhookDraft {
        url: Some(secret(url)),
        method: WebhookMethod::Post,
        headers: None,
    })
}

#[test]
fn not_002_the_name_is_required_plain_and_at_most_100_characters() {
    let named = |name: &str| {
        let mut draft = draft(webhook("https://hooks.example.test/a"));
        draft.name = name.to_owned();
        errors_of(draft)
    };
    assert_eq!(named(" "), ["name"]);
    assert_eq!(named(&"n".repeat(101)), ["name"]);
    assert_eq!(named("Ops\nalerts"), ["name"]);
    assert_eq!(named(&"n".repeat(100)), NONE);
}

#[test]
fn not_002_a_template_title_is_at_most_500_and_a_body_at_most_4000_characters() {
    let templated = |title: usize, body: usize| {
        let mut draft = draft(webhook("https://hooks.example.test/a"));
        draft.template = Some(Template {
            title: "t".repeat(title),
            body: "b".repeat(body),
        });
        errors_of(draft)
    };
    assert_eq!(templated(500, 4000), NONE);
    assert_eq!(templated(501, 4001), ["template.title", "template.body"]);
}

#[test]
fn not_002_common_and_kind_errors_come_together() {
    let mut draft = draft(webhook("http://hooks.example.test/a"));
    draft.name = String::new();
    assert_eq!(
        fields(&refused(prepare_new(draft))),
        ["name", "settings.webhook.url"]
    );
}

#[test]
fn cr_14_4_a_channel_kind_never_changes() {
    let before = Settings::Webhook(WebhookSettings {
        origin: "https://hooks.example.test".to_owned(),
        method: WebhookMethod::Post,
        header_names: Vec::new(),
    });
    let stored = Kept::Opened(Secrets::Webhook(WebhookSecrets {
        url: secret("https://hooks.example.test/a"),
        headers: Vec::new(),
    }));
    let ntfy = SettingsDraft::Ntfy(NtfyDraft {
        server: "https://ntfy.example.test".to_owned(),
        topic: Some(secret("alerts")),
        token: None,
        priority: 3,
    });
    let errors = refused(prepare_change(&before, stored, draft(ntfy)));
    assert_eq!(fields(&errors), ["settings"]);
    assert_eq!(
        errors[0].message,
        "A channel's kind cannot change. Create a new channel."
    );
}

#[test]
fn sec_005_settings_round_trip_through_their_stored_form() {
    let prepared = accepted(prepare_new(draft(webhook(
        "https://hooks.example.test/a?k=1",
    ))));
    let stored = prepared.settings.to_stored();
    let read = Settings::from_stored(prepared.settings.kind(), &stored).unwrap();
    assert_eq!(read, prepared.settings);
    let other = Settings::from_stored(crate::notifications::ChannelKind::Ntfy, &stored);
    assert!(other.is_err());
}
