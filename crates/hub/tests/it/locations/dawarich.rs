//! The Dawarich pull through the hub, against a loopback stub of the server (R-NEW-dawarichkey; K-E1
//! Section 7b; K-E4 Section 11).

use std::{sync::Arc, time::Duration};

use anima_hub::{
    clock::{Clock, FixedClock},
    engine::{
        files::Store,
        locations::{Seal, dawarich_intake},
        sealing_keys::SealingKeys,
        sources::{self, Reach, Registry, Scheduling, Transport, Trigger},
    },
    fleet::{events::Bus, targets::AddressPolicy},
    ids::SourceAccountId,
    schema::Dependencies,
};
use serde_json::{Value, json};
use sqlx::PgPool;

use crate::{
    hub::{Hub, dependencies, error, set_operator, start, test_vault},
    sources::{
        stub::{Reply, Stub},
        unreachable,
    },
};

const KEY: &str = "dw-synthetic-api-key";
const CONNECT: &str = "mutation($input: DawarichInput!) { connectDawarich(input: $input) \
    { id sourceKey label instanceKey credentialStored } }";
const POINTS: &str = "/api/v1/points";

struct Kit {
    pool: PgPool,
    hub: Hub,
    token: String,
    stub: Stub,
    reach: Reach,
    _keys: tempfile::TempDir,
}

impl Kit {
    async fn new(pool: &PgPool) -> Self {
        let stub = Stub::start(pool).await;
        let loopback = AddressPolicy::default().allowing_loopback();
        let transport = Transport::to_stub(stub.base.clone(), loopback).unwrap();
        Self::with(pool, stub, transport).await
    }

    async fn with(pool: &PgPool, stub: Stub, transport: Transport) -> Self {
        set_operator(pool).await;
        let keys = tempfile::tempdir().unwrap();
        let sealing = SealingKeys::open(Some(keys.path()), b"credential").unwrap();
        let intake = dawarich_intake(Seal::new(sealing.clone()));
        let registry = Registry::new(vec![provider_dawarich::DECLARATION], vec![intake]);
        let reach = Reach::new(registry, transport);
        let clock: Arc<dyn Clock> = Arc::new(FixedClock(start()));
        let parts = Dependencies {
            sources: reach.clone(),
            files: Store::in_folder(None, sealing),
            ..dependencies(pool, clock, Bus::new(16))
        };
        let hub = Hub::serving(pool, parts);
        let token = hub.token().await;
        Self {
            pool: pool.clone(),
            hub,
            token,
            stub,
            reach,
            _keys: keys,
        }
    }

    fn base(&self) -> String {
        self.stub.base.as_str().trim_end_matches('/').to_owned()
    }

    async fn connect(&self, base: &str, key: &str) -> Value {
        let input = json!({ "input": { "label": "Home", "baseUrl": base, "apiKey": key } });
        self.hub.post(Some(&self.token), CONNECT, input).await
    }

    fn points(&self, points: &Value) {
        let reply = Reply {
            status: 200,
            headers: Vec::new(),
            body: points.to_string(),
        };
        self.stub.reply(POINTS, reply);
    }

    async fn run(&self, id: SourceAccountId) {
        let clock: Arc<dyn Clock> = Arc::new(FixedClock(start()));
        let hub = (self.pool.clone(), Bus::new(16), clock);
        let scheduling = Scheduling::new(hub, test_vault(), self.reach.clone());
        sources::run(&scheduling, id, Trigger::Manual)
            .await
            .unwrap();
    }

    async fn stored(&self) -> i64 {
        sqlx::query_scalar("select count(*) from locations.sample where provider = 'dawarich'")
            .fetch_one(&self.pool)
            .await
            .unwrap()
    }

