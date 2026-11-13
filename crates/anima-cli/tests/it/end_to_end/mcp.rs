//! `anima mcp` against the real hub (MCP-002, MCP-007, MCP-013, MCP-014): the read tools find
//! what the command line made, a tool that is off or not approved changes nothing and leaves
//! no audit row, an approved call changes the hub, and only a person at a terminal widens the
//! saved permissions.

use serde_json::{Value, json};
use sqlx::PgPool;

use super::actors::audit_rows;
use super::words;
use crate::kit::real_hub::RealHub;
use crate::kit::terminal::Terminal;
use crate::mcp::session::{Desk, Session};

/// The sites on the hub, by name.
async fn sites_on(hub: &RealHub, reader: &str) -> Vec<String> {
    let data = hub.data(reader, "{ sites { name } }", json!({})).await;
    let sites = data["sites"].as_array().expect("sites").iter();
    let mut names: Vec<String> = sites
        .filter_map(|site| site["name"].as_str())
        .map(str::to_owned)
        .collect();
    names.sort();
    names
}

fn refused(answer: &Value) -> &str {
    assert_eq!(answer["isError"], true, "{answer}");
    answer["content"][0]["text"].as_str().unwrap_or_default()
}

/// The command line makes a site `home` and devices `nas` and `web`; their IDs by name.
async fn fleet(hub: &RealHub, reader: &str) -> [String; 2] {
    let anima = hub.anima().with("ANIMA_TOKEN", reader);
    let site = anima
        .run(&words("site create home --subnet 10.10.0.0/24"))
        .await;
    assert_eq!(site.code, 0, "{site:?}");
    let mut ids = Vec::new();
    for (name, host) in [("nas", "10.10.0.5"), ("web", "10.10.0.6")] {
        let line = format!("device create {name} --site home --host {host} --json");
        let made = anima.run(&words(&line)).await;
        assert_eq!(made.code, 0, "{made:?}");
        ids.push(made.json()["id"].as_str().expect("an id").to_owned());
    }
    [ids[0].clone(), ids[1].clone()]
}

#[sqlx::test(migrations = "../hub/migrations")]
async fn mcp_002_read_tools_find_the_fleet_and_an_off_tool_changes_nothing(pool: PgPool) {
    let hub = RealHub::start(pool).await;
    let reader = hub.login_token().await;
    let [nas, web] = fleet(&hub, &reader).await;
    let desk = Desk::on(hub.anima());
    let (mut session, initialised) = desk.start_at(&hub.origin, &reader, &[]).await;
    assert!(
        initialised["serverInfo"]["name"].is_string(),
        "{initialised}"
    );
    let names = session.tool_names().await;
    assert!(
        names.contains(&"anima_devices_search".to_owned()),
        "{names:?}"
    );
    assert!(
        names.iter().all(|name| name != "anima_site_create"),
        "read-only: {names:?}"
    );

    let found = session.call("anima_devices_search", json!({})).await;
    assert_eq!(found["isError"], false, "{found}");
    let devices = found["structuredContent"]["devices"]
        .as_array()
        .expect("devices")
        .clone();
    let pairs: Vec<(&str, &str)> = devices
        .iter()
        .map(|device| {
            (
                device["name"].as_str().unwrap_or_default(),
                device["id"].as_str().unwrap_or_default(),
            )
        })
        .collect();
    assert_eq!(
        pairs,
        [("nas", nas.as_str()), ("web", web.as_str())],
        "{found}"
    );
    let got = session
        .call("anima_device_get", json!({ "device": "web" }))
        .await;
    assert_eq!(got["isError"], false, "{got}");
    assert_eq!(
        got["structuredContent"]["device"]["id"],
        web.as_str(),
        "{got}"
    );

    let before = audit_rows(&hub, &reader).await.len();
    let off = session
        .call(
            "anima_site_create",
            json!({ "name": "away", "subnet": "10.30.0.0/24" }),
        )
        .await;
    let text = refused(&off);
    assert!(text.contains("operator"), "{text}");
    assert_eq!(audit_rows(&hub, &reader).await.len(), before);
    assert_eq!(sites_on(&hub, &reader).await, ["home"]);
    session.finish().await;
}

