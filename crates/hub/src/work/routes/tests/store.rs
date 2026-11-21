//! Adding, checking and removing a route, and its certificate, against PostgreSQL
//! (SF-NEW-routepublish, SF-NEW-routetarget, SF-NEW-routetoken, Section 12.5).

use super::support::{
    DOCKER_HOST, FakeDns, FakeIssuer, HUB, KEY_PEM, application, device, new_route, now, settings,
};
use crate::{
    fleet::credentials::test_vault,
    work::routes::{
        self, Error, Ordering, Waits,
        certificates::IssueError,
        proof::{Proof, record_value},
    },
};

async fn add(pool: &sqlx::PgPool, name: &str, port: i64) -> Result<routes::Added, Error> {
    let mut conn = pool.acquire().await.unwrap();
    routes::add(
        (pool, &settings()),
        &mut conn,
        &new_route(name, port),
        now(),
    )
    .await
}

async fn give_consent(pool: &sqlx::PgPool) {
    sqlx::query("UPDATE routes.route SET machine_consent = true")
        .execute(pool)
        .await
        .unwrap();
}

db_test! {
    async fn an_added_name_waits_for_its_proof_with_its_target_fixed(pool: PgPool) {
        let added = add(&pool, "Photos.Example.com", 3000).await.unwrap();
        let route = added.route;
        assert_eq!(route.name.as_str(), "photos.example.com");
        assert_eq!(route.proof, Proof::Waiting);
        assert_eq!(route.address, Some("10.10.0.21:3000".parse().unwrap()));
        assert_eq!(route.token.len(), 64);
        assert_eq!(route.waits(now()), Some(Waits::Proof));
        assert!(!added.same_site_as_hub);
        assert_eq!(routes::list(&pool, Some(application())).await.unwrap(), [route]);
    }
}

db_test! {
    async fn each_refused_add_stores_nothing(pool: PgPool) {
        add(&pool, "photos.example.com", 3000).await.unwrap();
        assert!(matches!(add(&pool, "photos.example.com", 3000).await, Err(Error::NameTaken)));
        assert!(matches!(add(&pool, "admin.example.com", 22).await, Err(Error::Target(_))));
        assert!(matches!(add(&pool, "*.example.com", 3000).await, Err(Error::Name(_))));
        assert!(matches!(add(&pool, "photos.anima.ing", 3000).await, Err(Error::Name(_))));
        assert!(matches!(add(&pool, "other.example.com", 3001).await, Err(Error::OtherTarget)));
        let mut conn = pool.acquire().await.unwrap();
        let engine = routes::NewRoute {
            holds_engine_socket: true,
            ..new_route("portainer.example.com", 9000)
        };
        let refused = routes::add((&pool, &settings()), &mut conn, &engine, now()).await;
        assert!(matches!(refused, Err(Error::EngineSocket)));
        assert_eq!(routes::list(&pool, None).await.unwrap().len(), 1);
    }
}

db_test! {
    async fn a_device_on_a_refused_address_is_never_a_target(pool: PgPool) {
        for (n, host) in ["127.0.0.1", "169.254.169.254"].into_iter().enumerate() {
            sqlx::query("UPDATE devices.devices SET host = $1 WHERE id = $2::uuid")
                .bind(host)
                .bind(DOCKER_HOST)
                .execute(&pool)
                .await
                .unwrap();
            let mut conn = pool.acquire().await.unwrap();
            let new = new_route(&format!("app{n}.example.com"), 3000);
            let refused = routes::add((&pool, &settings()), &mut conn, &new, now()).await;
            assert!(matches!(refused, Err(Error::Unresolved | Error::Target(_))), "{host}");
            assert_eq!(routes::list(&pool, None).await.unwrap().len(), 0);
        }
    }
}

db_test! {
    async fn the_check_proves_a_name_and_one_failed_read_keeps_it(pool: PgPool) {
        let route = add(&pool, "photos.example.com", 3000).await.unwrap().route;
        let dns = FakeDns::default();
        let mut conn = pool.acquire().await.unwrap();
        let checked = routes::check(&mut conn, (&dns, &settings()), route.id, now()).await.unwrap();
        assert_eq!(checked.proof, Proof::Waiting);
        dns.set_txt("_anima.photos.example.com", &[record_value(HUB, &route.token)]);
        let checked = routes::check(&mut conn, (&dns, &settings()), route.id, now()).await.unwrap();
        assert_eq!(checked.proof, Proof::Proven);
        dns.fail("_anima.photos.example.com");
        let checked = routes::check(&mut conn, (&dns, &settings()), route.id, now()).await.unwrap();
        assert_eq!((checked.proof, checked.failed_reads), (Proof::Proven, 1));
        assert_eq!(routes::get(&pool, route.id).await.unwrap().proof, Proof::Proven);
    }
}

