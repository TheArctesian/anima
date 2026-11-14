use source_exchange::{Owner, Request};

use crate::registration::REGISTRATION;

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
