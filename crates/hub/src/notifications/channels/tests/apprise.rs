//! The Apprise rules: an https server origin, the secret of the mode, tags Apprise cannot read
//! as an expression, and the CR-14 update rules for a mode or server change.

use super::{
    Kept, NONE, Prepared, Secrets, Settings, SettingsDraft, accepted, draft, fields, new_errors,
    prepare_change, refused, secret, text,
};
use crate::{
    api_errors::FieldError,
    notifications::channels::{AppriseDraft, AppriseMode, AppriseSecrets, AppriseSettings},
};

const SERVER: &str = "https://apprise.example.test";
const DISCORD: &str = "discord://1234/s3cr3t-webhook-token";

fn apprise(mode: AppriseMode, urls: Option<&[&str]>, key: Option<&str>) -> SettingsDraft {
    SettingsDraft::Apprise(AppriseDraft {
        server: SERVER.to_owned(),
        mode,
        urls: urls.map(|urls| urls.iter().copied().map(secret).collect()),
        key: key.map(secret),
        tags: vec!["ops".to_owned()],
    })
}

fn stateless(urls: &[&str]) -> SettingsDraft {
    apprise(AppriseMode::StatelessUrls, Some(urls), None)
}

fn secret_of(prepared: &Prepared) -> &AppriseSecrets {
    match (&prepared.settings, &prepared.secrets) {
        (Settings::Apprise(_), Some(Secrets::Apprise(secret))) => secret,
        _ => panic!("not Apprise"),
    }
}

fn urls(secret: &AppriseSecrets) -> Vec<&str> {
    match secret {
        AppriseSecrets::Urls(urls) => urls.iter().map(text).collect(),
        AppriseSecrets::Key(_) => panic!("a key"),
    }
}

/// A stored stateless channel with one Discord URL.
fn changed(next: SettingsDraft) -> Result<Prepared, Vec<FieldError>> {
    let before = Settings::Apprise(AppriseSettings {
        server: SERVER.to_owned(),
        mode: AppriseMode::StatelessUrls,
        tags: Vec::new(),
    });
    let stored = Kept::Opened(Secrets::Apprise(AppriseSecrets::Urls(vec![secret(
        DISCORD,
    )])));
    prepare_change(&before, stored, draft(next))
}

#[test]
fn cr_14_7_an_apprise_server_is_an_https_origin_only() {
    for server in [
        "http://apprise.example.test",
        "https://apprise.example.test/notify/",
        "https://apprise.example.test/?x=1",
        "https://apprise.example.test?x=1",
        "https://apprise.example.test/#frag",
        "https://apprise.example.test#frag",
        "https://user:pw@apprise.example.test",
        "https://user@apprise.example.test",
        "https://appr\tise.example.test",
        "https://appr\nise.example.test",
        "https://apprise.example.test\r\n",
    ] {
        let SettingsDraft::Apprise(mut input) = stateless(&[DISCORD]) else {
            unreachable!()
        };
        input.server = server.to_owned();
        assert_eq!(
            new_errors(SettingsDraft::Apprise(input)),
            ["settings.apprise.server"],
            "{server:?}"
        );
    }
}

#[test]
fn n_4_a_stateless_channel_needs_permitted_urls_and_no_key() {
    assert_eq!(
        new_errors(apprise(AppriseMode::StatelessUrls, None, None)),
        ["settings.apprise.urls"]
    );
    assert_eq!(new_errors(stateless(&[])), ["settings.apprise.urls"]);
    let errors = refused(super::prepare_new(draft(stateless(&[
        "json://evil.example.test/s3cr3t",
    ]))));
    assert_eq!(fields(&errors), ["settings.apprise.urls"]);
    super::quotes_none(&errors, &["evil.example.test", "s3cr3t"]);
    let both = apprise(AppriseMode::StatelessUrls, Some(&[DISCORD]), Some("key"));
    assert_eq!(new_errors(both), ["settings.apprise.key"]);
    let empty_key = apprise(AppriseMode::StatelessUrls, Some(&[DISCORD]), Some(""));
    assert_eq!(new_errors(empty_key), NONE);
    let many: Vec<&str> = std::iter::repeat_n(DISCORD, 21).collect();
    assert_eq!(new_errors(stateless(&many)), ["settings.apprise.urls"]);
}

