//! `Query.audit`: who changed what, newest first, with secrets redacted (LOG-005, LOG-007,
//! API-006).

use std::net::{IpAddr, Ipv4Addr};

use anima_hub::{
    events::{self, AuditAction, AuditTarget, ClientAddr, NewAudit, Operator},
    ids::OperatorId,
};
use chrono::TimeDelta;
use serde_json::{Value, json};
use sqlx::PgPool;

use super::{
    lists::ROUTER,
    pages::{AUDIT, walk},
};
use crate::hub::{CLIENT, Hub, USERNAME, set_operator, start};

fn audit_entry(change: Value) -> NewAudit {
    NewAudit {
        operator: Operator {
            id: OperatorId::from(uuid::Uuid::now_v7()),
            name: "operator".to_owned(),
        },
        action: AuditAction::Update,
        target: AuditTarget::Device(ROUTER.parse().unwrap()),
        client: ClientAddr(IpAddr::V4(Ipv4Addr::new(100, 64, 0, 9))),
        change,
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn log_005_audit_pages_show_who_changed_what_newest_first(pool: PgPool) {
    set_operator(&pool).await;
    let hub = Hub::new(&pool, start());
    let token = hub.token().await;
    let mut conn = pool.acquire().await.unwrap();
    for (minutes, change) in [
        (10, json!({ "name": "lab-router" })),
        (20, json!({ "ssh": { "password": "hunter2", "port": 22 } })),
    ] {
        let at = start() + TimeDelta::minutes(minutes);
        events::audit(&mut conn, at, audit_entry(change))
            .await
            .unwrap();
    }
    drop(conn);

    let first = hub.post(Some(&token), AUDIT, json!({ "first": 1 })).await;
    let page = &first["data"]["audit"];
    let node = &page["edges"][0]["node"];
    assert_eq!(node["at"], "2030-05-05T12:20:00+00:00");
    assert_eq!(node["operatorName"], "operator");
    assert_eq!(node["action"], "update");
    assert_eq!(node["targetType"], "device");
    assert_eq!(node["targetId"], ROUTER);
    assert_eq!(node["clientAddr"], "100.64.0.9");
    let redacted = json!({ "ssh": { "password": "[redacted]", "port": 22 } });
    assert_eq!(node["change"], redacted);
    assert!(!first.to_string().contains("hunter2"));
    assert_eq!(page["pageInfo"]["hasNextPage"], true);

    let (ids, more) = walk(&hub, &token, (AUDIT, "audit"), 1).await;
    assert_eq!((ids.len(), more), (3, vec![true, true, false]));
    let all = hub.post(Some(&token), AUDIT, json!({})).await;
    let entries: Vec<_> = all["data"]["audit"]["edges"]
        .as_array()
        .unwrap()
        .iter()
        .map(|edge| {
            let node = &edge["node"];
            let shown = [&node["action"], &node["operatorName"], &node["clientAddr"]];
            shown.map(|field| field.as_str().unwrap().to_owned())
        })
        .collect();
    let update = ["update", "operator", "100.64.0.9"].map(str::to_owned);
    let login = ["login", USERNAME, CLIENT].map(str::to_owned);
    assert_eq!(entries, [update.clone(), update, login]);
    assert_eq!(ids[0], page["edges"][0]["node"]["id"]);
}
