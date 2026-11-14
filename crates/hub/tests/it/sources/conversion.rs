//! PRJ-039, SRC-008: the manifest code is converted with one POST that carries no credential
//! and no body, only a plain code reaches a path, and each reply maps to one fixed failure.

use std::time::{Duration, Instant};

use anima_hub::engine::sources::{
    Transport,
    acquisition::{
        conversion::{ConvertFailure, convert},
        transport::BODY_CAP,
    },
};
use anima_hub::fleet::targets::AddressPolicy;
use chrono::TimeDelta;
use source_exchange::{Acquisition, Converted, Declaration, Minting};
use sqlx::PgPool;

use super::{
    stub::{Reply, Stub},
    unreachable,
};
use crate::hub::start;

/// GitHub's reply to a conversion, with a client secret, a webhook secret and a key in it.
pub const CONVERTED: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../providers/github/tests/fixtures/manifest_converted.json"
));
pub const CODE: &str = "abc123";
pub const PATH: &str = "/app-manifests/abc123/conversions";

pub async fn stub(pool: &PgPool) -> (Stub, Transport) {
    let stub = Stub::start(pool).await;
    let transport = Transport::to_stub(
        stub.base.clone(),
        AddressPolicy::default().allowing_loopback(),
    )
    .unwrap();
    (stub, transport)
}

pub fn reply(status: u16, headers: &[(&'static str, &str)], body: &str) -> Reply {
    Reply {
        status,
        headers: headers.iter().map(|(n, v)| (*n, (*v).to_owned())).collect(),
        body: body.to_owned(),
    }
}

/// Converts `code` with GitHub's declaration, never waiting before the second try.
pub async fn converting(transport: &Transport, code: &str) -> Result<Converted, ConvertFailure> {
    convert(
        transport,
        &provider_github::DECLARATION,
        code,
        (start(), Duration::ZERO),
    )
    .await
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_039_a_201_gives_the_app_from_a_post_with_no_credential_and_no_body(pool: PgPool) {
    let (stub, transport) = stub(&pool).await;
    stub.reply(PATH, reply(201, &[], CONVERTED));
    let Ok(app) = converting(&transport, CODE).await else {
        panic!("the conversion failed");
    };
    assert_eq!(
        (app.app_id, app.slug.as_str()),
        (1_234_567, "anima-k3f9x2mq7b")
    );
    assert_eq!(
        (app.owner.id, app.owner.login.as_str()),
        (9001, "octo-person")
    );
    assert!(!app.owner.organisation);
    assert!(app.private_key_pem.contains("PEM-MARKER-2222"));
    let heard = stub.with(|s| s.heard.clone());
    assert_eq!(heard.len(), 1, "one request");
    let post = &heard[0];
    assert_eq!((post.method.as_str(), post.path.as_str()), ("POST", PATH));
    assert!(post.header("authorization").is_empty(), "no credential");
    assert!(post.body.is_empty(), "no body");
    assert_eq!(post.header("accept"), [b"application/json".as_slice()]);
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_039_a_code_that_is_not_plain_is_refused_before_any_request(pool: PgPool) {
    let (stub, transport) = stub(&pool).await;
    let long = "a".repeat(201);
    for code in [
        "", "a/b", "..", "%2e%2e", "a%2Fb", "a b", "a?b=1", "a#b", "a@b", "a.b", "ü", "a\\b",
        "a\nb", &long,
    ] {
        assert_eq!(
            converting(&transport, code).await.err(),
            Some(ConvertFailure::Refused),
            "{code:?}"
        );
    }
    assert_eq!(stub.paths(), Vec::<String>::new(), "nothing was sent");

    let longest = "a".repeat(200);
    let path = format!("/app-manifests/{longest}/conversions");
    stub.reply(&path, reply(201, &[], CONVERTED));
    assert!(
        converting(&transport, &longest).await.is_ok(),
        "200 is allowed"
    );
    assert_eq!(stub.paths(), [path]);
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_039_each_reply_maps_to_its_failure(pool: PgPool) {
    let (stub, transport) = stub(&pool).await;
    let in_a_minute = start() + TimeDelta::seconds(60);
    let cases = [
        (reply(404, &[], "{}"), ConvertFailure::Refused),
        (reply(422, &[], "{}"), ConvertFailure::Refused),
        (reply(401, &[], "{}"), ConvertFailure::Refused),
        (
            reply(429, &[("retry-after", "60")], "{}"),
            ConvertFailure::RateLimited(in_a_minute),
        ),
        (
            reply(403, &[("x-ratelimit-remaining", "0")], "{}"),
            ConvertFailure::RateLimited(in_a_minute),
        ),
        (reply(200, &[], CONVERTED), ConvertFailure::Unusable),
        (reply(201, &[], "{\"id\":1}"), ConvertFailure::Unusable),
        (reply(201, &[], "not json"), ConvertFailure::Unusable),
    ];
    for (answer, failure) in cases {
        let status = answer.status;
        stub.with(|s| s.seen.clear());
        stub.reply(PATH, answer);
        assert_eq!(
            converting(&transport, CODE).await.err(),
            Some(failure),
            "{status}"
        );
        assert_eq!(stub.paths(), [PATH], "{status}: one request, no second try");
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_039_a_reply_past_the_cap_is_unusable(pool: PgPool) {
    let (stub, transport) = stub(&pool).await;
    stub.reply(PATH, reply(201, &[], &" ".repeat(BODY_CAP + 1)));
    assert_eq!(
        converting(&transport, CODE).await.err(),
        Some(ConvertFailure::Unusable)
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_039_a_5xx_is_tried_once_more_after_the_wait_then_fails(pool: PgPool) {
    let (stub, transport) = stub(&pool).await;
    stub.reply(PATH, reply(502, &[], "{}"));
    let wait = Duration::from_millis(50);
    let began = Instant::now();
    let outcome = convert(
        &transport,
        &provider_github::DECLARATION,
        CODE,
        (start(), wait),
    )
    .await;
    assert!(began.elapsed() >= wait, "the second try waited");
    assert_eq!(outcome.err(), Some(ConvertFailure::Unreachable));
    assert_eq!(stub.paths(), [PATH, PATH], "two requests");
}

#[tokio::test]
async fn prj_039_an_unreachable_source_is_unreachable() {
    assert_eq!(
        converting(&unreachable(), CODE).await.err(),
        Some(ConvertFailure::Unreachable)
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_039_a_source_that_cannot_make_an_app_is_not_registrable(pool: PgPool) {
    let (stub, transport) = stub(&pool).await;
    let mut declaration: Declaration = provider_github::DECLARATION;
    if let Acquisition::TokenPull(Minting::AppKey(app)) = &mut declaration.acquisition {
        app.registration = None;
    }
    let outcome = convert(&transport, &declaration, CODE, (start(), Duration::ZERO)).await;
    assert_eq!(outcome.err(), Some(ConvertFailure::NotRegistrable));
    assert_eq!(stub.paths(), Vec::<String>::new());
}
