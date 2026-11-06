//! Suggestions against PostgreSQL: accept and dismiss, idempotence, cascades and concurrent
//! accepts (WEB-002).

use super::*;

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
    async fn web_002_two_concurrent_accepts_create_one_interface(pool: PgPool) {
        let mut first = pool.begin().await.unwrap();
        let one = web_interfaces::accept_suggestion(&pool, &mut first, check(HTTP_CHECK), now())
            .await
            .unwrap();
        let racer = pool.clone();
        let second = tokio::spawn(async move {
            let mut conn = racer.acquire().await.unwrap();
            web_interfaces::accept_suggestion(&racer, &mut conn, check(HTTP_CHECK), now()).await
        });
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        assert!(!second.is_finished(), "the second accept waits for the device lock");
        first.commit().await.unwrap();

        let two = second.await.unwrap().unwrap();
        assert!(one.created);
        assert!(!two.created);
        assert_eq!(two.interface.id, one.interface.id);
        assert_eq!(count(&pool, "interfaces").await, 1);
    }
}
