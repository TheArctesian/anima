//! The hub served on a loopback port with every route, GitHub's replies on the stub, and a
//! stand-in for the operator's browser: a `reqwest` client with redirects off and a cookie jar
//! of its own, which reads the form from the start page and plays GitHub's redirects itself.

use std::{collections::BTreeMap, net::SocketAddr, sync::Arc, time::Duration};

use anima_hub::{
    all_routes,
    clock::{Clock, FixedClock},
    engine::sources::{Attempts, Reach, Registry},
    fleet::{access, events::Bus},
    schema::Dependencies,
};
use reqwest::{StatusCode, header::HeaderMap, redirect::Policy};
use serde_json::{Value, json};
use sqlx::PgPool;
use tokio::task::JoinHandle;

use crate::{
    hub::{Hub, dependencies, set_operator, start},
    sources::{Kit, TEST_KEY, intake, stub::Stub},
};

pub const CODE: &str = "CODEMARKER7c1f";
pub const CONVERSION: &str = "/app-manifests/CODEMARKER7c1f/conversions";
pub const INSTALLATION: &str = "4321";
pub const PUBLIC: &str = "https://hub.anima.ing";
pub const LIST: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../providers/github/tests/fixtures/installations.json"
));
pub const ONE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../providers/github/tests/fixtures/installation.json"
));

/// GitHub's conversion reply for the test App, with the test key and two secrets the hub must
/// never read.
pub fn converted() -> String {
    json!({
        "id": 1_234_567, "slug": "anima-k3f9x2mq7b",
        "owner": { "login": "octo-person", "id": 9001, "type": "User" },
        "client_secret": "CLIENT-SECRET-MARKER-0000",
        "webhook_secret": "WEBHOOK-SECRET-MARKER-1111",
        "pem": *TEST_KEY,
    })
    .to_string()
}

/// A kit whose hub serves every route on a loopback port; its poll asks every `poll_every`.
pub struct Served {
    pub kit: Kit,
    pub base: String,
    task: JoinHandle<()>,
}

impl Drop for Served {
    fn drop(&mut self) {
        self.task.abort();
    }
}

/// How long the bound browser's page on `/installed` waits for the attempt to end, in tests.
const PAGE_WAIT: Duration = Duration::from_millis(500);

/// The served hub, with GitHub's conversion and installation replies on the stub.
pub async fn served(pool: &PgPool, poll_every: Duration) -> Served {
    served_waiting(pool, poll_every, PAGE_WAIT).await
}

/// As [`served`], with the page on `/installed` waiting up to `page_wait`.
pub async fn served_waiting(pool: &PgPool, poll_every: Duration, page_wait: Duration) -> Served {
    let stub = Stub::start(pool).await;
    let transport = anima_hub::engine::sources::Transport::to_stub(
        stub.base.clone(),
        anima_hub::fleet::targets::AddressPolicy::default().allowing_loopback(),
    )
    .unwrap();
    intake::make_table(pool).await;
    set_operator(pool).await;
    let networks: [ipnet::IpNet; 1] = ["127.0.0.0/8".parse().unwrap()];
    access::permit_networks(pool, &networks).await.unwrap();
    let intake = intake::StubIntake::default();
    let registry = Registry::new(
        vec![provider_github::DECLARATION],
        vec![Arc::new(intake.clone())],
    );
    let reach = Reach {
        attempts: Attempts::new(poll_every, page_wait),
        ..Reach::new(registry, transport)
    };
    let clock: Arc<dyn Clock> = Arc::new(FixedClock(start()));
    let parts = || Dependencies {
        sources: reach.clone(),
        ..dependencies(pool, clock.clone(), Bus::new(16))
    };
    let hub = Hub::serving(pool, parts());
    let token = hub.token().await;
    let edge = access::Edge::new(pool.clone(), Bus::new(4), clock.clone());
    let app = all_routes(parts(), edge);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let service = app.into_make_service_with_connect_info::<SocketAddr>();
    let task = tokio::spawn(async move { axum::serve(listener, service).await.unwrap() });
    stub.reply(CONVERSION, reply(201, &converted()));
    stub.reply(
        &format!("/app/installations/{INSTALLATION}"),
        reply(200, ONE),
    );
    let kit = Kit {
        pool: pool.clone(),
        hub,
        token,
        stub,
        intake,
        reach,
    };
    Served {
        kit,
        base: format!("http://{addr}"),
        task,
    }
}

pub fn reply(status: u16, body: &str) -> crate::sources::stub::Reply {
    crate::sources::stub::Reply {
        status,
        headers: Vec::new(),
        body: body.to_owned(),
    }
}

/// What the hub answered.
pub struct Answer {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: String,
}

impl Answer {
    pub fn location(&self) -> &str {
        self.headers
            .get("location")
            .and_then(|v| v.to_str().ok())
            .unwrap_or_else(|| panic!("no Location: {} {}", self.status, self.body))
    }
}

/// The operator's browser: its own cookies, no redirect followed unless asked.
pub struct Browser {
    client: reqwest::Client,
    base: String,
    pub cookies: BTreeMap<String, String>,
}

