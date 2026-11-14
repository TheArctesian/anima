use serde_json::{Value, json};
use source_exchange::{Installation, ManifestFacts, Owner, Request, Webhook};

use super::REGISTRATION;
use crate::tests::fixture;

fn facts(webhook: Webhook) -> ManifestFacts<'static> {
    ManifestFacts {
        name: "anima-k3f9x2mq7b",
        origin: "https://hub.example.test",
        webhook,
    }
}

fn expected(hook: Option<Value>) -> Value {
    let mut body = json!({
        "name": "anima-k3f9x2mq7b",
        "url": "https://hub.example.test",
        "description": "Made by Anima for one hub. Anima reads and changes the repositories of this account.",
        "public": false,
        "redirect_url": "https://hub.example.test/connect/github/created",
        "setup_url": "https://hub.example.test/connect/github/installed",
        "setup_on_update": false,
        "request_oauth_on_install": false,
        "default_permissions": {
            "metadata": "read",
            "contents": "write", "administration": "write", "actions": "write",
            "actions_variables": "write", "checks": "write", "deployments": "write",
            "environments": "write", "issues": "write", "pull_requests": "write",
            "statuses": "write", "workflows": "write", "secrets": "write",
            "repository_hooks": "write", "pages": "write", "discussions": "write",
            "repository_projects": "write", "security_events": "write",
            "vulnerability_alerts": "write", "dependabot_secrets": "write",
            "merge_queues": "write", "packages": "write", "secret_scanning_alerts": "write",
            "repository_custom_properties": "write"
        }
    });
    if let Some(hook) = hook {
        body["hook_attributes"] = hook;
    }
    body
}

fn manifest(webhook: Webhook) -> Value {
    serde_json::from_str(&(REGISTRATION.manifest)(&facts(webhook))).unwrap()
}

#[test]
fn prj_039_the_manifest_with_the_webhook_inactive_at_the_hub_is_exact() {
    assert_eq!(
        manifest(Webhook::InactiveAtHub),
        expected(Some(json!({
            "url": "https://hub.example.test/connect/github/hook",
            "active": false
        })))
    );
}

#[test]
fn prj_039_the_manifest_without_a_webhook_has_no_hook_key() {
    assert_eq!(manifest(Webhook::Omitted), expected(None));
}

#[test]
fn prj_039_the_manifest_with_a_placeholder_webhook_is_exact() {
    assert_eq!(
        manifest(Webhook::InactivePlaceholder),
        expected(Some(json!({
            "url": "https://example.com/anima-webhook-off",
            "active": false
        })))
    );
}

#[test]
fn prj_039_the_manifest_asks_for_no_events_and_no_sign_in_callbacks() {
    let body = manifest(Webhook::InactiveAtHub);
    assert!(body.get("default_events").is_none());
    assert!(body.get("callback_urls").is_none());
    assert_eq!(body["default_permissions"].as_object().unwrap().len(), 24);
}

#[test]
fn prj_039_the_name_is_sixteen_characters_from_the_alphabet() {
    let name = (REGISTRATION.name)(&[0xFF, 0x00, 0xA5, 0x5A, 0x12, 0x34, 0x56]);
    let tail = name.strip_prefix("anima-").unwrap();
    assert_eq!(name.len(), 16);
    assert_eq!(tail.len(), 10);
    assert!(
        tail.bytes()
            .all(|b| b.is_ascii_lowercase() || (b'2'..=b'7').contains(&b))
    );
}

#[test]
fn prj_039_the_name_differs_for_different_entropy() {
    let a = (REGISTRATION.name)(&[1, 2, 3, 4, 5, 6, 7]);
    let b = (REGISTRATION.name)(&[2, 2, 3, 4, 5, 6, 7]);
    let c = (REGISTRATION.name)(&[1, 2, 3, 4, 5, 6, 0x47]);
    assert_ne!(a, b);
    assert_ne!(a, c);
    assert_eq!((REGISTRATION.name)(&[0; 7]), "anima-aaaaaaaaaa");
}

