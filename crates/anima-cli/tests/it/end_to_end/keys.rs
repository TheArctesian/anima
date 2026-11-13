//! `anima key` and what an API key may do on the real hub (INT-003, LOG-015, CMD-021, CMD-022):
//! a write key changes what INT-003 lets it change, and every other change is refused with
//! exit 5 and a hint, with nothing changed on the hub.

use serde_json::{Value, json};
use sqlx::PgPool;

use super::words;
use crate::kit::real_hub::RealHub;
use crate::kit::{Anima, Outcome};

/// Makes an API key with `anima key create`; its secret, read from standard output.
pub async fn make_key(anima: &Anima, name: &str, scope: &str, made_for: &str) -> String {
    let line = format!("key create --name {name} --scope {scope} --for {made_for} --expires 30d");
    let made = anima.run(&words(&line)).await;
    assert_eq!(made.code, 0, "{made:?}");
    let secret = made.stdout.trim().to_owned();
    let one_line = secret.starts_with("anima_") && !secret.contains('\n');
    assert!(one_line, "{made:?}");
    secret
}

/// Asserts `outcome` is the CMD-022 refusal of a key: exit 5, `FORBIDDEN`, and a hint whose
/// commands exist.
pub async fn assert_refused(outcome: &Outcome, what: &str) {
    assert_eq!(outcome.code, 5, "{what}: {outcome:?}");
    let error = outcome.error();
    assert_eq!(error["code"], "FORBIDDEN", "{what}: {outcome:?}");
    let hint = (error["hint"].as_str()).unwrap_or_else(|| panic!("{what}: no hint {outcome:?}"));
    for command in hint.split('`').skip(1).step_by(2) {
        let mut help = words(command);
        assert_eq!(help.first(), Some(&"anima"), "{what}: {hint}");
        help.remove(0);
        help.push("--help");
        let shown = Anima::new().run(&help).await;
        let exists = shown.code == 0;
        assert!(
            exists,
            "{what}: the hint names `{command}`, which does not exist"
        );
    }
}

/// The device's name and host on the hub.
async fn device_on(hub: &RealHub, reader: &str, id: &str) -> Value {
    let query = "query($id: ID!) { device(id: $id) { name host } }";
    let data = hub.data(reader, query, json!({ "id": id })).await;
    data["device"].clone()
}

/// A site `home` and a device `nas` at 10.10.0.5 made by `anima`; the device's ID.
pub async fn site_and_device(anima: &Anima) -> String {
    let site = anima
        .run(&words("site create home --subnet 10.10.0.0/24"))
        .await;
    assert_eq!(site.code, 0, "{site:?}");
    let line = "device create nas --site home --host 10.10.0.5 --no-default-check --json";
    let device = anima.run(&words(line)).await;
    assert_eq!(device.code, 0, "{device:?}");
    device.json()["id"].as_str().expect("an id").to_owned()
}

#[sqlx::test(migrations = "../hub/migrations")]
async fn int_003_a_write_key_changes_only_what_int_003_permits(pool: PgPool) {
    let hub = RealHub::start(pool).await;
    let reader = hub.login_token().await;
    let operator = hub.anima().with("ANIMA_TOKEN", &reader);
    let write = make_key(&operator, "script", "write", "user").await;
    let script = hub.anima().with("ANIMA_TOKEN", &write);
    let id = site_and_device(&script).await;

    let secret_file = tempfile::NamedTempFile::new().expect("a file");
    std::fs::write(secret_file.path(), "device password\n").expect("written");
    let secret_path = secret_file.path().to_str().expect("UTF-8");
    let credential = format!("device credential set ssh nas --from-file {secret_path} --json");
    let refused = [
        (
            "device update",
            "device update nas --host 10.10.0.66 --json",
        ),
        ("device delete", "device delete nas --yes --json"),
        ("device credential set", credential.as_str()),
        (
            "key create",
            "key create --name more --for user --expires 30d --json",
        ),
        ("retention set", "retention set events 30 --json"),
        ("ssh", "ssh nas --json"),
    ];
    for (what, line) in refused {
        let outcome = script.run(&words(line)).await;
        assert_refused(&outcome, what).await;
        outcome.assert_never_shows(&write);
    }
    let device = device_on(&hub, &reader, &id).await;
    assert_eq!(device, json!({ "name": "nas", "host": "10.10.0.5" }));
    let keys = hub.data(&reader, "{ apiKeys { name } }", json!({})).await;
    assert_eq!(keys["apiKeys"], json!([{ "name": "script" }]));
    let query = "query($id: ID!) { device(id: $id) { ssh { credentialStored } } }";
    let stored = hub.data(&reader, query, json!({ "id": id })).await;
    assert_eq!(
        stored["device"]["ssh"]["credentialStored"], false,
        "{stored}"
    );
}

#[sqlx::test(migrations = "../hub/migrations")]
async fn int_003_keys_are_made_for_a_user_or_an_agent_and_a_read_key_changes_nothing(pool: PgPool) {
    let hub = RealHub::start(pool).await;
    let reader = hub.login_token().await;
    let operator = hub.anima().with("ANIMA_TOKEN", &reader);
    site_and_device(&operator).await;
    let secrets = [
        make_key(&operator, "script", "write", "user").await,
        make_key(&operator, "helper", "write", "agent").await,
        make_key(&operator, "viewer", "read", "user").await,
    ];
    let query = "{ apiKeys { name scope madeFor } }";
    let keys = hub.data(&reader, query, json!({})).await;
    let mut keys = keys["apiKeys"].as_array().expect("keys").clone();
    keys.sort_by_key(|key| key["name"].as_str().unwrap_or_default().to_owned());
    let expected = [
        json!({ "name": "helper", "scope": "WRITE", "madeFor": "AGENT" }),
        json!({ "name": "script", "scope": "WRITE", "madeFor": "USER" }),
        json!({ "name": "viewer", "scope": "READ", "madeFor": "USER" }),
    ];
    assert_eq!(keys, expected);
    let listed = operator.run(&words("key list --json")).await;
    assert_eq!(listed.code, 0, "{listed:?}");
    for secret in &secrets {
        listed.assert_never_shows(secret);
    }

    let viewer = hub.anima().with("ANIMA_TOKEN", &secrets[2]);
    let seen = viewer.run(&words("device get nas --json")).await;
    assert_eq!(seen.code, 0, "a read key reads: {seen:?}");
    let refused = [
        "site create away --subnet 10.30.0.0/24 --json",
        "device create nas2 --site home --host 10.10.0.7 --json",
        "maintenance create --name m --kind manual --timezone UTC --targets device:nas --json",
    ];
    for line in refused {
        let outcome = viewer.run(&words(line)).await;
        assert_refused(&outcome, line).await;
    }
    let query = "{ sites { name } devices { totalCount } maintenanceWindows { id } }";
    let left = hub.data(&reader, query, json!({})).await;
    let expected = json!({ "sites": [{ "name": "home" }], "devices": { "totalCount": 1 },
                           "maintenanceWindows": [] });
    assert_eq!(left, expected);
}
