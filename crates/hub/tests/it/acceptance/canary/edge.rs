//! The whole hub as `main.rs` serves it ([`all_routes`]): `/graphql` at a public origin, the
//! agent routes, `/metrics` and the tunnels behind the permitted networks, with a VPN feed and
//! broker, at one fixed instant; and the requests the canary test sends to it.

use std::{collections::BTreeMap, net::SocketAddr, sync::Arc};

use anima_hub::{
    all_routes,
    clock::{Clock, FixedClock},
    config,
    fleet::{
        access,
        events::Bus,
        vpn::{Broker, Feed},
    },
    schema::Dependencies,
};
use axum::{
    Router,
    body::Body,
    extract::ConnectInfo,
    http::{Request, StatusCode, header},
};
use chrono::{DateTime, Utc};
use http_body_util::BodyExt as _;
use ipnet::IpNet;
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt as _;

use crate::hub::{CLIENT, dependencies};

/// The public origin the hub serves, as `ANIMA_PUBLIC_ORIGIN` sets it.
pub const PUBLIC: &str = "https://hub.anima.ing";

/// What the hub answered: the status, every `Set-Cookie` value and the body text.
pub struct Answer {
    pub status: StatusCode,
    pub set_cookies: Vec<String>,
    pub body: String,
}

impl Answer {
    pub fn json(&self) -> Value {
        serde_json::from_str(&self.body).unwrap_or_else(|_| panic!("not JSON: {}", self.status))
    }
}

/// Every route of the hub, served to `CLIENT`.
pub struct WholeHub(Router);

impl WholeHub {
    /// The hub at `now`, its VPN reading `feed` and calling `broker`, with `CLIENT`'s network
    /// permitted (NET-002).
    pub async fn new(pool: &PgPool, now: DateTime<Utc>, (feed, broker): (Feed, Broker)) -> Self {
        let networks: Vec<IpNet> = vec!["100.64.0.0/10".parse().unwrap()];
        access::permit_networks(pool, &networks).await.unwrap();
        let clock: Arc<dyn Clock> = Arc::new(FixedClock(now));
        let bus = Bus::new(64);
        let env = BTreeMap::from([("ANIMA_PUBLIC_ORIGIN".to_owned(), PUBLIC.to_owned())]);
        let edge = access::Edge::new(pool.clone(), bus.clone(), clock.clone());
        let parts = Dependencies {
            config: Arc::new(config::from_env(&env).unwrap()),
            vpn_feed: feed,
            vpn_broker: Some(broker),
            ..dependencies(pool, clock, bus)
        };
        Self(all_routes(parts, edge))
    }

    /// Sends `request` from `CLIENT`.
    pub async fn send(&self, request: Request<Body>) -> Answer {
        let mut request = request;
        let peer: SocketAddr = format!("{CLIENT}:50000").parse().unwrap();
        request.extensions_mut().insert(ConnectInfo(peer));
        let response = self.0.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let set_cookies = (response.headers().get_all(header::SET_COOKIE).iter())
            .map(|value| value.to_str().unwrap().to_owned())
            .collect();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let body = String::from_utf8_lossy(&bytes).into_owned();
        Answer {
            status,
            set_cookies,
            body,
        }
    }

    /// `POST uri` with `headers` and `body`.
    pub async fn post(&self, uri: &str, headers: &[(&str, &str)], body: String) -> Answer {
        let mut request = Request::post(uri).header(header::CONTENT_TYPE, "application/json");
        for (name, value) in headers {
            request = request.header(*name, *value);
        }
        self.send(request.body(Body::from(body)).unwrap()).await
    }

    /// A GraphQL request with `headers`; the response as JSON.
    pub async fn graphql(&self, headers: &[(&str, &str)], query: &str, variables: Value) -> Value {
        self.graphql_answer(headers, query, variables).await.json()
    }

    /// A GraphQL request with `headers`; the whole answer, for its cookies.
    pub async fn graphql_answer(
        &self,
        headers: &[(&str, &str)],
        query: &str,
        variables: Value,
    ) -> Answer {
        let body = json!({ "query": query, "variables": variables }).to_string();
        let answer = self.post("/graphql", headers, body).await;
        assert_eq!(answer.status, StatusCode::OK, "{query}");
        answer
    }

    /// A GraphQL request with `token` as the bearer, which must have no errors; its data.
    pub async fn data(&self, token: &str, query: &str, variables: Value) -> Value {
        let response = self.as_bearer(token, query, variables).await;
        assert!(
            response["errors"].is_null(),
            "{query}: {:?}",
            codes(&response)
        );
        response["data"].clone()
    }

    /// A GraphQL request with `token` as the bearer; the whole response.
    pub async fn as_bearer(&self, token: &str, query: &str, variables: Value) -> Value {
        let bearer = format!("Bearer {token}");
        let headers = [("authorization", bearer.as_str())];
        self.graphql(&headers, query, variables).await
    }

    /// `GET uri` with `token` as the bearer.
    pub async fn get_as(&self, uri: &str, token: &str) -> Answer {
        let request = Request::get(uri).header(header::AUTHORIZATION, format!("Bearer {token}"));
        self.send(request.body(Body::empty()).unwrap()).await
    }
}

/// The error codes of `response`, which never hold a value an operator typed.
pub fn codes(response: &Value) -> Vec<Value> {
    let errors = response["errors"].as_array().cloned().unwrap_or_default();
    errors
        .iter()
        .map(|error| error["extensions"]["code"].clone())
        .collect()
}

/// The code of the one error of `response`.
pub fn code(response: &Value) -> String {
    let codes = codes(response);
    assert_eq!(codes.len(), 1, "{codes:?}");
    codes[0].as_str().unwrap().to_owned()
}
