//! K-E1 Section 13.2, OD-E14: a change leaves as the provider rendered it with the headers of
//! decision 1 and the token in `Authorization` only; every reply reaches the provider's
//! `settle`; a change whose answer was lost is `Unknown` and sent once; a source that makes no
//! changes is refused before anything is sent.

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use anima_hub::{
    engine::sources::{LiveFailure, Reserve, Transport, change},
    fleet::targets::AddressPolicy,
};
use chrono::{TimeDelta, TimeZone as _, Utc};
use reqwest::Url;
use serde_json::json;
use source_exchange::{
    Change, CommitSha, Declaration, IssueState, MergeMethod, Minted, Number, Receipt,
    RepositoryName, Settled,
};
use sqlx::PgPool;
use tokio::{io::AsyncReadExt as _, net::TcpListener};

use super::{COMMENT_PATH, REPOSITORY, comment, live_kit, reply, striding};
use crate::{
    hub::start,
    sources::{kit_of, stub::Heard, stub::Stub, unreachable},
};

fn sent(kit: &crate::sources::Kit, path: &str) -> Vec<Heard> {
    kit.stub
        .with(|s| s.heard.iter().filter(|h| h.path == path).cloned().collect())
}

#[sqlx::test(migrations = "./migrations")]
async fn od_e14_the_stub_hears_exactly_the_verb_path_body_and_headers(pool: PgPool) {
    let (kit, id) = live_kit(&pool).await;
    // The longest text, every character escaped: past the 64 KiB a smaller cap would allow.
    let text = "\"".repeat(65_536);
    kit.stub.reply(COMMENT_PATH, reply(201, &[], "{}"));

    let made = change(&kit.at(start()), id, &comment(&text), Reserve::Spendable)
        .await
        .unwrap();

    assert_eq!(made, Settled::Done(Receipt::Created { number: 201 }));
    let heard = sent(&kit, COMMENT_PATH);
    assert_eq!(heard.len(), 1);
    let one = &heard[0];
    assert_eq!(one.method, "POST");
    let body: serde_json::Value = serde_json::from_slice(&one.body).unwrap();
    assert_eq!(body, json!({ "body": text }));
    let bearer = format!("Bearer {}", kit.stub.token());
    assert_eq!(one.header("authorization"), [bearer.as_bytes()]);
    assert_eq!(
        one.header("accept"),
        [b"application/vnd.github+json".as_slice()]
    );
    assert_eq!(
        one.header("x-github-api-version"),
        [b"2022-11-28".as_slice()]
    );
    assert_eq!(one.header("content-type"), [b"application/json".as_slice()]);
    assert_eq!(one.header("user-agent"), [b"anima-hub".as_slice()]);
    let token = kit.stub.token();
    let elsewhere = one
        .headers
        .iter()
        .filter(|(name, _)| name != "authorization")
        .any(|(_, value)| String::from_utf8_lossy(value).contains(&token));
    assert!(!elsewhere && !one.path.contains(&token));
    assert!(!String::from_utf8_lossy(&one.body).contains(&token));
}

