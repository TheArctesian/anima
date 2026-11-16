//! Live reads and named changes against the loopback GitHub (K-E1 Section 13; PRJ-027 to
//! PRJ-035, OD-E14): the cache, the budget, the pace, the change path, the leaks and the sync
//! left alone.

mod budget;
mod cache;
mod changes;
mod leaks;
mod pace;
mod sync;

use std::sync::{
    Arc,
    atomic::{AtomicI64, Ordering},
};

use anima_hub::{
    clock::Clock,
    engine::sources::{Scheduling, Transport},
    fleet::{events::Bus, targets::AddressPolicy},
    ids::SourceAccountId,
};
use chrono::{DateTime, TimeDelta, Utc};
use serde_json::{Value, json};
use source_exchange::{
    Change, ChangePath, ChangeReply, Changes, Declaration, Fixed, Number, Receipt, Rendered,
    RepositoryName, Request, Settled, Text,
};
use sqlx::PgPool;

use super::{CONNECT, Kit, kit_of, stub::Reply, stub::Stub};
use crate::hub::{start, test_vault};

/// GitHub as declared, with changes rendered by this test's own provider.
pub const CHANGING: Declaration = Declaration {
    changes: Some(Changes { render, settle }),
    ..provider_github::DECLARATION
};

pub const REPOSITORY: &str = "octo/widgets";
pub const PULLS: &str = "/repos/octo/widgets/pulls";
/// Where the test provider sends a comment.
pub const COMMENT_PATH: &str = "/repos/octo/widgets/test-changes/add_issue_comment";

/// `/repos/{owner}/{name}/test-changes/{function}`, from fixed segments and checked parts.
fn render(change: &Change) -> Rendered {
    let path = ChangePath::new(Fixed::new("repos"))
        .repository(change.repository())
        .fixed(Fixed::new("test-changes"))
        .fixed(Fixed::new(change.function()));
    let body = match change {
        Change::AddIssueComment { body, .. } => Some(json!({ "body": body.as_str() })),
        Change::CancelWorkflowRun { .. } => None,
        other => Some(json!({ "function": other.function() })),
    };
    Rendered::new(change, path, body)
}

/// Every status the source sent, as the receipt's number; a reply with no request left and a
/// reset time is a limit.
fn settle(_: &Change, reply: &ChangeReply<'_>) -> Settled {
    match (reply.limit.remaining, reply.limit.reset_at) {
        (Some(0), Some(until)) => Settled::Limited { until },
        _ => Settled::Done(Receipt::Created {
            number: u64::from(reply.status),
        }),
    }
}

/// A comment of `text` on issue 7.
pub fn comment(text: &str) -> Change {
    Change::AddIssueComment {
        repository: RepositoryName::parse(REPOSITORY).unwrap(),
        number: Number::new(7).unwrap(),
        body: Text::parse(text).unwrap(),
    }
}

pub fn get(path: &str) -> Request {
    Request::get(path, &[]).unwrap()
}

/// A reply of `status` with `headers` and `body`.
pub fn reply(status: u16, headers: &[(&'static str, &str)], body: &str) -> Reply {
    Reply {
        status,
        headers: headers.iter().map(|(n, v)| (*n, (*v).to_owned())).collect(),
        body: body.to_owned(),
    }
}

/// 200 with `body` and plenty of requests left.
pub fn ok(body: &str) -> Reply {
    reply(200, &[("x-ratelimit-remaining", "4000")], body)
}

/// A kit whose GitHub makes changes, with one account connected.
pub async fn live_kit(pool: &PgPool) -> (Kit, SourceAccountId) {
    let stub = Stub::start(pool).await;
    let transport = Transport::to_stub(
        stub.base.clone(),
        AddressPolicy::default().allowing_loopback(),
    )
    .unwrap();
    let kit = kit_of(pool, (stub, transport), CHANGING).await;
    let (_, id) = kit.connect().await;
    (kit, id)
}

/// Connects a second account, installation `installation` of the same App.
pub async fn second_account(kit: &Kit, installation: &str) -> SourceAccountId {
    let mut input = super::input(&super::TEST_KEY);
    input["input"]["installationId"] = Value::from(installation);
    input["input"]["label"] = Value::from("Second");
    let response = kit.hub.post(Some(&kit.token), CONNECT, input).await;
    response["data"]["connectGithubApp"]["id"]
        .as_str()
        .unwrap_or_else(|| panic!("connect failed: {response}"))
        .parse()
        .unwrap()
}

/// A clock that moves ten seconds on at each reading, so a change never waits for the last.
struct Striding(AtomicI64);

impl Clock for Striding {
    fn now(&self) -> DateTime<Utc> {
        start() + TimeDelta::seconds(self.0.fetch_add(10, Ordering::SeqCst))
    }
}

/// The kit's parts with `transport` and a striding clock.
pub fn striding(kit: &Kit, transport: Transport) -> Scheduling {
    let clock: Arc<dyn Clock> = Arc::new(Striding(AtomicI64::new(0)));
    let reach = anima_hub::engine::sources::Reach::new(kit.reach.registry.clone(), transport);
    Scheduling::new((kit.pool.clone(), Bus::new(16), clock), test_vault(), reach)
}

/// Waits until the stub has heard `count` GET requests, for at most five seconds.
pub async fn heard_gets(stub: &Stub, count: usize) {
    for _ in 0..500 {
        if stub.gets().len() >= count {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    panic!("the stub heard {:?}", stub.gets());
}
