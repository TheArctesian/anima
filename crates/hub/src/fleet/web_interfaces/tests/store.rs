//! The store and the face against PostgreSQL: reads, writes, pins, cascades, idempotence and
//! the runtime role's privileges (WEB-001, WEB-002, WEB-008, R-016).

use super::*;

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
        let read = crate::fleet::web_interfaces::store::for_devices(&mut conn, &[device(PVE_01)])
            .await
            .unwrap();
        assert_eq!(read.len(), 2);
        let dismissed = crate::fleet::web_interfaces::store::dismissed_among(&mut conn, &[check(HTTP_CHECK)])
            .await
            .unwrap();
        assert_eq!(dismissed.len(), 1);
        web_interfaces::delete(&mut conn, id).await.unwrap();
    }
}
