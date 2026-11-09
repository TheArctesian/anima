//! Session shares (RA-035, D-7, D-9, S-1 to S-6): the bounds, the registry, `shareSession`,
//! `endShare`, `Session.shares` and the keeper, against a session whose tunnel is registered
//! in `Live` from a redeemed ticket, as a tunnel task holds it.

mod cancelling;
mod ending;
mod logging;
mod racing;
mod refusals;
mod registry;
mod sharing;
mod terms;

use std::{collections::BTreeMap, sync::Arc, time::Duration};

use async_graphql::{EmptySubscription, MergedObject, Request, Response, Schema, Variables};
use chrono::TimeDelta;
use serde_json::{Value, json};
use sqlx::PgPool;

use super::ShareMutation;
use crate::{
    access::Identity,
    clock::{Clock, FixedClock},
    config,
    events::{Bus, Operator},
    ids::{OperatorId, SessionId},
    sessions::{
        Live, Presented, Tunnel,
        graphql::{Mutation as SessionsMutation, Query},
        start,
        tests::{
            ED25519,
            fixture::{address, now, operator, prepare, request, vault},
        },
    },
    trust::PublicKey,
};

/// The address the owner's client calls from.
pub(super) const CLIENT: &str = "100.64.0.7";

/// The public origin `config::from_env` gives when `ANIMA_PUBLIC_ORIGIN` is unset.
pub(super) const ORIGIN: &str = "https://hub.anima.ing";

/// The share mutations and the sessions part's, merged as `schema.rs` merges them.
#[derive(MergedObject, Default)]
pub(super) struct Mutation(ShareMutation, SessionsMutation);

pub(super) type TestSchema = Schema<Query, Mutation, EmptySubscription>;

/// A started session on `docker-host` whose tunnel is open: its ticket was redeemed and the
/// registration is held here, as a tunnel task holds it.
pub(super) struct Bench {
    pub(super) pool: PgPool,
    pub(super) live: Live,
    pub(super) schema: TestSchema,
    pub(super) session: SessionId,
    pub(super) tunnel: Option<Tunnel>,
}

/// The schema `schema.rs` would build, on `live` and `pool`, with a clock fixed at `now()`.
pub(super) fn schema(pool: &PgPool, live: &Live) -> TestSchema {
    let clock: Arc<dyn Clock> = Arc::new(FixedClock(now()));
    let config = config::from_env(&BTreeMap::new()).unwrap();
    Schema::build(Query, Mutation::default(), EmptySubscription)
        .data(pool.clone())
        .data(clock)
        .data(live.clone())
        .data(Bus::new(16))
        .data(Arc::new(config))
        .finish()
}

/// Starts a session on `docker-host` from [`CLIENT`] and returns its ticket's token.
pub(super) async fn started(pool: &PgPool, live: &Live) -> (SessionId, String) {
    let clock: Arc<dyn Clock> = Arc::new(FixedClock(now()));
    let ticket = start(pool, &clock, live, &operator(), address(CLIENT), request())
        .await
        .unwrap();
    (ticket.session, ticket.token().to_owned())
}

/// A session with an open tunnel. `endSession` and shutdown wait at most `wait` for it.
pub(super) async fn bench_waiting(pool: &PgPool, wait: Duration) -> Bench {
    let key = PublicKey::from_base64(ED25519).unwrap();
    prepare(pool, &vault(), &key).await;
    let live = Live::with_tunnel_end_wait(wait);
    let (session, token) = started(pool, &live).await;
    let redeemed = live
        .redeem(Presented::terminal(&token, address(CLIENT)), now())
        .unwrap();
    Bench {
        pool: pool.clone(),
        schema: schema(pool, &live),
        live,
        session,
        tunnel: Some(redeemed.tunnel),
    }
}

pub(super) async fn bench(pool: &PgPool) -> Bench {
    bench_waiting(pool, Duration::from_secs(5)).await
}

impl Bench {
    /// Plays the tunnel task: when the tunnel is told to end, it deregisters.
    pub(super) fn ends_when_told(&mut self) {
        let mut tunnel = self.tunnel.take().unwrap();
        tokio::spawn(async move {
            tunnel.ended().await;
            drop(tunnel);
        });
    }