/// Saves `--ask anima_site_create --allow anima_device_delete` as a person at a terminal.
async fn widen_at_a_terminal(desk: &Desk) {
    let line = "mcp permissions set --ask anima_site_create --allow anima_device_delete";
    let refused = desk.anima().run(&words(line)).await;
    assert_eq!(refused.code, 2, "no terminal, no widening: {refused:?}");
    assert!(refused.stderr.contains("terminal"), "{refused:?}");
    let (code, screen) = Terminal::start(desk.anima(), &words(line)).finish().await;
    assert_eq!(code, 0, "{screen}");
}

/// A session whose client asks the person and answers with `approve`.
async fn asking_session(hub: &RealHub, desk: &Desk, token: &str, approve: bool) -> Session {
    let (mut session, _) = desk.start_at(&hub.origin, token, &[]).await;
    session.approval = Some(json!({ "action": "accept", "content": { "approve": approve } }));
    session
}

#[sqlx::test(migrations = "../hub/migrations")]
async fn mcp_013_an_asking_tool_changes_the_hub_only_when_the_person_approves(pool: PgPool) {
    let hub = RealHub::start(pool).await;
    let reader = hub.login_token().await;
    let [nas, _] = fleet(&hub, &reader).await;
    let desk = Desk::on(hub.anima()).with_elicitation();
    widen_at_a_terminal(&desk).await;
    let away = json!({ "name": "away", "subnet": "10.30.0.0/24" });

    let before = audit_rows(&hub, &reader).await.len();
    let mut declined = asking_session(&hub, &desk, &reader, false).await;
    refused(&declined.call("anima_site_create", away.clone()).await);
    assert_eq!(declined.asked.len(), 1);
    declined.finish().await;
    assert_eq!(audit_rows(&hub, &reader).await.len(), before);
    assert_eq!(sites_on(&hub, &reader).await, ["home"]);

    let mut approved = asking_session(&hub, &desk, &reader, true).await;
    let made = approved.call("anima_site_create", away).await;
    assert_eq!(made["isError"], false, "{made}");
    assert_eq!(sites_on(&hub, &reader).await, ["away", "home"]);

    let wrong = approved
        .call(
            "anima_device_delete",
            json!({ "device": nas, "confirm": "NAS" }),
        )
        .await;
    refused(&wrong);
    let query = "query($id: ID!) { device(id: $id) { name } }";
    let still = hub.data(&reader, query, json!({ "id": nas })).await;
    assert_eq!(
        still["device"]["name"], "nas",
        "a wrong confirm deletes nothing"
    );
    let right = approved
        .call(
            "anima_device_delete",
            json!({ "device": nas, "confirm": "nas" }),
        )
        .await;
    assert_eq!(right["isError"], false, "{right}");
    let gone = hub.data(&reader, query, json!({ "id": nas })).await;
    assert_eq!(gone["device"], Value::Null);
    approved.finish().await;
}

#[sqlx::test(migrations = "../hub/migrations")]
async fn mcp_009_every_read_tool_answers_from_the_real_hub(pool: PgPool) {
    let hub = RealHub::start(pool).await;
    let reader = hub.login_token().await;
    let [nas, _] = fleet(&hub, &reader).await;
    let query = "query($id: ID!) { device(id: $id) { checks { id } } }";
    let checks = hub.data(&reader, query, json!({ "id": nas })).await;
    let check = checks["device"]["checks"][0]["id"].clone();
    let desk = Desk::on(hub.anima());
    let (mut session, _) = desk.start_at(&hub.origin, &reader, &[]).await;
    let metrics = json!({ "device": "nas", "range": "DAY", "detail": true });
    let calls = [
        ("anima_fleet_overview", json!({})),
        ("anima_devices_search", json!({ "detail": true })),
        (
            "anima_device_get",
            json!({ "device": "nas", "detail": true }),
        ),
        ("anima_device_metrics", metrics),
        ("anima_check_get", json!({ "check": check, "detail": true })),
        ("anima_maintenance_list", json!({})),
        ("anima_events_search", json!({})),
        ("anima_audit_list", json!({ "actor": "USER" })),
        ("anima_sessions_list", json!({})),
        ("anima_notifications_overview", json!({})),
        ("anima_releases_list", json!({})),
        ("anima_vpn_status", json!({})),
        ("anima_retention_list", json!({})),
    ];
    for (tool, arguments) in calls {
        let answer = session.call(tool, arguments).await;
        assert_eq!(answer["isError"], false, "{tool}: {answer}");
        assert!(answer["structuredContent"].is_object(), "{tool}: {answer}");
    }
    session.finish().await;
}
