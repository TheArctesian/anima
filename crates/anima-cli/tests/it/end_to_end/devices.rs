//! `anima site` and `anima device` against the real hub: each change read back through the hub
//! (CMD-020, CMD-024, DEV-014).

use serde_json::{Value, json};
use sqlx::PgPool;

use super::hub::RealHub;
use super::terminal::Terminal;
use super::words;

/// The devices on the hub, as `[name, host, site name]`, read by the hub's own query.
async fn devices_on(hub: &RealHub, reader: &str) -> Vec<[String; 3]> {
    let query = "{ devices(first: 200) { edges { node { name host site { name } } } } }";
    let data = hub.data(reader, query, json!({})).await;
    let text = |value: &Value| value.as_str().expect("text").to_owned();
    let edges = data["devices"]["edges"].as_array().expect("edges").iter();
    edges
        .map(|edge| {
            let node = &edge["node"];
            let site = &node["site"]["name"];
            [text(&node["name"]), text(&node["host"]), text(site)]
        })
        .collect()
}

fn device(name: &str, host: &str, site: &str) -> [String; 3] {
    [name, host, site].map(str::to_owned)
}

#[sqlx::test(migrations = "../hub/migrations")]
async fn cmd_020_sites_and_devices_round_trip_through_the_hub(pool: PgPool) {
    let hub = RealHub::start(pool).await;
    let reader = hub.login_token().await;
    let anima = hub.anima().with("ANIMA_TOKEN", &reader);

    let site = anima
        .run(&words("site create home --subnet 10.10.0.0/24 --json"))
        .await;
    assert_eq!(site.code, 0, "{site:?}");
    assert_eq!(site.json()["name"], "home", "{site:?}");
    let created = anima
        .run(&words(
            "device create nas --site home --host 10.10.0.5 --json",
        ))
        .await;
    assert_eq!(created.code, 0, "{created:?}");
    let id = created.json()["id"].as_str().expect("an id").to_owned();
    let on_hub = devices_on(&hub, &reader).await;
    assert_eq!(on_hub, [device("nas", "10.10.0.5", "home")]);

    let listed = anima.run(&words("device list --json")).await;
    assert_eq!(listed.code, 0, "{listed:?}");
    let listed = listed.json();
    assert_eq!(listed["totalCount"], 1, "{listed}");
    let node = &listed["edges"][0]["node"];
    let shown = (node["id"].as_str(), node["name"].as_str());
    assert_eq!(shown, (Some(id.as_str()), Some("nas")), "{listed}");
    let got = anima.run(&words("device get nas --json")).await;
    assert_eq!(got.code, 0, "{got:?}");
    assert_eq!(got.json()["id"], id.as_str(), "{got:?}");

    let updated = anima
        .run(&words("device update nas --name vault --host 10.10.0.6"))
        .await;
    assert_eq!(updated.code, 0, "{updated:?}");
    let on_hub = devices_on(&hub, &reader).await;
    assert_eq!(on_hub, [device("vault", "10.10.0.6", "home")]);

    let mut terminal = Terminal::start(&anima, &words("device delete vault --json"));
    terminal.expect("[y/N]").await;
    terminal.type_line("n").await;
    let (code, screen) = terminal.finish().await;
    assert_eq!(code, 1, "{screen}");
    assert!(screen.contains(r#""code":"DECLINED""#), "{screen}");
    let left = devices_on(&hub, &reader).await.len();
    assert_eq!(left, 1, "a declined delete sent nothing");
    let declined = anima.run(&words("device delete vault --json")).await;
    assert_eq!(declined.code, 2, "no terminal and no --yes: {declined:?}");
    assert_eq!(devices_on(&hub, &reader).await.len(), 1);

    let deleted = anima.run(&words("device delete vault --yes --json")).await;
    assert_eq!(deleted.code, 0, "{deleted:?}");
    assert_eq!(deleted.json(), json!(id));
    assert_eq!(devices_on(&hub, &reader).await.len(), 0);
    let gone = anima.run(&words("device get vault")).await;
    assert_eq!(gone.code, 3, "{gone:?}");
}