    /// Waits for the first run, which the connection starts, to store `count` samples.
    async fn wait_for(&self, count: i64) {
        for _ in 0..100 {
            if self.stored().await == count {
                return;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        panic!(
            "the run stored {} samples, not {count}",
            self.stored().await
        );
    }
}

fn two_points() -> Value {
    json!([
        { "id": 41, "timestamp": 1_904_208_000, "latitude": "52.1", "longitude": "4.2",
          "accuracy": 12, "velocity": "36" },
        { "id": 42, "timestamp": 1_904_208_060, "latitude": "52.2", "longitude": "4.3" },
    ])
}

#[sqlx::test(migrations = "./migrations")]
async fn dawarich_key_a_connected_dawarich_account_pulls_its_points_one_time(pool: PgPool) {
    let kit = Kit::new(&pool).await;
    kit.points(&two_points());
    let connected = kit.connect(&kit.base(), KEY).await;
    let account = &connected["data"]["connectDawarich"];
    assert_eq!(account["sourceKey"], "dawarich", "{connected}");
    assert_eq!(account["instanceKey"], kit.base().as_str());
    assert_eq!(account["credentialStored"], true);
    let id: SourceAccountId = account["id"].as_str().unwrap().parse().unwrap();

    kit.wait_for(2).await;
    let heard = kit.stub.with(|script| script.heard.clone());
    assert!(heard.iter().all(|request| request.path.starts_with(POINTS)));
    for request in &heard {
        assert_eq!(
            request.header("authorization"),
            [format!("Bearer {KEY}").as_bytes()]
        );
    }
    let pull = heard.last().unwrap();
    assert!(
        pull.path.contains("order=asc") && pull.path.contains("per_page=1000"),
        "{}",
        pull.path
    );

    let rows: Vec<(String, String, bool)> = sqlx::query_as(
        "select account, sample_key, observed_at is not null from locations.sample \
         order by sample_key",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        rows,
        [
            (id.to_string(), "41".to_owned(), true),
            (id.to_string(), "42".to_owned(), true)
        ]
    );
    // The next run reads the last point again and stores nothing new.
    kit.run(id).await;
    assert_eq!(kit.stored().await, 2);
    let run: (String, i32, i32) = sqlx::query_as(
        "select outcome, inserted, unchanged from sources.sync_runs order by started_at desc, \
         id desc limit 1",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(run, ("ok".to_owned(), 0, 2));

    let logs: String = sqlx::query_scalar(
        "select coalesce(string_agg(t, ' '), '') from (select message as t from events.event_log \
         union all select change::text from events.audit_log) logs",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(!logs.contains(KEY) && !logs.contains("52.1"), "{logs}");

    let removed = kit
        .hub
        .post(
            Some(&kit.token),
            "mutation($id: ID!) { removeSourceAccount(id: $id, deleteRecords: true) }",
            json!({ "id": id.to_string() }),
        )
        .await;
    assert!(removed["errors"].is_null(), "{removed}");
    assert_eq!(kit.stored().await, 0, "the samples go with the account");
}

#[sqlx::test(migrations = "./migrations")]
async fn dawarich_key_a_bad_address_or_a_refused_key_stores_nothing(pool: PgPool) {
    let kit = Kit::new(&pool).await;
    kit.points(&two_points());
    for base in [
        format!("{}/dawarich", kit.base()),
        format!("{}?x=1", kit.base()),
        "ftp://dawarich.example".to_owned(),
        "not an address".to_owned(),
    ] {
        let refused = kit.connect(&base, KEY).await;
        assert_eq!(error(&refused).1, "VALIDATION", "{base}");
        assert_eq!(
            refused["errors"][0]["extensions"]["fields"][0]["field"], "baseUrl",
            "{base}"
        );
    }
    // The address of cloud metadata is one that the hub never connects to.
    let metadata = kit.connect("http://169.254.169.254", KEY).await;
    assert_eq!(
        metadata["errors"][0]["extensions"]["fields"][0]["message"],
        "The hub does not connect to this address."
    );
    let spaced = kit.connect(&kit.base(), "two words").await;
    assert_eq!(
        spaced["errors"][0]["extensions"]["fields"][0]["field"],
        "apiKey"
    );
    assert!(
        kit.stub.paths().is_empty(),
        "nothing was sent: {:?}",
        kit.stub.paths()
    );

    let unauthorized = Reply {
        status: 401,
        headers: Vec::new(),
        body: "{}".to_owned(),
    };
    kit.stub.reply(POINTS, unauthorized);
    let refused = kit.connect(&kit.base(), KEY).await;
    assert_eq!(
        refused["errors"][0]["extensions"]["fields"][0]["field"],
        "apiKey"
    );
    let accounts: i64 = sqlx::query_scalar("select count(*) from sources.accounts")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(accounts, 0);
}

#[sqlx::test(migrations = "./migrations")]
async fn dawarich_key_a_server_that_does_not_answer_is_unavailable_in_plain_words(pool: PgPool) {
    let stub = Stub::start(&pool).await;
    let kit = Kit::with(&pool, stub, unreachable()).await;
    let refused = kit.connect("http://127.0.0.1:9", KEY).await;
    assert_eq!(error(&refused).1, "UNAVAILABLE", "{refused}");
    assert!(!refused.to_string().contains(KEY));
}