db_test! {
    async fn a_certificate_waits_for_the_proof_and_its_key_is_stored_sealed(pool: PgPool) {
        let route = add(&pool, "photos.example.com", 3000).await.unwrap().route;
        let dns = FakeDns::default();
        let vault = test_vault();
        let issuer = FakeIssuer { refuse: false };
        let ordering = Ordering { db: &pool, vault: &vault, resolver: &dns, issuer: &issuer };
        let early = routes::order_certificate(ordering, route.id, now()).await;
        assert!(matches!(early, Err(Error::NotServing(Waits::Proof))));

        dns.set_txt("_anima.photos.example.com", &[record_value(HUB, &route.token)]);
        let mut conn = pool.acquire().await.unwrap();
        routes::check(&mut conn, (&dns, &settings()), route.id, now()).await.unwrap();
        assert!(routes::order_certificate(ordering, route.id, now()).await.unwrap());
        assert!(!routes::order_certificate(ordering, route.id, now()).await.unwrap(), "not again");

        let (sealed,): (Vec<u8>,) = sqlx::query_as("SELECT sealed_key FROM routes.certificate")
            .fetch_one(&pool)
            .await
            .unwrap();
        let plain = KEY_PEM.as_bytes();
        assert!(!sealed.windows(plain.len()).any(|part| part == plain));

        let stored = routes::get(&pool, route.id).await.unwrap();
        assert_eq!(stored.waits(now()), Some(Waits::MachineConsent));
        assert!(routes::served(&pool, &vault, now()).await.unwrap().is_empty());
        give_consent(&pool).await;
        let served = routes::served(&pool, &vault, now()).await.unwrap();
        assert_eq!(served.len(), 1);
        assert_eq!(served[0].key_pem.expose(), plain);
    }
}

db_test! {
    async fn a_failed_order_makes_the_next_one_wait(pool: PgPool) {
        let route = add(&pool, "photos.example.com", 3000).await.unwrap().route;
        let dns = FakeDns::default();
        dns.set_txt("_anima.photos.example.com", &[record_value(HUB, &route.token)]);
        let mut conn = pool.acquire().await.unwrap();
        routes::check(&mut conn, (&dns, &settings()), route.id, now()).await.unwrap();
        let vault = test_vault();
        let refusing = FakeIssuer { refuse: true };
        let ordering = Ordering { db: &pool, vault: &vault, resolver: &dns, issuer: &refusing };
        let failed = routes::order_certificate(ordering, route.id, now()).await;
        assert!(matches!(failed, Err(Error::Issue(IssueError::Refused))));
        let giving = FakeIssuer { refuse: false };
        let ordering = Ordering { issuer: &giving, ..ordering };
        assert!(!routes::order_certificate(ordering, route.id, now()).await.unwrap());
        let later = now() + chrono::TimeDelta::hours(1);
        assert!(routes::order_certificate(ordering, route.id, later).await.unwrap());
    }
}

db_test! {
    async fn a_removed_route_takes_its_certificate_and_counts(pool: PgPool) {
        let route = add(&pool, "photos.example.com", 3000).await.unwrap().route;
        let traffic = routes::Traffic { requests: 2, bytes_in: 10, bytes_out: 20, refused: 1 };
        routes::count(&pool, "photos.example.com", traffic, now()).await.unwrap();
        routes::count(&pool, "photos.example.com", traffic, now()).await.unwrap();
        assert_eq!(routes::get(&pool, route.id).await.unwrap().traffic.requests, 4);
        let mut conn = pool.acquire().await.unwrap();
        let (removed_from, name) = routes::remove(&mut conn, route.id).await.unwrap();
        assert_eq!((removed_from, name.as_str()), (device(DOCKER_HOST), "photos.example.com"));
        assert!(matches!(routes::remove(&mut conn, route.id).await, Err(Error::NotFound)));
        let (left,): (i64,) = sqlx::query_as("SELECT count(*) FROM routes.traffic")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(left, 0);
    }
}
