//! Pure rule tests and `#[sqlx::test]` store and face tests of web interfaces.

use chrono::{DateTime, Utc};
use sqlx::{PgConnection, PgPool};

use crate::{
    fleet::web_interfaces::{self, Acceptance, Error, Missing, OpenIn, WebInterfaceInput},
    ids::{CheckId, DeviceId, OperatorId, WebInterfaceId},
};

const OPERATOR: &str = "0190a000-0000-7000-8000-000000000001";
const PVE_01: &str = "0190a000-0000-7000-8002-000000000001";
const DOCKER_HOST: &str = "0190a000-0000-7000-8002-000000000002";
/// An HTTP check on `pve-01` in the checks fixture: `https://pve-01.home.lan:8006/`.
const HTTP_CHECK: &str = "0190a000-0000-7000-8003-00000000001a";
const PING_CHECK: &str = "0190a000-0000-7000-8003-000000000001";
const HEX: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

macro_rules! db_test {
    ($(#[$meta:meta])* async fn $name:ident($pool:ident: PgPool) $body:block) => {
        #[sqlx::test(
            migrations = "./migrations",
            fixtures(path = "../../../../tests/fixtures", scripts("devices", "checks"))
        )]
        $(#[$meta])*
        async fn $name($pool: PgPool) $body
    };
}

fn now() -> DateTime<Utc> {
    "2026-11-06T09:00:00Z".parse().unwrap()
}

fn device(id: &str) -> DeviceId {
    id.parse().unwrap()
}

fn check(id: &str) -> CheckId {
    id.parse().unwrap()
}

fn input(url: &str) -> WebInterfaceInput {
    WebInterfaceInput {
        label: Some("Panel".to_owned()),
        url: url.to_owned(),
        open_in: OpenIn::AnimaTab,
    }
}

fn acceptance(fingerprint: &str) -> Acceptance<'_> {
    Acceptance {
        fingerprint,
        operator: OPERATOR.parse::<OperatorId>().unwrap(),
        at: now(),
    }
}

async fn add_operator(pool: &PgPool) {
    sqlx::query(
        "insert into access.operators (id, username, password_hash, created_at) \
         values ($1::text::uuid, 'operator', '$argon2id$x', now())",
    )
    .bind(OPERATOR)
    .execute(pool)
    .await
    .unwrap();
}

async fn count(pool: &PgPool, table: &str) -> i64 {
    sqlx::query_scalar(&format!("select count(*) from web_interfaces.{table}"))
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn make(conn: &mut PgConnection, url: &str) -> WebInterfaceId {
    web_interfaces::create(conn, device(PVE_01), &input(url), now())
        .await
        .unwrap()
        .id
}

mod accepting;
mod fingerprints;
mod pins;
mod store;
mod suggestions;
mod urls;
