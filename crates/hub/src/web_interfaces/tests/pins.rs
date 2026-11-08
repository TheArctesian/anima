//! Certificate pins against PostgreSQL: kept on a path change, dropped by an origin change,
//! replaced on a new accept, and never left on an `http` interface by a race (WEB-008).

use super::*;

db_test! {
    async fn web_008_a_pin_survives_a_path_change_and_is_dropped_by_an_origin_change(pool: PgPool) {
        add_operator(&pool).await;
        let mut conn = pool.acquire().await.unwrap();
        let id = make(&mut conn, "https://pve.lan:8006/").await;
        let pinned = web_interfaces::accept_certificate(&mut conn, id, acceptance(&HEX.to_uppercase()))
            .await
            .unwrap();
        let pin = pinned.pin.clone().unwrap();
        assert_eq!(pin.fingerprint.hex(), HEX);

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
        assert_eq!(again.pin.unwrap().fingerprint.hex(), other);
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
    async fn web_008_a_pin_cannot_land_on_an_interface_moved_to_http_meanwhile(pool: PgPool) {
        add_operator(&pool).await;
        let mut setup = pool.acquire().await.unwrap();
        let id = make(&mut setup, "https://pve.lan/").await;
        drop(setup);

        let mut mover = pool.begin().await.unwrap();
        web_interfaces::update(&mut mover, id, &input("http://evil.lan/")).await.unwrap();
        let racer = pool.clone();
        let accept = tokio::spawn(async move {
            let mut conn = racer.acquire().await.unwrap();
            web_interfaces::accept_certificate(&mut conn, id, acceptance(HEX)).await
        });
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        assert!(!accept.is_finished(), "the accept waits for the row lock");
        mover.commit().await.unwrap();

        assert!(matches!(accept.await.unwrap(), Err(Error::Validation(_))));
        assert_eq!(count(&pool, "pins").await, 0);
    }
}