    /// Runs `query` with `variables` as `by`, signed in, from `client`.
    pub(super) async fn run_as(
        &self,
        (by, client): (Operator, &str),
        query: &str,
        variables: Value,
    ) -> Response {
        let identity = Identity::for_tests(by, now() + TimeDelta::days(1));
        let request = Request::new(query).variables(Variables::from_json(variables));
        self.schema
            .execute(identity.attach(request).data(address(client)))
            .await
    }

    /// Runs `query` as the owner, from [`CLIENT`].
    pub(super) async fn run(&self, query: &str, variables: Value) -> Response {
        self.run_as((operator(), CLIENT), query, variables).await
    }

    /// `shareSession` for the bench's session with `input`, as the owner.
    pub(super) async fn share(&self, input: Value) -> Response {
        let variables = json!({ "id": self.session.to_string(), "input": input });
        self.run(SHARE, variables).await
    }

    /// A share with K1's defaults, as `ShareLink` JSON; it must succeed.
    pub(super) async fn shared(&self) -> Value {
        let response = self.share(json!({})).await;
        assert!(response.errors.is_empty(), "{:?}", response.errors);
        response.data.into_json().unwrap()["shareSession"].clone()
    }

    /// `endShare(id)` as `by` from `client`.
    pub(super) async fn end_share_as(&self, by: Operator, client: &str, id: &str) -> Response {
        self.run_as((by, client), END, json!({ "id": id })).await
    }

    /// The ids `Session.shares` lists for the bench's session.
    pub(super) async fn listed(&self) -> Vec<String> {
        let response = self.run(LISTED, json!({})).await;
        assert!(response.errors.is_empty(), "{:?}", response.errors);
        let data = response.data.into_json().unwrap();
        let session = self.session.to_string();
        let node = data["sessions"]["edges"]
            .as_array()
            .unwrap()
            .iter()
            .map(|edge| &edge["node"])
            .find(|node| node["id"] == session.as_str())
            .unwrap()
            .clone();
        node["shares"]
            .as_array()
            .unwrap()
            .iter()
            .map(|share| share["id"].as_str().unwrap().to_owned())
            .collect()
    }
}

pub(super) const SHARE: &str = "mutation($id: ID!, $input: ShareInput!) { \
    shareSession(sessionId: $id, input: $input) { url share { id sessionId access maxViewers \
    viewers createdAt expiresAt } } }";

pub(super) const END: &str = "mutation($id: ID!) { endShare(id: $id) }";

const LISTED: &str = "{ sessions { edges { node { id shares { id } } } } }";

/// An operator who did not start the bench's session.
pub(super) fn other_operator() -> Operator {
    Operator {
        id: "0190a000-0000-7000-8000-0000000000ff"
            .parse::<OperatorId>()
            .unwrap(),
        name: "other".to_owned(),
    }
}

/// The only error's code and message.
pub(super) fn error_of(response: &Response) -> (String, String) {
    assert_eq!(response.errors.len(), 1, "{:?}", response.errors);
    let error = serde_json::to_value(&response.errors[0]).unwrap();
    let code = error["extensions"]["code"].as_str().unwrap().to_owned();
    (code, error["message"].as_str().unwrap().to_owned())
}

/// `(action, target id, client, change)` of each audit entry about a share, oldest first.
pub(super) async fn share_audits(pool: &PgPool) -> Vec<(String, String, String, Value)> {
    sqlx::query_as(
        "select action, target_id, host(client_addr), change from events.audit_log \
         where target_type = 'share' order by at, id",
    )
    .fetch_all(pool)
    .await
    .unwrap()
}

/// `(severity, source, device name, message)` of each event.
pub(super) async fn events(pool: &PgPool) -> Vec<(String, String, Option<String>, String)> {
    sqlx::query_as(
        "select severity, source, device_name, message from events.event_log order by at, id",
    )
    .fetch_all(pool)
    .await
    .unwrap()
}

/// Waits up to five seconds for the `share.end` entry of `share`, then returns its client and
/// change.
pub(super) async fn share_end(pool: &PgPool, share: &str) -> (String, Value) {
    for _ in 0..250 {
        let ends: Vec<(String, Value)> = sqlx::query_as(
            "select host(client_addr), change from events.audit_log \
             where target_type = 'share' and action = 'end' and target_id = $1",
        )
        .bind(share)
        .fetch_all(pool)
        .await
        .unwrap();
        if let Some(end) = ends.into_iter().next() {
            return end;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("the share did not end");
}