#[test]
fn prj_039_the_form_path_is_personal_or_for_a_valid_organisation() {
    assert_eq!(
        (REGISTRATION.form_path)(None).unwrap(),
        "/settings/apps/new"
    );
    assert_eq!(
        (REGISTRATION.form_path)(Some("octo-org")).unwrap(),
        "/organizations/octo-org/settings/apps/new"
    );
}

#[test]
fn prj_039_the_form_path_refuses_an_organisation_that_is_not_a_login() {
    let long = "a".repeat(40);
    for bad in [
        "",
        "-lead",
        "trail-",
        "dou--ble",
        "a/b",
        "..",
        "a b",
        "a@b",
        "a?b",
        "a#b",
        "über",
        long.as_str(),
    ] {
        assert!((REGISTRATION.form_path)(Some(bad)).is_none(), "{bad:?}");
    }
}

#[test]
fn prj_039_the_conversion_reply_gives_the_app_and_neither_secret() {
    let converted = (REGISTRATION.parse_converted)(&fixture("manifest_converted.json"))
        .ok()
        .unwrap();

    assert_eq!(converted.app_id, 1_234_567);
    assert_eq!(converted.slug, "anima-k3f9x2mq7b");
    assert_eq!(
        converted.owner,
        Owner {
            id: 9001,
            login: "octo-person".to_owned(),
            organisation: false
        }
    );
    assert!(converted.private_key_pem.contains("PEM-MARKER-2222"));
    // `Converted` has no `Debug`, so the key cannot be printed with `{:?}`. The only fields that
    // hold text are the slug, the owner's login and the key; neither secret is among them.
    for text in [&converted.slug, &converted.owner.login] {
        assert!(!text.contains("SECRET-MARKER"));
    }
    assert!(!converted.private_key_pem.contains("SECRET-MARKER"));
}

#[test]
fn prj_039_an_organisation_owner_is_marked() {
    let mut body: Value = serde_json::from_slice(&fixture("manifest_converted.json")).unwrap();
    body["owner"] = json!({ "login": "octo-org", "id": 9002, "type": "Organization" });
    let converted = (REGISTRATION.parse_converted)(body.to_string().as_bytes())
        .ok()
        .unwrap();
    assert!(converted.owner.organisation);
}

#[test]
fn prj_039_an_unreadable_conversion_never_quotes_the_reply() {
    let body = br#"{"id":"CONVERT-MARKER","slug":"x","owner":null,"pem":"PEM-MARKER"}"#;
    let Err(problem) = (REGISTRATION.parse_converted)(body) else {
        panic!("the reply is not valid");
    };
    let all = format!("{} {} {}", problem.code, problem.subject, problem.message);
    assert!(!all.contains("MARKER"));
    assert!(!all.contains('{'));
}

#[test]
fn prj_039_a_reply_with_a_hostile_slug_or_login_or_no_key_is_unusable() {
    let mut base: Value = serde_json::from_slice(&fixture("manifest_converted.json")).unwrap();
    base["slug"] = json!("../evil");
    assert!((REGISTRATION.parse_converted)(base.to_string().as_bytes()).is_err());
    base["slug"] = json!("fine");
    base["owner"]["login"] = json!("a/b");
    assert!((REGISTRATION.parse_converted)(base.to_string().as_bytes()).is_err());
    base["owner"]["login"] = json!("ok");
    base["pem"] = json!("");
    assert!((REGISTRATION.parse_converted)(base.to_string().as_bytes()).is_err());
}

#[test]
fn prj_039_an_installation_is_read() {
    let one = (REGISTRATION.parse_installation)(&fixture("installation.json"))
        .ok()
        .unwrap();
    assert_eq!(
        one,
        Installation {
            id: 4321,
            app_id: 1_234_567,
            account_id: 9001,
            suspended: false
        }
    );
}

#[test]
fn prj_039_a_suspended_installation_is_marked() {
    let one = (REGISTRATION.parse_installation)(&fixture("installation_suspended.json"))
        .ok()
        .unwrap();
    assert!(one.suspended);
    assert_eq!(one.id, 4322);
}