impl Browser {
    pub fn new(served: &Served) -> Self {
        Self {
            client: reqwest::Client::builder()
                .redirect(Policy::none())
                .build()
                .unwrap(),
            base: served.base.clone(),
            cookies: BTreeMap::new(),
        }
    }

    fn cookie_header(&self) -> String {
        let pairs: Vec<String> = self
            .cookies
            .iter()
            .map(|(n, v)| format!("{n}={v}"))
            .collect();
        pairs.join("; ")
    }

    async fn send(&mut self, request: reqwest::RequestBuilder) -> Answer {
        let request = if self.cookies.is_empty() {
            request
        } else {
            request.header("cookie", self.cookie_header())
        };
        let response = request.send().await.unwrap();
        let (status, headers) = (response.status(), response.headers().clone());
        for set in headers.get_all("set-cookie") {
            let text = set.to_str().unwrap();
            let (pair, attributes) = text.split_once(';').unwrap_or((text, ""));
            let (name, value) = pair.split_once('=').unwrap();
            if value.is_empty() || attributes.contains("Max-Age=0") {
                self.cookies.remove(name);
            } else {
                self.cookies.insert(name.to_owned(), value.to_owned());
            }
        }
        let body = response.text().await.unwrap();
        Answer {
            status,
            headers,
            body,
        }
    }

    /// `GET` a path of the hub (with its query).
    pub async fn get(&mut self, path: &str) -> Answer {
        let request = self.client.get(format!("{}{path}", self.base));
        self.send(request).await
    }

    /// `POST` a path of the hub with the extra `headers`, as a form with no fields.
    pub async fn post(&mut self, path: &str, headers: &[(&str, &str)]) -> Answer {
        let mut request = self
            .client
            .post(format!("{}{path}", self.base))
            .header("content-type", "application/x-www-form-urlencoded");
        for (name, value) in headers {
            request = request.header(*name, *value);
        }
        self.send(request).await
    }

    /// Signs this browser in to Anima from the public origin: it gets the `__Host-anima` cookie.
    pub async fn sign_in(&mut self) {
        let body = json!({
            "query": "mutation($input: LoginInput!) { signInBrowser(input: $input) { expiresAt } }",
            "variables": { "input": { "username": crate::hub::USERNAME, "password": crate::hub::PASSWORD } },
        });
        let request = self
            .client
            .post(format!("{}/graphql", self.base))
            .header("content-type", "application/json")
            .header("origin", PUBLIC)
            .body(body.to_string());
        let answer = self.send(request).await;
        assert!(self.cookies.contains_key("__Host-anima"), "{}", answer.body);
    }

    /// Opens the one-time `link` and follows the hub to the start page: returns the state S1
    /// in the form's action, as GitHub would get it.
    pub async fn open(&mut self, link: &str) -> String {
        let path = link.strip_prefix(PUBLIC).expect("a link to the hub");
        let bound = self.get(path).await;
        assert_eq!(bound.status, StatusCode::SEE_OTHER, "{}", bound.body);
        assert_eq!(bound.location(), "/connect/github/start");
        self.form_state().await
    }

    /// The start page's state S1.
    pub async fn form_state(&mut self) -> String {
        let page = self.get("/connect/github/start").await;
        assert_eq!(page.status, StatusCode::OK, "{}", page.body);
        let action = page
            .body
            .split("action=\"")
            .nth(1)
            .and_then(|rest| rest.split('"').next())
            .unwrap();
        let state = action
            .split("?state=")
            .nth(1)
            .expect("a state in the action");
        assert!(action.starts_with("https://github.com/settings/apps/new?state="));
        state.to_owned()
    }

    /// GitHub's redirect after "Create GitHub App" with `state`; returns the hub's answer.
    pub async fn created(&mut self, state: &str) -> Answer {
        self.get(&format!(
            "/connect/github/created?code={CODE}&state={state}"
        ))
        .await
    }

    /// GitHub's redirect after "Install" of `installation` with `state`.
    pub async fn installed(&mut self, installation: &str, state: &str) -> Answer {
        let path = format!(
            "/connect/github/installed?installation_id={installation}&setup_action=install&state={state}"
        );
        self.get(&path).await
    }
}

/// The state S2 in the install URL the hub sent the browser to.
pub fn install_state(answer: &Answer) -> String {
    assert_eq!(answer.status, StatusCode::SEE_OTHER, "{}", answer.body);
    let location = answer.location();
    let state = location
        .strip_prefix("https://github.com/apps/anima-k3f9x2mq7b/installations/new?state=")
        .unwrap_or_else(|| panic!("not the install page: {location}"));
    state.to_owned()
}

/// The attempt as `githubConnect` shows it.
pub async fn shown(kit: &Kit) -> Value {
    let response = kit
        .hub
        .post(Some(&kit.token), super::STATE, json!({}))
        .await;
    response["data"]["githubConnect"].clone()
}

/// How many conversions the stub saw.
pub fn conversions(kit: &Kit) -> usize {
    kit.stub
        .paths()
        .iter()
        .filter(|path| path.starts_with("/app-manifests/"))
        .count()
}
