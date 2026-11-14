//! The App's name and the manifest the operator's browser posts to GitHub (PRJ-039).

use serde_json::{Value, json};
use source_exchange::{ManifestFacts, Webhook};

const BASE32: &[u8; 32] = b"abcdefghijklmnopqrstuvwxyz234567";
const PLACEHOLDER_HOOK: &str = "https://example.com/anima-webhook-off";

/// What the App can do: read metadata, write everything else the hand-made App has.
const WRITE_PERMISSIONS: [&str; 23] = [
    "contents",
    "administration",
    "actions",
    "actions_variables",
    "checks",
    "deployments",
    "environments",
    "issues",
    "pull_requests",
    "statuses",
    "workflows",
    "secrets",
    "repository_hooks",
    "pages",
    "discussions",
    "repository_projects",
    "security_events",
    "vulnerability_alerts",
    "dependabot_secrets",
    "merge_queues",
    "packages",
    "secret_scanning_alerts",
    "repository_custom_properties",
];

/// `anima-` and 10 lowercase base32 characters from the first 50 bits of `entropy`.
#[expect(
    clippy::trivially_copy_pass_by_ref,
    reason = "the signature is the one of `Registration::name`, which the hub calls with a borrowed buffer"
)]
pub(crate) fn name(entropy: &[u8; 7]) -> String {
    let bits = entropy
        .iter()
        .fold(0_u64, |all, byte| (all << 8) | u64::from(*byte))
        >> 6;
    let mut out = String::from("anima-");
    for place in (0..10).rev() {
        let index = usize::try_from((bits >> (place * 5)) & 0x1F).unwrap_or(0);
        out.push(char::from(BASE32[index]));
    }
    out
}

/// The manifest as JSON text. No events and no callback URLs: the App is private, asks for no
/// sign-in, and its webhook is off.
pub(crate) fn manifest(facts: &ManifestFacts<'_>) -> String {
    let origin = facts.origin;
    let mut permissions = serde_json::Map::new();
    permissions.insert("metadata".to_owned(), json!("read"));
    for permission in WRITE_PERMISSIONS {
        permissions.insert(permission.to_owned(), json!("write"));
    }
    let mut body = json!({
        "name": facts.name,
        "url": origin,
        "description": "Made by Anima for one hub. Anima reads and changes the repositories of this account.",
        "public": false,
        "redirect_url": format!("{origin}/connect/github/created"),
        "setup_url": format!("{origin}/connect/github/installed"),
        "setup_on_update": false,
        "request_oauth_on_install": false,
        "default_permissions": Value::Object(permissions),
    });
    let hook = match facts.webhook {
        Webhook::InactiveAtHub => Some(format!("{origin}/connect/github/hook")),
        Webhook::Omitted => None,
        Webhook::InactivePlaceholder => Some(PLACEHOLDER_HOOK.to_owned()),
    };
    if let Some(url) = hook {
        body["hook_attributes"] = json!({ "url": url, "active": false });
    }
    body.to_string()
}
