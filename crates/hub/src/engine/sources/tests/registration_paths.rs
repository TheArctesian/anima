//! PRJ-039: a code or an installation id from a browser redirect reaches a path only when it
//! is plain, and then the path stays one fixed shape below the API base, whatever the text.

use proptest::prelude::*;
use reqwest::Url;
use source_exchange::{Acquisition, Minting, Registration, Request};

use crate::engine::sources::acquisition::{
    app_key::digits, conversion::is_code, transport::address,
};

const BASE: &str = "https://api.github.com";

fn registration() -> Registration {
    let Acquisition::TokenPull(Minting::AppKey(app)) = provider_github::DECLARATION.acquisition
    else {
        unreachable!("GitHub mints with an App key");
    };
    app.registration.expect("GitHub can make its App")
}

/// The URL of `path` below the API base, or `None` when the request or the address refuses it.
fn url_of(path: &str) -> Option<Url> {
    let request = Request::get(path, &[]).ok()?;
    address(&Url::parse(BASE).unwrap(), &request.path, &request.query).ok()
}

#[test]
fn prj_039_the_edges_of_a_plain_code_and_id() {
    assert!(is_code("aZ09-_"));
    assert!(is_code(&"a".repeat(200)));
    for code in ["", "a/b", "..", "%2e", "a.b", "a b", "ü", &"a".repeat(201)] {
        assert!(!is_code(code), "{code:?}");
    }
    assert!(digits("1") && digits(&"9".repeat(20)));
    for id in ["", "-1", "1a", "1/2", "１", &"9".repeat(21)] {
        assert!(!digits(id), "{id:?}");
    }
}

proptest! {
    #[test]
    fn prj_039_a_plain_code_gives_exactly_its_conversion_url(
        code in prop_oneof!["[A-Za-z0-9_.%/-]{0,210}", "\\PC{0,210}"],
    ) {
        let registration = registration();
        if is_code(&code) {
            let url = url_of(&(registration.conversion_path)(&code)).expect("a plain code is sent");
            prop_assert_eq!(url.as_str(), format!("{BASE}/app-manifests/{code}/conversions"));
            prop_assert!(code.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'));
        }
    }

    #[test]
    fn prj_039_a_digit_id_gives_exactly_its_installation_url(
        id in prop_oneof!["[0-9./%a-]{0,24}", "\\PC{0,24}"],
    ) {
        let registration = registration();
        if digits(&id) {
            let url = url_of(&(registration.installation_path)(&id)).expect("digits are sent");
            prop_assert_eq!(url.as_str(), format!("{BASE}/app/installations/{id}"));
        }
    }

    #[test]
    fn prj_039_a_plain_looking_code_is_accepted(code in "[A-Za-z0-9_-]{1,200}") {
        prop_assert!(is_code(&code));
    }
}
