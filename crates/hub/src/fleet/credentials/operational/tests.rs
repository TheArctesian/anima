//! Tests of the operational keys against a real database, grouped by concept; the shared
//! fixtures live here. The pure parts are tested beside them in `lifecycle` and `rules` tests.

mod disclosure;
mod lifecycle;
mod pure;
mod retrieval;

use std::net::{IpAddr, Ipv4Addr};

use chrono::{DateTime, DurationRound as _, TimeDelta, Utc};
use daemon_core::wire::FetchedKey;
use sqlx::PgPool;

use super::{Ending, Grant, Metadata, NewKey, OperationalKey};
use crate::{
    fleet::{
        credentials::{Secret, Vault, test_vault},
        events::{Actor, Bus, ClientAddr, Operator, Trail},
    },
    ids::DeviceId,
};

/// A value no metadata, log or answer may hold.
const VALUE: &str = "sk-live-5ecret-0perational-Value";
const NEXT_VALUE: &str = "sk-live-n3xt-0perational-Value";

/// pve-01 of the devices fixture.
fn pve() -> DeviceId {
    "0190a000-0000-7000-8002-000000000001".parse().unwrap()
}

/// docker-host of the devices fixture.
fn docker_host() -> DeviceId {
    "0190a000-0000-7000-8002-000000000002".parse().unwrap()
}

fn client() -> ClientAddr {
    ClientAddr(IpAddr::V4(Ipv4Addr::new(100, 64, 0, 7)))
}

async fn database_now(pool: &PgPool) -> DateTime<Utc> {
    let now: DateTime<Utc> = sqlx::query_scalar("SELECT pg_catalog.now()")
        .fetch_one(pool)
        .await
        .unwrap();
    now.duration_trunc(TimeDelta::seconds(1)).unwrap()
}

fn metadata() -> Metadata {
    Metadata {
        provider: "Cloudflare".to_owned(),
        purpose: "DNS updates".to_owned(),
        instructions: "Make a new token under API Tokens.".to_owned(),
        provider_link: Some("https://dash.cloudflare.com/profile/api-tokens".to_owned()),
        rotation_days: Some(90),
        expires_at: None,
    }
}

/// The operator's changes of the tests, on one bus.
struct Desk {
    pool: PgPool,
    vault: Vault,
    bus: Bus,
}

impl Desk {
    fn new(pool: &PgPool) -> Self {
        Self {
            pool: pool.clone(),
            vault: test_vault(),
            bus: Bus::new(64),
        }
    }

    fn trail(&self, now: DateTime<Utc>) -> Trail<'_> {
        let operator = Operator {
            id: uuid::Uuid::from_u128(1).into(),
            name: "operator".to_owned(),
            actor: Actor::hub(),
        };
        Trail::new(&self.bus, (operator, client()), now)
    }

    async fn create(&self, name: &str, metadata: Metadata, now: DateTime<Utc>) -> OperationalKey {
        let new = NewKey {
            name: name.to_owned(),
            metadata,
            value: Secret::from(VALUE.to_owned()),
        };
        let mut tx = self.pool.begin().await.unwrap();
        let key = super::create(&mut tx, &self.vault, new, self.trail(now))
            .await
            .unwrap();
        tx.commit().await.unwrap();
        key
    }

    async fn rotate(&self, name: &str, value: &str, now: DateTime<Utc>) -> Ending {
        let mut tx = self.pool.begin().await.unwrap();
        let asked = (name, Secret::from(value.to_owned()), false);
        let ending = super::rotate(&mut tx, &self.vault, asked, self.trail(now))
            .await
            .unwrap();
        tx.commit().await.unwrap();
        ending
    }

    async fn revoke(&self, name: &str, now: DateTime<Utc>) -> Ending {
        let mut tx = self.pool.begin().await.unwrap();
        let ending = super::revoke(&mut tx, name, self.trail(now)).await.unwrap();
        tx.commit().await.unwrap();
        ending
    }

    async fn grant(&self, name: &str, device: DeviceId, now: DateTime<Utc>) -> OperationalKey {
        let grant = Grant {
            name: name.to_owned(),
            device,
            workload: "ddns".to_owned(),
        };
        let mut tx = self.pool.begin().await.unwrap();
        let key = super::grant(&mut tx, grant, self.trail(now)).await.unwrap();
        tx.commit().await.unwrap();
        key
    }

    async fn fetch(&self, device: DeviceId, name: &str, now: DateTime<Utc>) -> Option<FetchedKey> {
        super::fetch(&self.pool, &self.vault, (device, name), (now, client()))
            .await
            .unwrap()
    }
}

/// The value of a fetched key as text.
fn text_of(fetched: &FetchedKey) -> &str {
    std::str::from_utf8(fetched.value.expose()).unwrap()
}
