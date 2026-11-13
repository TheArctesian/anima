//! The rest of the command line against the real hub, one run each: every read command answers
//! with JSON, and the changes not covered elsewhere land on the hub (CMD-020, CMD-021).

use serde_json::{Value, json};
use sqlx::PgPool;

use super::words;
use crate::kit::Anima;
use crate::kit::real_hub::RealHub;

/// A site `home`, a device `nas` with its default check, and the check's ID.
async fn fleet(hub: &RealHub, anima: &Anima, reader: &str) -> String {
    let site = anima
        .run(&words("site create home --subnet 10.10.0.0/24"))
        .await;
    assert_eq!(site.code, 0, "{site:?}");
    let line = "device create nas --site home --host 10.10.0.5 --mac-address aa:bb:cc:dd:ee:01 \
        --ssh-enabled true --ssh-port 22 --ssh-username admin --ssh-auth password";
    let device = anima.run(&words(line)).await;
    assert_eq!(device.code, 0, "{device:?}");
    let query = "{ devices { edges { node { checks { id } } } } }";
    let data = hub.data(reader, query, json!({})).await;
    let check = &data["devices"]["edges"][0]["node"]["checks"][0]["id"];
    check.as_str().expect("the default check").to_owned()
}

/// Runs `line` with `--json`; its JSON output.
async fn json_of(anima: &Anima, line: &str) -> Value {
    let outcome = anima.run(&words(&format!("{line} --json"))).await;
    assert_eq!(outcome.code, 0, "{line}: {outcome:?}");
    serde_json::from_str(&outcome.stdout)
        .unwrap_or_else(|_| panic!("{line}: not JSON: {outcome:?}"))
}

#[sqlx::test(migrations = "../hub/migrations")]
async fn cmd_020_every_read_command_answers_from_the_real_hub(pool: PgPool) {
    let hub = RealHub::start(pool).await;
    let reader = hub.login_token().await;
    let anima = hub.anima().with("ANIMA_TOKEN", &reader);
    let check = fleet(&hub, &anima, &reader).await;
    let reads = [
        "auth status".to_owned(),
        "site list".to_owned(),
        "tag list".to_owned(),
        "device uptime nas".to_owned(),
        "device metrics nas".to_owned(),
        "agent status nas".to_owned(),
        "web list nas".to_owned(),
        format!("check get {check}"),
        format!("check heartbeats {check}"),
        "check defaults get".to_owned(),
        "maintenance list".to_owned(),
        "event list".to_owned(),
        "audit list".to_owned(),
        "session list".to_owned(),
        "key list".to_owned(),
        "retention list".to_owned(),
        "notify channel list".to_owned(),
        "notify settings get".to_owned(),
        "notify deliveries".to_owned(),
        "release list".to_owned(),
        "release next-version".to_owned(),
        "builder list".to_owned(),
        "vpn status".to_owned(),
    ];
    for line in &reads {
        json_of(&anima, line).await;
    }
    let tables = [
        "device list",
        "audit list",
        "event list",
        "check list --device nas",
    ];
    for line in tables {
        let outcome = anima.run(&words(line)).await;
        assert_eq!(outcome.code, 0, "{line}: {outcome:?}");
    }
}

#[sqlx::test(migrations = "../hub/migrations")]
async fn cmd_020_the_other_changes_land_on_the_real_hub(pool: PgPool) {
    let hub = RealHub::start(pool).await;
    let reader = hub.login_token().await;
    let anima = hub.anima().with("ANIMA_TOKEN", &reader);
    let check = fleet(&hub, &anima, &reader).await;

    json_of(&anima, "device update nas --tag rack --tag core").await;
    assert_eq!(json_of(&anima, "tag list").await, json!(["core", "rack"]));
    json_of(
        &anima,
        &format!("check update {check} --name ping --retries 4"),
    )
    .await;
    json_of(&anima, "check defaults set --retries 5").await;
    json_of(&anima, "site set-wake-relay home --device nas").await;
    json_of(&anima, "retention set events 30").await;
    let query = format!(
        "{{ check(id: \"{check}\") {{ name retries }} checkDefaults {{ retries }} \
         sites {{ wakeRelay {{ name }} }} retention {{ subject days }} }}"
    );
    let data = hub.data(&reader, &query, json!({})).await;
    assert_eq!(data["check"], json!({ "name": "ping", "retries": 4 }));
    assert_eq!(data["checkDefaults"]["retries"], 5);
    assert_eq!(data["sites"][0]["wakeRelay"]["name"], "nas");
    let events = data["retention"].as_array().expect("periods").iter();
    let kept: Vec<&Value> = events
        .filter(|period| period["subject"] == "EVENTS")
        .collect();
    assert_eq!(kept[0]["days"], 30, "{data}");

    let web = json_of(
        &anima,
        "web create nas --url https://10.10.0.5:8443/ --label panel",
    )
    .await;
    let web_id = web["id"]
        .as_str()
        .unwrap_or_else(|| panic!("an id: {web}"))
        .to_owned();
    json_of(&anima, &format!("web update {web_id} --label admin")).await;
    let query = "{ devices { edges { node { webInterfaces { id label } } } } }";
    let data = hub.data(&reader, query, json!({})).await;
    let interfaces = &data["devices"]["edges"][0]["node"]["webInterfaces"];
    assert_eq!(interfaces, &json!([{ "id": web_id, "label": "admin" }]));
    json_of(&anima, &format!("web delete {web_id} --yes")).await;
    let data = hub.data(&reader, query, json!({})).await;
    let interfaces = &data["devices"]["edges"][0]["node"]["webInterfaces"];
    assert_eq!(interfaces, &json!([]));

    let line = "maintenance create --name hold --kind manual --timezone UTC --targets device:nas";
    let window = json_of(&anima, line).await;
    let window = window["id"].as_str().expect("an id").to_owned();
    json_of(&anima, &format!("maintenance start {window}")).await;
    let query = "{ maintenanceWindows { activeManual activeNow } }";
    let active = hub.data(&reader, query, json!({})).await;
    assert_eq!(
        active["maintenanceWindows"][0]["activeManual"], true,
        "{active}"
    );
    json_of(&anima, &format!("maintenance stop {window}")).await;
    json_of(&anima, &format!("maintenance delete {window} --yes")).await;
    let query = "{ maintenanceWindows { id } }";
    let left = hub.data(&reader, query, json!({})).await;
    assert_eq!(left["maintenanceWindows"], json!([]), "{left}");

    let gone = anima
        .run(&words(
            "session end 0190a000-0000-7000-8000-00000000dead --json",
        ))
        .await;
    assert_eq!(gone.code, 3, "{gone:?}");
}
