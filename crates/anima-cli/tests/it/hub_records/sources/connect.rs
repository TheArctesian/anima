//! `anima source connect github` without `--from-file` against the scripted hub: the three
//! presses, every phase line, `--state`, `--cancel`, every exit code and the one-time link that
//! is printed once and is in no other output (PRJ-039, PRJ-041, CMD-021, CMD-030, MCP-004).
//! This file holds the scripted hub and the attempts it plays; the tests are in the folder.

mod link;
mod phases;
mod state;

use serde_json::{Value, json};

use crate::kit::Anima;
use crate::kit::fake_hub::{FakeHub, data};
use crate::kit::subscription_hub::{Script, Socket, socket};

const ID: &str = "0190b000-0000-7000-8000-000000000001";
const PAGE: &str = "https://hub.example/connect/github";
/// The one-time link of the scripted hub; no output but the terminal's may hold it.
const TICKET: &str = "TICKET-MARKER-0123456789012345678901234567";
const LINK: &str =
    "https://hub.example/connect/github?ticket=TICKET-MARKER-0123456789012345678901234567";
const INSTALL: &str = "https://github.com/apps/anima-1a2b/installations/new";
const DELETE: &str = "https://github.com/settings/apps/anima-1a2b/advanced";

fn attempt(phase: &str) -> Value {
    json!({
        "id": ID, "phase": phase, "startedBy": "HUMAN", "startedAt": "2026-11-14T10:00:00Z",
        "expiresAt": "2026-11-14T10:55:00Z", "pageUrl": PAGE, "appId": null, "appSlug": null,
        "ownerLogin": null, "installUrl": null, "appSettingsUrl": null, "failure": null,
        "message": null, "account": null,
    })
}

fn made(mut base: Value) -> Value {
    base["appId"] = json!("4242");
    base["appSlug"] = json!("anima-1a2b");
    base["ownerLogin"] = json!("me");
    base
}

fn app_created() -> Value {
    let mut app = made(attempt("APP_CREATED"));
    app["installUrl"] = json!(INSTALL);
    app["message"] = json!("The App anima-1a2b is made. Press Install on GitHub.");
    app
}

fn connected() -> Value {
    let mut done = made(attempt("CONNECTED"));
    done["message"] = json!("GitHub is connected. The first sync has started.");
    done["account"] = json!({
        "id": "0190b000-0000-7000-8000-0000000000aa", "sourceKey": "github", "label": "mine",
        "instanceKey": "4242/77", "credentialStored": true, "health": "STARTING",
        "pausedBy": null, "lastSyncAt": null, "lastError": null, "failureCount": 0,
        "nextSyncAt": null, "syncing": true, "createdAt": "2026-11-14T10:05:00Z",
    });
    done
}

fn failed(failure: &str, message: &str, delete: bool) -> Value {
    let mut over = made(attempt("FAILED"));
    over["failure"] = json!(failure);
    over["message"] = json!(message);
    if delete {
        over["appSettingsUrl"] = json!(DELETE);
    }
    over
}

fn changes(attempts: &[Value]) -> Vec<Value> {
    attempts
        .iter()
        .map(|attempt| json!({ "githubConnectChanged": attempt }))
        .collect()
}

/// A hub that starts attempt `ID` and plays `script` on its subscription.
async fn scripted(script: Script) -> (FakeHub, Socket) {
    let start = data(json!({ "startGithubConnect": attempt("WAITING_FOR_BROWSER") }));
    scripted_start(script, start).await
}

/// As [`scripted`], with `start` as the answer to the start.
async fn scripted_start(script: Script, start: Value) -> (FakeHub, Socket) {
    let (route, socket) = socket(script);
    let hub = FakeHub::start_with(route).await;
    hub.answer("GithubConnectStart", start);
    hub.answer(
        "GithubConnectOpen",
        data(json!({ "openGithubConnect": { "url": LINK, "expiresAt": "2026-11-14T10:05:00Z" } })),
    );
    (hub, socket)
}

fn signed_in() -> Anima {
    Anima::new().with("ANIMA_TOKEN", "login-token-for-connect-tests")
}

fn asked(hub: &FakeHub, operation: &str) -> usize {
    hub.seen()
        .iter()
        .filter(|seen| seen.operation == operation)
        .count()
}

async fn connect_github(hub: &FakeHub, args: &[&str]) -> crate::kit::Outcome {
    let mut all = vec!["source", "connect", "github"];
    all.extend(args);
    all.extend(["--hub", &hub.origin]);
    signed_in().run(&all).await
}
