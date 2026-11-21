//! The one interface of the proxy (SF-NEW-routeproxyexit, SF-NEW-routelinkshare): a route is
//! reached only by the identifier of its application, at the target fixed in the hub's record.

use std::{
    io,
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use tokio::io::{AsyncBufReadExt as _, AsyncReadExt as _, AsyncWriteExt as _, BufReader, duplex};

use super::support::{FakeDns, FakeIssuer, HUB, application, device, new_route, now, settings};
use crate::{
    clock::FixedClock,
    fleet::credentials::test_vault,
    work::routes::{
        self, Ordering,
        door::{Connector, Door, IDLE_DEFAULT, LinkStreams, Stream, answer},
        proof::record_value,
    },
};

/// A connector that records each address and answers each connection with `hello`.
#[derive(Default)]
pub struct FakeConnector {
    reached: Mutex<Vec<SocketAddr>>,
}

#[async_trait]
impl Connector for FakeConnector {
    async fn connect(&self, address: SocketAddr) -> io::Result<Box<dyn Stream>> {
        self.reached.lock().unwrap().push(address);
        let (mut machine, hub) = duplex(64);
        machine.write_all(b"hello").await?;
        Ok(Box::new(hub))
    }
}

/// Adds `photos.example.com` to port 3000 of `docker-host` and makes it serve.
pub async fn serving(pool: &sqlx::PgPool) {
    let mut conn = pool.acquire().await.unwrap();
    let route = routes::add(
        (pool, &settings()),
        &mut conn,
        &new_route("photos.example.com", 3000),
        now(),
    )
    .await
    .unwrap()
    .route;
    let dns = FakeDns::default();
    dns.set_txt(
        "_anima.photos.example.com",
        &[record_value(HUB, &route.token)],
    );
    routes::check(&mut conn, (&dns, &settings()), route.id, now())
        .await
        .unwrap();
    let vault = test_vault();
    let issuer = FakeIssuer { refuse: false };
    let ordering = Ordering {
        db: pool,
        vault: &vault,
        resolver: &dns,
        issuer: &issuer,
    };
    routes::order_certificate(ordering, route.id, now())
        .await
        .unwrap();
    sqlx::query("UPDATE routes.route SET machine_consent = true, application_recorded = true")
        .execute(pool)
        .await
        .unwrap();
}

pub fn door<C: Connector + 'static>(
    pool: &sqlx::PgPool,
    connector: Arc<C>,
    streams: usize,
) -> Door {
    Door {
        db: pool.clone(),
        vault: test_vault(),
        clock: Arc::new(FixedClock(now())),
        connector,
        streams: LinkStreams::new(streams),
        proxy_uid: 0,
        idle: IDLE_DEFAULT,
    }
}

/// Sends one request line and gives the first line of the answer and the bytes after it.
async fn ask(door: &Door, request: &str) -> (String, String) {
    let (proxy, hub) = duplex(1 << 16);
    let (read_half, mut write_half) = tokio::io::split(proxy);
    write_half.write_all(request.as_bytes()).await.unwrap();
    write_half.write_all(b"\n").await.unwrap();
    let served = answer(door, hub);
    let reading = async {
        let mut reader = BufReader::new(read_half);
        let mut line = String::new();
        reader.read_line(&mut line).await.unwrap();
        let mut rest = vec![0_u8; 5];
        if line.trim() == "ok" {
            reader.read_exact(&mut rest).await.unwrap();
        } else {
            rest.clear();
        }
        drop(write_half);
        (line.trim().to_owned(), String::from_utf8(rest).unwrap())
    };
    let (result, answered) = tokio::join!(served, reading);
    if answered.0 != "ok" {
        result.unwrap();
    }
    answered
}

fn connect(application: &str) -> String {
    format!(r#"{{"connect":{{"application":"{application}"}}}}"#)
}

db_test! {
    async fn an_application_is_reached_only_at_the_target_in_the_record(pool: PgPool) {
        serving(&pool).await;
        let connector = Arc::new(FakeConnector::default());
        let door = door(&pool, Arc::clone(&connector), 4);
        let answered = ask(&door, &connect(&application().to_string())).await;
        assert_eq!(answered, ("ok".to_owned(), "hello".to_owned()));
        assert_eq!(*connector.reached.lock().unwrap(), ["10.10.0.21:3000".parse().unwrap()]);
    }
}

db_test! {
    async fn a_device_and_a_port_are_not_a_request_of_the_interface(pool: PgPool) {
        serving(&pool).await;
        let connector = Arc::new(FakeConnector::default());
        let door = door(&pool, Arc::clone(&connector), 4);
        for request in [
            format!("dial {} 22", device(super::support::DOCKER_HOST)),
            r#"{"connect":{"application":"0190a000-0000-7000-8009-000000000001","port":22}}"#.to_owned(),
            r#"{"connect":{"device":"0190a000-0000-7000-8002-000000000002","port":22}}"#.to_owned(),
        ] {
            let (proxy, hub) = duplex(1024);
            let (_read, mut write) = tokio::io::split(proxy);
            write.write_all(request.as_bytes()).await.unwrap();
            write.write_all(b"\n").await.unwrap();
            assert!(answer(&door, hub).await.is_err(), "{request}");
        }
        assert!(connector.reached.lock().unwrap().is_empty());
    }
}

db_test! {
    async fn an_application_with_no_serving_route_is_refused(pool: PgPool) {
        let connector = Arc::new(FakeConnector::default());
        let door = door(&pool, Arc::clone(&connector), 4);
        let mut conn = pool.acquire().await.unwrap();
        routes::add((&pool, &settings()), &mut conn, &new_route("photos.example.com", 3000), now())
            .await
            .unwrap();
        let answered = ask(&door, &connect(&application().to_string())).await;
        assert_eq!(answered.0, "refused", "a route that waits for its proof does not serve");
        let other = ask(&door, &connect("0190a000-0000-7000-8009-0000000000ff")).await;
        assert_eq!(other.0, "refused");
        assert!(connector.reached.lock().unwrap().is_empty());
    }
}

db_test! {
    async fn the_streams_of_one_link_have_a_limit(pool: PgPool) {
        serving(&pool).await;
        let connector = Arc::new(FakeConnector::default());
        let door = door(&pool, connector, 1);
        let held = door.streams.take(device(super::support::DOCKER_HOST)).unwrap();
        let answered = ask(&door, &connect(&application().to_string())).await;
        assert_eq!(answered.0, "refused");
        drop(held);
        let answered = ask(&door, &connect(&application().to_string())).await;
        assert_eq!(answered.0, "ok");
    }
}

db_test! {
    async fn the_table_holds_only_the_names_that_serve(pool: PgPool) {
        let connector = Arc::new(FakeConnector::default());
        let door = door(&pool, connector, 4);
        let (empty, _) = ask(&door, r#""table""#).await;
        assert_eq!(empty, r#"{"names":[]}"#);
        serving(&pool).await;
        let (table, _) = ask(&door, r#""table""#).await;
        let table: anima_route_proxy::wire::Table = serde_json::from_str(&table).unwrap();
        assert_eq!(table.names.len(), 1);
        assert_eq!(table.names[0].name, "photos.example.com");
        assert_eq!(table.names[0].application, uuid::Uuid::from(application()));
    }
}
