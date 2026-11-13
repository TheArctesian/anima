//! `anima check` and `anima maintenance` against the real hub: a TCP and an HTTP check from
//! files, pause and resume, delete, and a maintenance window, each read back (CMD-020).

use serde_json::{Value, json};
use sqlx::PgPool;

use super::words;
use crate::kit::Anima;
use crate::kit::real_hub::RealHub;

/// The device's checks on the hub, as `(name, type, paused)`, by name.
async fn checks_on(hub: &RealHub, reader: &str, device: &str) -> Vec<(String, String, bool)> {
    let query = "query($id: ID!) { device(id: $id) { checks { name type paused } } }";
    let data = hub.data(reader, query, json!({ "id": device })).await;
    let text = |value: &Value| value.as_str().expect("text").to_owned();
    let mut checks: Vec<(String, String, bool)> = (data["device"]["checks"].as_array())
        .expect("checks")
        .iter()
        .map(|check| {
            let paused = check["paused"] == true;
            (text(&check["name"]), text(&check["type"]), paused)
        })
        .collect();
    checks.sort();
    checks
}

/// A device `web` with no checks, made through the command line; its ID.
async fn bare_device(anima: &Anima) -> String {
    let site = anima
        .run(&words("site create lab --subnet 10.20.0.0/24"))
        .await;
    assert_eq!(site.code, 0, "{site:?}");
    let line = "device create web --site lab --host 10.20.0.9 --no-default-check --json";
    let device = anima.run(&words(line)).await;
    assert_eq!(device.code, 0, "{device:?}");
    device.json()["id"].as_str().expect("an id").to_owned()
}

/// Creates a check named `name` on `web` from `config`, written to a file.
async fn create_check(anima: &Anima, name: &str, config: &Value) -> String {
    let file = tempfile::NamedTempFile::new().expect("a file");
    std::fs::write(file.path(), config.to_string()).expect("the config written");
    let path = file.path().to_str().expect("a UTF-8 path");
    let line = format!("check create --device web --name {name} --from-file {path} --json");
    let created = anima.run(&words(&line)).await;
    assert_eq!(created.code, 0, "{created:?}");
    created.json()["id"].as_str().expect("an id").to_owned()
}

fn row(name: &str, kind: &str, paused: bool) -> (String, String, bool) {
    (name.to_owned(), kind.to_owned(), paused)
}

#[sqlx::test(migrations = "../hub/migrations")]
async fn cmd_020_checks_are_created_paused_resumed_and_deleted_on_the_hub(pool: PgPool) {
    let hub = RealHub::start(pool).await;
    let reader = hub.login_token().await;
    let anima = hub.anima().with("ANIMA_TOKEN", &reader);
    let device = bare_device(&anima).await;

    create_check(&anima, "ssh-port", &json!({ "tcp": { "port": 22 } })).await;
    let http = json!({ "http": { "url": "http://10.20.0.9/health", "keyword": "ok" } });
    let http_id = create_check(&anima, "health", &http).await;
    let on_hub = checks_on(&hub, &reader, &device).await;
    let both = [row("health", "HTTP", false), row("ssh-port", "TCP", false)];
    assert_eq!(on_hub, both);
    let listed = anima.run(&words("check list --device web --json")).await;
    assert_eq!(listed.code, 0, "{listed:?}");
    let listed = listed.json();
    let mut names: Vec<&str> = (listed["checks"].as_array().expect("checks").iter())
        .filter_map(|check| check["name"].as_str())
        .collect();
    names.sort_unstable();
    assert_eq!(names, ["health", "ssh-port"], "{listed}");

    let paused = anima.run(&words("check pause ssh-port --device web")).await;
    assert_eq!(paused.code, 0, "{paused:?}");
    let on_hub = checks_on(&hub, &reader, &device).await;
    assert!(on_hub.contains(&row("ssh-port", "TCP", true)), "{on_hub:?}");
    let resumed = anima
        .run(&words("check resume ssh-port --device web"))
        .await;
    assert_eq!(resumed.code, 0, "{resumed:?}");
    assert_eq!(checks_on(&hub, &reader, &device).await, both);

    let line = format!("check delete {http_id} --yes --json");
    let deleted = anima.run(&words(&line)).await;
    assert_eq!(deleted.code, 0, "{deleted:?}");
    assert_eq!(deleted.json(), json!(http_id));
    let on_hub = checks_on(&hub, &reader, &device).await;
    assert_eq!(on_hub, [row("ssh-port", "TCP", false)]);
}

#[sqlx::test(migrations = "../hub/migrations")]
async fn cmd_020_a_maintenance_window_is_created_and_listed(pool: PgPool) {
    let hub = RealHub::start(pool).await;
    let reader = hub.login_token().await;
    let anima = hub.anima().with("ANIMA_TOKEN", &reader);
    let device = bare_device(&anima).await;

    let line = "maintenance create --name patching --kind single --timezone Europe/Berlin \
        --starts-at 2031-01-01T02:00:00Z --ends-at 2031-01-01T04:00:00Z --targets device:web \
        --json";
    let created = anima.run(&words(line)).await;
    assert_eq!(created.code, 0, "{created:?}");
    let query = "{ maintenanceWindows { name kind targets { type target } } }";
    let windows = hub.data(&reader, query, json!({})).await;
    let target = json!([{ "type": "DEVICE", "target": device }]);
    let expected = json!([{ "name": "patching", "kind": "SINGLE", "targets": target }]);
    assert_eq!(windows["maintenanceWindows"], expected);
    let listed = anima.run(&words("maintenance list --json")).await;
    assert_eq!(listed.code, 0, "{listed:?}");
    assert_eq!(listed.json()[0]["name"], "patching", "{listed:?}");
    let table = anima.run(&words("maintenance list")).await;
    assert!(table.stdout.contains("patching"), "{table:?}");
}
