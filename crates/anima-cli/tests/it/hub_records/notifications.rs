//! `anima notify` against the scripted hub (NOT-001..007, CMD-024, CMD-025): the shared fixtures;
//! the tests are in `channels`, `channel_test` and `settings`.

use serde_json::{Value, json};

use crate::kit::Anima;
use crate::kit::fake_hub::data;

pub const TOKEN: &str = "login-token-for-notify-tests";
pub const WEBHOOK_URL: &str = "https://hooks.example.net/services/T000/B000/SECRETSECRET";
pub const SMTP_PASSWORD: &str = "smtp-password-that-must-not-leak";

pub fn fixtures() -> Value {
    let text = include_str!("../../../../../contracts/fixtures/phase3.json");
    serde_json::from_str(text).expect("fixtures")
}

/// The K6 channels as `ChannelList` nodes: each setting gets its `__typename`, and the
/// webhook's last delivery is the first K6 delivery.
pub fn channels() -> Vec<Value> {
    let fixtures = fixtures();
    let delivery = fixtures["deliveries"][0].clone();
    fixtures["channels"]
        .as_array()
        .expect("channels")
        .iter()
        .map(|channel| {
            let mut channel = channel.clone();
            let name = match channel["kind"].as_str().expect("kind") {
                "WEBHOOK" => "WebhookSettings",
                "NTFY" => "NtfySettings",
                "EMAIL" => "EmailSettings",
                _ => "AppriseSettings",
            };
            channel["settings"]["__typename"] = json!(name);
            channel["lastDelivery"] = if channel["kind"] == "WEBHOOK" {
                delivery.clone()
            } else {
                Value::Null
            };
            channel
        })
        .collect()
}

pub fn signed_in() -> Anima {
    Anima::new().with("ANIMA_TOKEN", TOKEN)
}

pub fn saved(mutation: &str, name: &str) -> Value {
    data(json!({ mutation: {
        "id": "0190a000-0000-7000-8017-0000000000aa", "name": name, "kind": "WEBHOOK",
        "onByDefault": false, "updatedAt": "2026-11-13T10:00:00Z",
    }}))
}

pub fn webhook_document() -> String {
    json!({
        "name": "ops-hook", "onByDefault": true,
        "settings": { "webhook": { "url": WEBHOOK_URL, "method": "POST",
                                   "headers": [{ "name": "X-Key", "value": SMTP_PASSWORD }] } },
    })
    .to_string()
}

mod channel_test;
mod channels;
mod settings;