#[test]
fn prj_039_a_list_of_installations_is_read_with_its_suspended_one() {
    let all = (REGISTRATION.parse_installations)(&fixture("installations.json"))
        .ok()
        .unwrap();
    assert_eq!(all.len(), 2);
    assert!(!all[0].suspended);
    assert_eq!((all[1].account_id, all[1].suspended), (9002, true));
}

#[test]
fn prj_039_an_unreadable_installation_reply_is_a_fixed_problem() {
    let Err(problem) = (REGISTRATION.parse_installation)(br#"{"id":"INSTALL-MARKER"}"#) else {
        panic!("the reply is not valid");
    };
    assert!(!problem.message.contains("MARKER"));
    assert!((REGISTRATION.parse_installations)(b"{}").is_err());
}

#[test]
fn prj_039_the_pages_for_a_plain_slug_and_owner() {
    assert_eq!(
        (REGISTRATION.install_path)("anima-k3f9x2mq7b"),
        "/apps/anima-k3f9x2mq7b/installations/new"
    );
    let person = Owner {
        id: 1,
        login: "octo-person".to_owned(),
        organisation: false,
    };
    let org = Owner {
        id: 2,
        login: "octo-org".to_owned(),
        organisation: true,
    };
    assert_eq!(
        (REGISTRATION.settings_path)(Some(&person), Some("s-1")),
        "/settings/apps/s-1/advanced"
    );
    assert_eq!(
        (REGISTRATION.settings_path)(Some(&org), Some("s-1")),
        "/organizations/octo-org/settings/apps/s-1/advanced"
    );
    assert_eq!((REGISTRATION.settings_path)(None, None), "/settings/apps");
    assert_eq!(
        (REGISTRATION.settings_path)(Some(&org), None),
        "/organizations/octo-org/settings/apps"
    );
}

const HOSTILE: [&str; 14] = [
    "..",
    ".",
    "../x",
    "a/../b",
    "a//b",
    "a@evil.test",
    "a?x=1",
    "a#frag",
    "https://evil.test",
    "javascript:alert(1)",
    "%2e%2e",
    "a b",
    "",
    "\\evil",
];

fn is_safe(path: &str) -> bool {
    path.starts_with('/')
        && !path.starts_with("//")
        && !path.contains("..")
        && !path.contains("//")
        && !path.contains('@')
        && !path.contains('?')
        && !path.contains('#')
        && !path.contains(':')
        && !path.contains('\\')
        && !path.to_ascii_lowercase().contains("%2e")
}

#[test]
fn prj_039_a_hostile_slug_or_login_never_builds_a_path_that_leaves_the_page() {
    for bad in HOSTILE {
        let owner = Owner {
            id: 1,
            login: bad.to_owned(),
            organisation: true,
        };
        for path in [
            (REGISTRATION.install_path)(bad),
            (REGISTRATION.settings_path)(Some(&owner), Some(bad)),
            (REGISTRATION.settings_path)(None, Some(bad)),
        ] {
            assert!(is_safe(&path), "{bad:?} gave {path:?}");
        }
    }
}

#[test]
fn prj_039_a_hostile_code_or_id_never_builds_a_path_the_request_guards_refuse() {
    let long = "a".repeat(201);
    for bad in HOSTILE.iter().copied().chain([long.as_str()]) {
        for path in [
            (REGISTRATION.conversion_path)(bad),
            (REGISTRATION.installation_path)(bad),
        ] {
            assert!(is_safe(&path), "{bad:?} gave {path:?}");
            assert!(Request::get(&path, &[]).is_ok(), "{bad:?} gave {path:?}");
        }
    }
}

#[test]
fn prj_039_a_plain_code_and_id_go_into_their_paths() {
    assert_eq!(
        (REGISTRATION.conversion_path)("Ab-9_z"),
        "/app-manifests/Ab-9_z/conversions"
    );
    assert_eq!(
        (REGISTRATION.installation_path)("4321"),
        "/app/installations/4321"
    );
    assert_eq!(REGISTRATION.installations_path, "/app/installations");
}
