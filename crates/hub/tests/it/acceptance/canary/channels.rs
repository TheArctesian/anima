//! Notification channels of each kind with their secrets, each sent a test to a receiver on
//! 127.0.0.1, which the hub's address policy refuses, so each send takes its error path; an
//! update and a refused input with secrets too (NOT-002, NOT-004, P3-003, K7, LOG-007).

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use serde_json::{Value, json};
use tokio::net::TcpListener;

use super::{
    Secrets,
    edge::{WholeHub, code},
};

const CREATE: &str = "mutation($input: NotificationChannelInput!) { \
    createNotificationChannel(input: $input) { id } }";
const UPDATE: &str = "mutation($id: ID!, $input: NotificationChannelInput!) { \
    updateNotificationChannel(id: $id, input: $input) { id } }";
const SEND_TEST: &str = "mutation($id: ID!) { sendTestNotification(id: $id) { ok reason code } }";

/// A listener on 127.0.0.1 that counts the connections it is offered.
async fn receiver() -> (u16, Arc<AtomicUsize>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let offered = Arc::new(AtomicUsize::new(0));
    let counted = offered.clone();
    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            counted.fetch_add(1, Ordering::SeqCst);
            drop(stream);
        }
    });
    (port, offered)
}

/// The settings of one channel of each kind at `server`, each holding canaries.
fn settings(server: &str, secrets: &mut Secrets) -> [Value; 5] {
    let (path, query) = (
        secrets.plant("webhook-path"),
        secrets.plant("webhook-query"),
    );
    let header = secrets.plant("webhook-header");
    let recipient = format!("{}@example.test", secrets.plant("smtp-recipient"));
    let apprise_url = format!(
        "discord://1234/{}?format=markdown",
        secrets.plant("apprise-url")
    );
    [
        json!({ "webhook": { "url": format!("{server}/hooks/{path}?token={query}"),
                "headers": [{ "name": "X-Api-Key", "value": header }] } }),
        json!({ "ntfy": { "server": server, "topic": secrets.plant("ntfy-topic"),
                "token": secrets.plant("ntfy-token") } }),
        json!({ "email": { "host": "127.0.0.1", "port": port_of(server), "username": "hub",
                "password": secrets.plant("smtp-password"), "from": "Anima <hub@example.test>",
                "to": [recipient] } }),
        json!({ "apprise": { "server": server, "urls": [apprise_url] } }),
        json!({ "apprise": { "server": server, "mode": "CONFIG_KEY",
                "key": secrets.plant("apprise-key") } }),
    ]
}

fn port_of(server: &str) -> u16 {
    server.rsplit(':').next().unwrap().parse().unwrap()
}

/// Makes one channel of each kind with the operator's `token`, sends each a test, updates the
/// webhook's header, and sends an input the hub refuses.
pub async fn send_tests(hub: &WholeHub, token: &str, secrets: &mut Secrets) {
    let (port, offered) = receiver().await;
    let server = format!("https://127.0.0.1:{port}");
    let mut ids = Vec::new();
    for (number, settings) in settings(&server, secrets).into_iter().enumerate() {
        let input = json!({ "name": format!("channel {number}"), "settings": settings });
        let made = hub.data(token, CREATE, json!({ "input": input })).await;
        let id = made["createNotificationChannel"]["id"].clone();
        let sent = hub.data(token, SEND_TEST, json!({ "id": id })).await;
        let delivery = &sent["sendTestNotification"];
        assert_eq!(delivery["ok"], false, "channel {number}");
        assert_eq!(
            delivery["reason"], "TARGET_NOT_PERMITTED",
            "channel {number}"
        );
        ids.push(id);
    }
    assert_eq!(
        offered.load(Ordering::SeqCst),
        0,
        "P3-003: no loopback send"
    );

    let header = secrets.plant("webhook-new-header");
    let webhook = json!({ "name": "channel 0", "settings": { "webhook": {
        "headers": [{ "name": "X-Api-Key", "value": header }] } } });
    hub.data(token, UPDATE, json!({ "id": ids[0], "input": webhook }))
        .await;
    let plain = format!(
        "http://127.0.0.1/hooks/{}",
        secrets.plant("refused-webhook-path")
    );
    let refused = json!({ "name": "refused", "settings": { "webhook": { "url": plain } } });
    let response = hub
        .as_bearer(token, CREATE, json!({ "input": refused }))
        .await;
    assert_eq!(code(&response), "VALIDATION", "an http URL is refused");
}
