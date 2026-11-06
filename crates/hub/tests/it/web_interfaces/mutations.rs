//! The result of each mutation and the pin's life (WEB-001, WEB-002, WEB-008).

use serde_json::json;
use sqlx::PgPool;

use super::harness::{
    DELETE, DISMISS, FINGERPRINT, HTTP_CHECK, PVE_01, accept_certificate_query,
    accept_suggestion_query, add_operator, create, create_query, data, fixture_test, input,
    schema_for, update_query,
};

fixture_test! {
    async fn web_001_create_returns_the_k1_shape_with_defaults(pool: PgPool) {
        let created = data(
            &schema_for(&pool),
            &create_query(),
            json!({ "deviceId": PVE_01, "input": { "label": null, "url": "https://pve-01.home.lan:8006/" } }),
        )
        .await["createWebInterface"].clone();
        assert_eq!(created["deviceId"], PVE_01);
        assert_eq!(created["label"], "pve-01.home.lan");
        assert_eq!(created["url"], "https://pve-01.home.lan:8006/");
        assert_eq!(created["openIn"], "ANIMA_TAB");
        assert_eq!(created["pinnedFingerprintSha256"], json!(null));
        assert_eq!(created["pinnedAt"], json!(null));
    }

    async fn web_001_update_replaces_the_fields_and_delete_returns_the_id(pool: PgPool) {
        let schema = schema_for(&pool);
        let id = create(&schema, "http://pve-01.home.lan/").await;
        let updated = data(
            &schema,
            &update_query(),
            json!({ "id": id, "input": { "label": "Admin", "url": "http://pve-01.home.lan:81/", "openIn": "SYSTEM_BROWSER" } }),
        )
        .await["updateWebInterface"].clone();
        assert_eq!(updated["id"], id.as_str());
        assert_eq!(updated["label"], "Admin");
        assert_eq!(updated["openIn"], "SYSTEM_BROWSER");
        let deleted = data(&schema, DELETE, json!({ "id": id })).await;
        assert_eq!(deleted["deleteWebInterface"], id.as_str());
        let left: i64 = sqlx::query_scalar("SELECT count(*) FROM web_interfaces.interfaces")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(left, 0);
    }

    async fn web_008_accepting_a_certificate_pins_lower_hex_and_an_origin_change_drops_it(pool: PgPool) {
        add_operator(&pool).await;
        let schema = schema_for(&pool);
        let id = create(&schema, "https://pve-01.home.lan:8006/").await;
        let typed = FINGERPRINT.to_uppercase();
        let pinned = data(
            &schema,
            &accept_certificate_query(),
            json!({ "id": id, "fingerprint": typed }),
        )
        .await["acceptWebCertificate"].clone();
        assert_eq!(pinned["pinnedFingerprintSha256"], FINGERPRINT);
        assert_eq!(pinned["pinnedAt"], "2026-11-06T09:00:00+00:00");

        let same_origin = data(
            &schema,
            &update_query(),
            json!({ "id": id, "input": input("https://pve-01.home.lan:8006/other") }),
        )
        .await["updateWebInterface"].clone();
        assert_eq!(same_origin["pinnedFingerprintSha256"], FINGERPRINT);

        let moved = data(
            &schema,
            &update_query(),
            json!({ "id": id, "input": input("https://pve-01.home.lan:9443/") }),
        )
        .await["updateWebInterface"].clone();
        assert_eq!(moved["pinnedFingerprintSha256"], json!(null));
        assert_eq!(moved["pinnedAt"], json!(null));
    }

    async fn web_002_accepting_a_suggestion_makes_an_interface_and_a_second_accept_returns_it(pool: PgPool) {
        let schema = schema_for(&pool);
        let first = data(&schema, &accept_suggestion_query(), json!({ "checkId": HTTP_CHECK }))
            .await["acceptWebInterfaceSuggestion"].clone();
        assert_eq!(first["deviceId"], PVE_01);
        assert_eq!(first["url"], "https://pve-01.home.lan:8006/");
        assert_eq!(first["openIn"], "ANIMA_TAB");
        let second = data(&schema, &accept_suggestion_query(), json!({ "checkId": HTTP_CHECK }))
            .await["acceptWebInterfaceSuggestion"].clone();
        assert_eq!(second["id"], first["id"]);
    }

    async fn web_002_a_dismissed_suggestion_is_gone_and_dismissing_again_is_harmless(pool: PgPool) {
        let schema = schema_for(&pool);
        let hidden = data(&schema, DISMISS, json!({ "checkId": HTTP_CHECK })).await;
        assert_eq!(hidden["dismissWebInterfaceSuggestion"], HTTP_CHECK);
        data(&schema, DISMISS, json!({ "checkId": HTTP_CHECK })).await;
        let device: anima_hub::ids::DeviceId = PVE_01.parse().unwrap();
        let offered = anima_hub::web_interfaces::suggestions(&pool, &[device]).await.unwrap();
        assert!(offered.get(&device).is_none_or(Vec::is_empty));
    }
}
