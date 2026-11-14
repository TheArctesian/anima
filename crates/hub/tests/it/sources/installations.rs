//! PRJ-039: the App's installations are read signed in as the App, an installation id reaches a
//! path only as 1 to 20 digits, and each reply maps to one outcome.

use anima_hub::engine::sources::acquisition::{
    app_key::AppSigner,
    installations::{Lookup, installation, installations},
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use ring::rand::SystemRandom;
use serde_json::Value;
use source_exchange::Installation;
use sqlx::PgPool;

use super::{
    TEST_KEY,
    conversion::{reply, stub},
};
use crate::hub::start;

const ONE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../providers/github/tests/fixtures/installation.json"
));
const SUSPENDED: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../providers/github/tests/fixtures/installation_suspended.json"
));
const LIST: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../providers/github/tests/fixtures/installations.json"
));
const APP: u64 = 1_234_567;

fn signer() -> AppSigner {
    AppSigner::read(APP, TEST_KEY.as_bytes()).unwrap()
}

async fn one(transport: &anima_hub::engine::sources::Transport, id: &str) -> Lookup<Installation> {
    let signer = signer();
    let random = SystemRandom::new();
    installation(
        transport,
        &provider_github::DECLARATION,
        (&signer, id),
        (start(), &random),
    )
    .await
}

/// The `iss` claim of a `Bearer <JWT>` header value.
fn issuer(header: &[u8]) -> Value {
    let jwt = std::str::from_utf8(header)
        .unwrap()
        .strip_prefix("Bearer ")
        .unwrap();
    let claims = jwt.split('.').nth(1).unwrap();
    let claims: Value = serde_json::from_slice(&URL_SAFE_NO_PAD.decode(claims).unwrap()).unwrap();
    claims["iss"].clone()
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_039_an_installation_is_read_with_the_app_token(pool: PgPool) {
    let (stub, transport) = stub(&pool).await;
    stub.reply("/app/installations/4321", reply(200, &[], ONE));
    let found = Installation {
        id: 4321,
        app_id: APP,
        account_id: 9001,
        suspended: false,
    };
    assert_eq!(one(&transport, "4321").await, Lookup::Found(found));
    let heard = stub.with(|s| s.heard.clone());
    let get = &heard[0];
    assert_eq!(
        (get.method.as_str(), get.path.as_str()),
        ("GET", "/app/installations/4321")
    );
    assert_eq!(
        issuer(get.header("authorization")[0]),
        APP,
        "the App's own JWT"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_039_a_suspended_installation_is_reported_as_suspended(pool: PgPool) {
    let (stub, transport) = stub(&pool).await;
    stub.reply("/app/installations/4322", reply(200, &[], SUSPENDED));
    let Lookup::Found(found) = one(&transport, "4322").await else {
        panic!("not found");
    };
    assert!(found.suspended);
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_039_an_id_that_is_not_digits_makes_no_request(pool: PgPool) {
    let (stub, transport) = stub(&pool).await;
    for id in [
        "",
        "12a",
        "1/2",
        "..",
        "%2e",
        "-1",
        " 1",
        "123456789012345678901",
    ] {
        assert_eq!(one(&transport, id).await, Lookup::NotFound, "{id:?}");
    }
    assert_eq!(stub.paths(), Vec::<String>::new());
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_039_each_reply_about_an_installation_maps_to_its_outcome(pool: PgPool) {
    let (stub, transport) = stub(&pool).await;
    assert_eq!(
        one(&transport, "1").await,
        Lookup::NotFound,
        "the stub's 404"
    );
    for (answer, outcome) in [
        (reply(500, &[], "{}"), Lookup::Unreachable),
        (
            reply(429, &[("retry-after", "5")], "{}"),
            Lookup::Unreachable,
        ),
        (reply(401, &[], "{}"), Lookup::Unusable),
        (reply(200, &[], "not json"), Lookup::Unusable),
    ] {
        let status = answer.status;
        stub.reply("/app/installations/1", answer);
        assert_eq!(one(&transport, "1").await, outcome, "{status}");
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_039_the_list_asks_for_100_with_the_app_token(pool: PgPool) {
    let (stub, transport) = stub(&pool).await;
    stub.reply("/app/installations", reply(200, &[], LIST));
    let signer = signer();
    let random = SystemRandom::new();
    let listed = installations(
        &transport,
        &provider_github::DECLARATION,
        &signer,
        (start(), &random),
    )
    .await;
    let Lookup::Found(all) = listed else {
        panic!("no list");
    };
    assert_eq!(
        all.iter()
            .map(|i| (i.id, i.account_id, i.suspended))
            .collect::<Vec<_>>(),
        [(4321, 9001, false), (4322, 9002, true)]
    );
    let heard = stub.with(|s| s.heard.clone());
    assert_eq!(heard[0].path, "/app/installations?per_page=100");
    assert_eq!(issuer(heard[0].header("authorization")[0]), APP);
}