#[test]
fn n_4_a_config_key_channel_needs_a_key_that_stays_in_the_path() {
    let keyed = |key: Option<&str>| new_errors(apprise(AppriseMode::ConfigKey, None, key));
    assert_eq!(keyed(None), ["settings.apprise.key"]);
    for key in ["", "../admin", "a/b", "a.b", "a b", &"k".repeat(129)] {
        assert_eq!(keyed(Some(key)), ["settings.apprise.key"], "{key:?}");
    }
    assert_eq!(keyed(Some("home_alerts-1")), NONE);
    let both = apprise(AppriseMode::ConfigKey, Some(&[DISCORD]), Some("home"));
    assert_eq!(new_errors(both), ["settings.apprise.urls"]);
}

#[test]
fn not_002_apprise_tags_cannot_form_an_expression() {
    let tagged = |tags: Vec<String>| {
        let SettingsDraft::Apprise(mut input) = stateless(&[DISCORD]) else {
            unreachable!()
        };
        input.tags = tags;
        new_errors(SettingsDraft::Apprise(input))
    };
    for tag in ["a,b", "a b", "", "a|b", &"t".repeat(65)] {
        assert_eq!(
            tagged(vec![tag.to_owned()]),
            ["settings.apprise.tags"],
            "{tag:?}"
        );
    }
    assert_eq!(
        tagged((0..21).map(|i| format!("t{i}")).collect()),
        ["settings.apprise.tags"]
    );
    assert_eq!(
        tagged(vec!["home.ops".to_owned(), "a_b-c".to_owned()]),
        NONE
    );
}

#[test]
fn cr_14_1_a_null_apprise_secret_on_the_same_server_and_mode_keeps_the_stored_one() {
    let prepared = accepted(changed(apprise(AppriseMode::StatelessUrls, None, None)));
    assert_eq!(urls(secret_of(&prepared)), [DISCORD]);
    assert_eq!(prepared.touched.set, NONE);
    assert_eq!(prepared.touched.cleared, NONE);
}

#[test]
fn cr_14_2_a_mode_change_drops_the_old_secret_and_needs_the_new_one() {
    let errors = refused(changed(apprise(AppriseMode::ConfigKey, None, None)));
    assert_eq!(fields(&errors), ["settings.apprise.key"]);
    assert_eq!(
        errors[0].message,
        "The mode changed: enter the configuration key."
    );
    let switched = accepted(changed(apprise(AppriseMode::ConfigKey, None, Some("home"))));
    match secret_of(&switched) {
        AppriseSecrets::Key(key) => assert_eq!(text(key), "home"),
        AppriseSecrets::Urls(_) => panic!("urls kept"),
    }
    assert_eq!(switched.touched.cleared, ["urls"]);
    assert_eq!(switched.touched.set, ["key"]);
}

#[test]
fn cr_14_1_a_new_apprise_server_needs_the_secret_again() {
    let SettingsDraft::Apprise(mut input) = apprise(AppriseMode::StatelessUrls, None, None) else {
        unreachable!()
    };
    input.server = "https://other.example.test".to_owned();
    let errors = refused(changed(SettingsDraft::Apprise(input)));
    assert_eq!(fields(&errors), ["settings.apprise.urls"]);
    let replaced = accepted(changed(stateless(&["tgram://bot/123"])));
    assert_eq!(urls(secret_of(&replaced)), ["tgram://bot/123"]);
    assert_eq!(replaced.touched.set, ["urls"]);
}

#[test]
fn sec_003_a_kept_apprise_secret_that_does_not_open_asks_for_it_again() {
    let before = Settings::Apprise(AppriseSettings {
        server: SERVER.to_owned(),
        mode: AppriseMode::StatelessUrls,
        tags: Vec::new(),
    });
    let next = draft(apprise(AppriseMode::StatelessUrls, None, None));
    let errors = refused(prepare_change(&before, Kept::Unopened, next));
    assert_eq!(fields(&errors), ["settings"]);
}
