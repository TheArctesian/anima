//! A stand-in for the operator's browser at the Connect GitHub pages of the real hub: a
//! `reqwest` client with redirects off and a cookie jar of its own, which opens the one-time
//! link, reads the form from the start page and plays GitHub's redirect after "Create GitHub
//! App". It is the hub's own flow-test browser (`crates/hub/tests/it/sources/registration/
//! browser.rs`) cut to what the command line's test needs; that one cannot be included by path,
//! for it stands on the hub suite's own kit.

use std::collections::BTreeMap;

use reqwest::StatusCode;
use reqwest::redirect::Policy;

/// What the hub answered one request of the browser.
pub struct Answer {
    pub status: StatusCode,
    pub location: Option<String>,
    pub body: String,
}

/// The operator's browser: its own cookies, no redirect followed unless the test plays it.
pub struct Browser {
    client: reqwest::Client,
    base: String,
    cookies: BTreeMap<String, String>,
}

impl Browser {
    /// A browser that reaches the hub at `base`, which the links name by its public origin.
    pub fn at(base: &str) -> Self {
        Self {
            client: reqwest::Client::builder()
                .redirect(Policy::none())
                .no_proxy()
                .build()
                .expect("a client"),
            base: base.to_owned(),
            cookies: BTreeMap::new(),
        }
    }

    async fn get(&mut self, path: &str) -> Answer {
        let mut request = self.client.get(format!("{}{path}", self.base));
        if !self.cookies.is_empty() {
            let pairs: Vec<String> = self
                .cookies
                .iter()
                .map(|(name, value)| format!("{name}={value}"))
                .collect();
            request = request.header("cookie", pairs.join("; "));
        }
        let response = request.send().await.expect("the hub answers");
        let status = response.status();
        let location = response
            .headers()
            .get("location")
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        for set in response.headers().get_all("set-cookie") {
            let text = set.to_str().expect("a cookie");
            let (pair, attributes) = text.split_once(';').unwrap_or((text, ""));
            let (name, value) = pair.split_once('=').expect("name=value");
            if value.is_empty() || attributes.contains("Max-Age=0") {
                self.cookies.remove(name);
            } else {
                self.cookies.insert(name.to_owned(), value.to_owned());
            }
        }
        let body = response.text().await.expect("a body");
        Answer {
            status,
            location,
            body,
        }
    }

    /// Opens the one-time `link` (it names the hub's public origin; the path and query go to
    /// `base`), follows the hub to the start page and returns the state in the form's action,
    /// as GitHub would get it.
    pub async fn open(&mut self, link: &str) -> String {
        let at = link.find("/connect/github").expect("a link to the hub");
        let bound = self.get(&link[at..]).await;
        assert_eq!(bound.status, StatusCode::SEE_OTHER, "{}", bound.body);
        assert_eq!(bound.location.as_deref(), Some("/connect/github/start"));
        let page = self.get("/connect/github/start").await;
        assert_eq!(page.status, StatusCode::OK, "{}", page.body);
        let action = page
            .body
            .split("action=\"")
            .nth(1)
            .and_then(|rest| rest.split('"').next())
            .expect("a form");
        assert!(action.starts_with("https://github.com/settings/apps/new?state="));
        action.split("?state=").nth(1).expect("a state").to_owned()
    }

    /// GitHub's redirect after "Create GitHub App", with the `code` GitHub gave and `state`.
    pub async fn created(&mut self, code: &str, state: &str) -> Answer {
        self.get(&format!(
            "/connect/github/created?code={code}&state={state}"
        ))
        .await
    }
}
