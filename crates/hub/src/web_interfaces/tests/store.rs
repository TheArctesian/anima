//! The store and the face against PostgreSQL: reads, writes, pins, cascades, idempotence and
//! the runtime role's privileges (WEB-001, WEB-002, WEB-008, R-016).

use chrono::{DateTime, Utc};
use sqlx::{PgConnection, PgPool};

use crate::{
    ids::{CheckId, DeviceId, OperatorId, WebInterfaceId},
    web_interfaces::{self, Acceptance, Error, Missing, OpenIn, WebInterfaceInput},
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
            fixtures(path = "../../../tests/fixtures", scripts("devices", "checks"))
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

db_test! {
    async fn web_001_create_read_update_delete(pool: PgPool) {
        let mut conn = pool.acquire().await.unwrap();
        let created = web_interfaces::create(&mut conn, device(PVE_01), &input("https://pve.lan:8006/"), now())
            .await
            .unwrap();
        assert_eq!(created.label, "Panel");
        assert_eq!(created.pin, None);

        let read = web_interfaces::for_devices(&pool, &[device(PVE_01), device(DOCKER_HOST)])
            .await
            .unwrap();
        assert_eq!(read.len(), 1, "a device with none is absent");
        assert_eq!(read[&device(PVE_01)], std::slice::from_ref(&created));

        let replaced = WebInterfaceInput {
            label: None,
            url: "https://pve.lan:8006/ui".to_owned(),
            open_in: OpenIn::SystemBrowser,
        };
        let updated = web_interfaces::update(&mut conn, created.id, &replaced).await.unwrap();
        assert_eq!(updated.before, created);
        assert_eq!(updated.after.label, "pve.lan");
        assert_eq!(updated.after.open_in, OpenIn::SystemBrowser);
        let reread = web_interfaces::for_devices(&pool, &[device(PVE_01)]).await.unwrap();
        assert_eq!(reread[&device(PVE_01)], [updated.after]);

        assert_eq!(web_interfaces::delete(&mut conn, created.id).await.unwrap(), device(PVE_01));
        assert!(web_interfaces::for_devices(&pool, &[device(PVE_01)]).await.unwrap().is_empty());
    }
}

db_test! {
    async fn web_001_missing_rows_are_not_found_and_bad_input_is_validation(pool: PgPool) {
        let mut conn = pool.acquire().await.unwrap();
        let ghost = WebInterfaceId::from(uuid::Uuid::from_u128(9));
        let nobody = DeviceId::from(uuid::Uuid::from_u128(9));
        assert!(matches!(
            web_interfaces::create(&mut conn, nobody, &input("http://a.lan/"), now()).await,
            Err(Error::NotFound(Missing::Device(_)))
        ));
        let mut conn = pool.acquire().await.unwrap();
        assert!(matches!(
            web_interfaces::update(&mut conn, ghost, &input("http://a.lan/")).await,
            Err(Error::NotFound(Missing::Interface(_)))
        ));
        assert!(matches!(
            web_interfaces::delete(&mut conn, ghost).await,
            Err(Error::NotFound(Missing::Interface(_)))
        ));
        assert!(matches!(
            web_interfaces::accept_certificate(&mut conn, ghost, acceptance(HEX)).await,
            Err(Error::NotFound(Missing::Interface(_)))
        ));
        let Err(Error::Validation(errors)) =
            web_interfaces::create(&mut conn, device(PVE_01), &input("http://u:p@a.lan/"), now()).await
        else {
            panic!("expected validation");
        };
        assert_eq!(errors[0].field, "url");
    }
}

db_test! {
    async fn web_008_a_pin_survives_a_path_change_and_is_dropped_by_an_origin_change(pool: PgPool) {
        add_operator(&pool).await;
        let mut conn = pool.acquire().await.unwrap();
        let id = make(&mut conn, "https://pve.lan:8006/").await;
        let pinned = web_interfaces::accept_certificate(&mut conn, id, acceptance(&HEX.to_uppercase()))
            .await
            .unwrap();
        let pin = pinned.pin.clone().unwrap();
        assert_eq!(pin.fingerprint.as_str(), HEX);

        let moved = web_interfaces::update(&mut conn, id, &input("https://pve.lan:8006/other"))
            .await
            .unwrap();
        assert_eq!(moved.after.pin, Some(pin.clone()));
        assert_eq!(count(&pool, "pins").await, 1);

        for url in ["https://pve.lan:9000/", "https://pve2.lan:8006/", "http://pve.lan:8006/"] {
            let id = make(&mut conn, "https://pve.lan:8006/").await;
            web_interfaces::accept_certificate(&mut conn, id, acceptance(HEX)).await.unwrap();
            let changed = web_interfaces::update(&mut conn, id, &input(url)).await.unwrap();
            assert!(changed.before.pin.is_some());
            assert_eq!(changed.after.pin, None, "{url}");
            let stored = web_interfaces::for_devices(&pool, &[device(PVE_01)]).await.unwrap();
            let found = stored[&device(PVE_01)].iter().find(|w| w.id == id).unwrap();
            assert_eq!(found.pin, None);
        }
    }
}

db_test! {
    async fn web_008_accepting_again_replaces_the_pin_and_bad_requests_are_refused(pool: PgPool) {
        add_operator(&pool).await;
        let mut conn = pool.acquire().await.unwrap();
        let secure = make(&mut conn, "https://pve.lan/").await;
        let plain = make(&mut conn, "http://pve.lan/").await;
        web_interfaces::accept_certificate(&mut conn, secure, acceptance(HEX)).await.unwrap();
        let other = "ab".repeat(32);
        let again = web_interfaces::accept_certificate(&mut conn, secure, acceptance(&other))
            .await
            .unwrap();
        assert_eq!(again.pin.unwrap().fingerprint.as_str(), other);
        assert_eq!(count(&pool, "pins").await, 1);

        for (id, text) in [(secure, "nope"), (plain, HEX)] {
            let Err(Error::Validation(errors)) =
                web_interfaces::accept_certificate(&mut conn, id, acceptance(text)).await
            else {
                panic!("expected validation");
            };
            assert_eq!(errors[0].field, "fingerprintSha256");
        }
        assert_eq!(count(&pool, "pins").await, 1);
    }
}

db_test! {
    async fn web_001_deleting_the_device_removes_its_interfaces_and_pins(pool: PgPool) {
        add_operator(&pool).await;
        let mut conn = pool.acquire().await.unwrap();
        let id = make(&mut conn, "https://pve.lan/").await;
        web_interfaces::accept_certificate(&mut conn, id, acceptance(HEX)).await.unwrap();
        sqlx::query("delete from devices.devices where id = $1::text::uuid")
            .bind(PVE_01)
            .execute(&pool)
            .await
            .unwrap();
        assert_eq!(count(&pool, "interfaces").await, 0);
        assert_eq!(count(&pool, "pins").await, 0);
    }
}

db_test! {
    async fn web_002_accepting_a_suggestion_creates_one_interface_and_is_idempotent(pool: PgPool) {
        let mut conn = pool.acquire().await.unwrap();
        let first = web_interfaces::accept_suggestion(&pool, &mut conn, check(HTTP_CHECK), now())
            .await
            .unwrap();
        assert!(first.created);
        assert_eq!(first.interface.url, "https://pve-01.home.lan:8006/");
        assert_eq!(first.interface.label, "pve-01.home.lan");
        assert_eq!(first.interface.open_in, OpenIn::AnimaTab);
        let second = web_interfaces::accept_suggestion(&pool, &mut conn, check(HTTP_CHECK), now())
            .await
            .unwrap();
        assert!(!second.created);
        assert_eq!(second.interface, first.interface);
        assert_eq!(count(&pool, "interfaces").await, 1);
        // The accepted check is no longer offered.
        let offered = web_interfaces::suggestions(&pool, &[device(PVE_01)]).await.unwrap();
        assert!(offered.is_empty());
    }
}

db_test! {
    async fn web_002_accept_reuses_an_interface_at_the_same_parsed_url(pool: PgPool) {
        let mut conn = pool.acquire().await.unwrap();
        let typed = make(&mut conn, "HTTPS://PVE-01.home.lan:8006").await;
        let accepted = web_interfaces::accept_suggestion(&pool, &mut conn, check(HTTP_CHECK), now())
            .await
            .unwrap();
        assert!(!accepted.created);
        assert_eq!(accepted.interface.id, typed);
    }
}

db_test! {
    async fn web_002_dismiss_hides_the_suggestion_is_idempotent_and_needs_an_http_check(pool: PgPool) {
        let mut conn = pool.acquire().await.unwrap();
        let before = web_interfaces::suggestions(&pool, &[device(PVE_01)]).await.unwrap();
        assert_eq!(before[&device(PVE_01)].len(), 1);
        assert_eq!(before[&device(PVE_01)][0].check, check(HTTP_CHECK));

        for _ in 0..2 {
            let owner = web_interfaces::dismiss_suggestion(&pool, &mut conn, check(HTTP_CHECK), now())
                .await
                .unwrap();
            assert_eq!(owner, device(PVE_01));
        }
        assert_eq!(count(&pool, "dismissed").await, 1);
        assert!(web_interfaces::suggestions(&pool, &[device(PVE_01)]).await.unwrap().is_empty());

        for id in [check(PING_CHECK), CheckId::from(uuid::Uuid::from_u128(9))] {
            assert!(matches!(
                web_interfaces::dismiss_suggestion(&pool, &mut conn, id, now()).await,
                Err(Error::NotFound(Missing::Check(_)))
            ));
            assert!(matches!(
                web_interfaces::accept_suggestion(&pool, &mut conn, id, now()).await,
                Err(Error::NotFound(Missing::Check(_)))
            ));
        }
    }
}

db_test! {
    async fn web_002_deleting_the_check_forgets_the_dismissal(pool: PgPool) {
        let mut conn = pool.acquire().await.unwrap();
        web_interfaces::dismiss_suggestion(&pool, &mut conn, check(HTTP_CHECK), now()).await.unwrap();
        sqlx::query("delete from checks.checks where id = $1::text::uuid")
            .bind(HTTP_CHECK)
            .execute(&pool)
            .await
            .unwrap();
        assert_eq!(count(&pool, "dismissed").await, 0);
    }
}

db_test! {
    async fn api_005_reads_many_devices_with_one_query_each(pool: PgPool) {
        let mut conn = pool.acquire().await.unwrap();
        make(&mut conn, "http://a.lan/").await;
        web_interfaces::create(&mut conn, device(DOCKER_HOST), &input("http://b.lan/"), now())
            .await
            .unwrap();
        let read = web_interfaces::for_devices(&pool, &[device(PVE_01), device(DOCKER_HOST)])
            .await
            .unwrap();
        assert_eq!(read.len(), 2);
        let none = web_interfaces::for_devices(&pool, &[]).await.unwrap();
        assert!(none.is_empty());
    }
}

db_test! {
    async fn r_016_the_runtime_role_can_do_every_read_and_write(pool: PgPool) {
        add_operator(&pool).await;
        // Detached: a connection whose role changed must not return to the pool (R-019).
        let mut conn = pool.acquire().await.unwrap().detach();
        sqlx::query("set role anima_runtime").execute(&mut conn).await.unwrap();

        let id = make(&mut conn, "https://pve.lan/").await;
        web_interfaces::accept_certificate(&mut conn, id, acceptance(HEX)).await.unwrap();
        web_interfaces::update(&mut conn, id, &input("https://pve.lan/x")).await.unwrap();
        web_interfaces::update(&mut conn, id, &input("https://other.lan/")).await.unwrap();
        web_interfaces::accept_suggestion(&pool, &mut conn, check(HTTP_CHECK), now()).await.unwrap();
        web_interfaces::dismiss_suggestion(&pool, &mut conn, check(HTTP_CHECK), now()).await.unwrap();
        let read = crate::web_interfaces::store::for_devices(&mut conn, &[device(PVE_01)])
            .await
            .unwrap();
        assert_eq!(read.len(), 2);
        let dismissed = crate::web_interfaces::store::dismissed_among(&mut conn, &[check(HTTP_CHECK)])
            .await
            .unwrap();
        assert_eq!(dismissed.len(), 1);
        web_interfaces::delete(&mut conn, id).await.unwrap();
    }
}