#[sqlx::test(migrations = "./migrations")]
async fn od_e14_the_verb_is_the_changes_own(pool: PgPool) {
    let (kit, id) = live_kit(&pool).await;
    let repository = RepositoryName::parse(REPOSITORY).unwrap();
    let number = Number::new(12).unwrap();
    let merge = Change::MergePullRequest {
        repository: repository.clone(),
        number,
        head: CommitSha::parse(&"a".repeat(40)).unwrap(),
        method: MergeMethod::Squash,
    };
    let close = Change::SetIssueOpen {
        repository,
        number,
        state: IssueState::Open,
    };
    let s = striding(&kit, kit.reach.transport.clone());
    for (made, verb) in [(merge, "PUT"), (close, "PATCH")] {
        change(&s, id, &made, Reserve::Spendable).await.unwrap();
        let path = format!("/repos/octo/widgets/test-changes/{}", made.function());
        let heard = sent(&kit, &path);
        assert_eq!(
            heard.iter().map(|h| h.method.as_str()).collect::<Vec<_>>(),
            [verb]
        );
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn od_e14_every_status_reaches_settle_and_a_401_drops_the_token(pool: PgPool) {
    let (kit, id) = live_kit(&pool).await;
    let s = striding(&kit, kit.reach.transport.clone());
    for status in [
        200, 201, 202, 204, 400, 401, 403, 404, 405, 409, 410, 422, 429, 500, 502,
    ] {
        kit.stub
            .reply(COMMENT_PATH, reply(status, &[], "{\"message\":\"no\"}"));
        let settled = change(&s, id, &comment("hello"), Reserve::Spendable)
            .await
            .unwrap();
        assert_eq!(
            settled,
            Settled::Done(Receipt::Created {
                number: u64::from(status)
            })
        );
        if status == 401 {
            let token = kit.reach.transport.tokens().fresh(id, start());
            assert!(token.is_none(), "the token is dropped");
        }
    }
    let reset = start() + TimeDelta::minutes(5);
    let headers = [
        ("x-ratelimit-remaining", "0"),
        ("x-ratelimit-reset", &*reset.timestamp().to_string()),
    ];
    kit.stub.reply(COMMENT_PATH, reply(403, &headers, "{}"));
    let limited = change(&s, id, &comment("hello"), Reserve::Spendable)
        .await
        .unwrap();
    assert_eq!(
        limited,
        Settled::Limited { until: reset },
        "the limit headers, as numbers"
    );
}

/// A server that reads one whole request, counts it and closes the connection unanswered.
async fn hang_up() -> (Url, Arc<AtomicUsize>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = Url::parse(&format!("http://{}", listener.local_addr().unwrap())).unwrap();
    let count = Arc::new(AtomicUsize::new(0));
    let heard = count.clone();
    tokio::spawn(async move {
        loop {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut text = Vec::new();
            let mut chunk = [0_u8; 4096];
            while !text.windows(4).any(|w| w == b"\r\n\r\n") {
                let n = socket.read(&mut chunk).await.unwrap();
                if n == 0 {
                    break;
                }
                text.extend_from_slice(&chunk[..n]);
            }
            heard.fetch_add(1, Ordering::SeqCst);
            drop(socket);
        }
    });
    (base, count)
}

/// A token for `id` in `transport`, so no mint is needed.
fn hand_token(transport: &Transport, id: anima_hub::ids::SourceAccountId) {
    let minted = Minted {
        token: "handed-token".to_owned(),
        expires_at: Utc.with_ymd_and_hms(2099, 1, 1, 0, 0, 0).unwrap(),
    };
    let ticket = transport.tokens().ticket();
    drop(transport.tokens().keep(id, minted, ticket));
}

#[sqlx::test(migrations = "./migrations")]
async fn od_e14_a_change_whose_answer_was_lost_is_unknown_and_sent_once(pool: PgPool) {
    let (kit, id) = live_kit(&pool).await;
    let (base, count) = hang_up().await;
    let transport = Transport::to_stub(base, AddressPolicy::default().allowing_loopback()).unwrap();
    hand_token(&transport, id);

    let settled = change(
        &striding(&kit, transport),
        id,
        &comment("hello"),
        Reserve::Spendable,
    )
    .await;

    assert_eq!(settled.unwrap(), Settled::Unknown);
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    assert_eq!(count.load(Ordering::SeqCst), 1, "sent once, never again");
}

#[sqlx::test(migrations = "./migrations")]
async fn od_e14_a_change_that_could_not_connect_is_unreachable(pool: PgPool) {
    let (kit, id) = live_kit(&pool).await;
    let transport = unreachable();
    hand_token(&transport, id);

    let failed = change(
        &striding(&kit, transport),
        id,
        &comment("hello"),
        Reserve::Spendable,
    )
    .await;

    assert_eq!(failed.unwrap_err(), LiveFailure::Unreachable);
}

#[sqlx::test(migrations = "./migrations")]
async fn od_e14_a_source_that_declares_no_changes_is_refused_before_sending(pool: PgPool) {
    let stub = Stub::start(&pool).await;
    let transport = Transport::to_stub(
        stub.base.clone(),
        AddressPolicy::default().allowing_loopback(),
    )
    .unwrap();
    let reads_only = Declaration {
        changes: None,
        ..provider_github::DECLARATION
    };
    let kit = kit_of(&pool, (stub, transport), reads_only).await;
    let (_, id) = kit.connect().await;
    let before = kit.stub.paths();

    let refused = change(&kit.at(start()), id, &comment("hello"), Reserve::Spendable).await;

    assert_eq!(refused.unwrap_err(), LiveFailure::NoChanges);
    assert_eq!(kit.stub.paths(), before, "nothing was sent");
}
