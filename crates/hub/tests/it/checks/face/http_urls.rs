//! The URLs of the HTTP checks of devices, in one query (WEB-002, API-005).

use std::collections::HashMap;

use anima_hub::fleet::checks::{
    self, Definition,
    config::{CheckConfig, HttpCredentials, HttpInput, HttpMethod, StatusRange},
};
use sqlx::PgPool;

use super::{DOCKER_HOST, PVE_01, TRUENAS, create, device, new_check};

fn http(url: &str) -> Definition {
    Definition::Http(HttpInput {
        url: url.to_owned(),
        method: HttpMethod::Get,
        headers: vec![],
        body: None,
        max_redirects: 3,
        accepted_status: vec![StatusRange { from: 200, to: 299 }],
        keyword: None,
        json_query: None,
        json_expected: None,
        credentials: HttpCredentials::None,
        verify_tls: true,
        timeout_ms: 5000,
    })
}

fn tcp() -> Definition {
    Definition::Config(CheckConfig::Tcp {
        port: 22,
        timeout_ms: 5000,
    })
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn web_002_http_urls_lists_each_devices_http_checks_by_name_then_id(pool: PgPool) {
    let b = create(&pool, new_check(PVE_01, "b", http("https://pve.lan:8006"))).await;
    let a = create(&pool, new_check(PVE_01, "a", http("http://pve.lan/"))).await;
    create(&pool, new_check(PVE_01, "ssh", tcp())).await;
    let docker = create(
        &pool,
        new_check(DOCKER_HOST, "ui", http("http://docker.lan")),
    )
    .await;
    create(&pool, new_check(TRUENAS, "ssh", tcp())).await;

    let asked = [device(PVE_01), device(DOCKER_HOST), device(TRUENAS)];
    let found = checks::http_urls(&pool, &asked).await.unwrap();
    assert_eq!(
        found,
        HashMap::from([
            (
                device(PVE_01),
                vec![
                    (a, "http://pve.lan/".to_owned()),
                    (b, "https://pve.lan:8006".to_owned()),
                ]
            ),
            (
                device(DOCKER_HOST),
                vec![(docker, "http://docker.lan".to_owned())]
            ),
        ])
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn web_002_http_urls_ignores_devices_not_asked_and_empty_input(pool: PgPool) {
    create(&pool, new_check(PVE_01, "web", http("http://pve.lan"))).await;
    assert!(checks::http_urls(&pool, &[]).await.unwrap().is_empty());
    let found = checks::http_urls(&pool, &[device(DOCKER_HOST)])
        .await
        .unwrap();
    assert!(found.is_empty());
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn web_002_http_urls_does_not_read_other_types_configs(pool: PgPool) {
    let web = create(&pool, new_check(PVE_01, "web", http("http://pve.lan"))).await;
    let ssh = create(&pool, new_check(PVE_01, "ssh", tcp())).await;
    sqlx::query("UPDATE checks.checks SET config = '{\"broken\": true}' WHERE id = $1")
        .bind(ssh)
        .execute(&pool)
        .await
        .unwrap();
    let found = checks::http_urls(&pool, &[device(PVE_01)]).await.unwrap();
    assert_eq!(
        found,
        HashMap::from([(device(PVE_01), vec![(web, "http://pve.lan".to_owned())])])
    );
}
