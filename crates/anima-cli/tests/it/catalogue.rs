//! `anima catalogue list`, `show` and `install`, and the MCP tools `anima_catalogue_list`,
//! `anima_catalogue_show` and `anima_catalogue_install`, against the scripted hub (task H38,
//! CMD-030, R-234).

use serde_json::{Value, json};

use crate::kit::Anima;
use crate::kit::fake_hub::{FakeHub, data};
use crate::mcp::session::{API_KEY, Desk, assert_protocol_only, hub};

const TOKEN: &str = "login-token-for-catalogue-tests";
const DEVICE: &str = "0190a000-0000-7000-8002-000000000002";
const REASON: &str = "The install needs task H6 of the hosted applications plan, which is not \
                      built. No machine was reached.";
const COMPOSE: &str = "services:\n  postgres:\n    image: postgres@sha256:0\n";
const DIGEST: &str = "sha256:0000000000000000000000000000000000000000000000000000000000000000";

fn refused() -> Value {
    json!({ "refusal": "INSTALL_NOT_BUILT", "reason": REASON, "needsOpenToNetwork": false })
}

fn listed() -> Value {
    json!({ "catalogueEntries": [
        { "id": "owntracks-recorder", "name": "OwnTracks Recorder", "kind": "APPLICATION",
          "version": "1.0.7", "tags": ["location"], "marks": [], "class": "APPLICATION",
          "tier": "TESTED",
          "lastMachineTest": "2026-11-20", "pinned": "2026-11-20",
          "install": { "refusal": "INSTALL_NOT_BUILT", "needsOpenToNetwork": false } },
        { "id": "postgresql", "name": "PostgreSQL", "kind": "DATABASE", "version": "18.7",
          "tags": ["database"], "marks": [], "lastMachineTest": null, "pinned": "2026-11-20",
          "class": "APPLICATION", "tier": "TESTED",
          "install": { "refusal": "INSTALL_NOT_BUILT", "needsOpenToNetwork": false } },
        { "id": "code-server", "name": "code-server", "kind": "APPLICATION",
          "version": "4.145.2-ls377", "tags": ["linuxserver", "programming"], "marks": [],
          "lastMachineTest": null, "pinned": "2026-11-21", "class": "SHELL",
          "tier": "GENERATED",
          "install": { "refusal": "GENERATED_NEEDS_DIGEST", "needsOpenToNetwork": false } }
    ] })
}

fn described() -> Value {
    json!({ "catalogueEntry": {
        "id": "postgresql", "name": "PostgreSQL", "description": "A PostgreSQL server.",
        "kind": "DATABASE", "version": "18.7", "author": "PGDG", "tags": ["database"],
        "iconUrl": null, "documentationUrl": "https://www.postgresql.org/docs/18/",
        "marks": [], "networkPorts": [], "lastMachineTest": null, "pinned": "2026-11-20",
        "class": "APPLICATION", "tier": "TESTED", "origin": null, "reviewedBy": null,
        "reviewedOn": null, "streams": [],
        "optionalParts": [], "digests": [DIGEST],
        "install": refused(), "environmentVariables": ["POSTGRES_PASSWORD"],
        "composeFile": COMPOSE
    } })
}

fn generated() -> Value {
    json!({ "catalogueEntry": {
        "id": "jellyfin", "name": "jellyfin", "description": "A media server.",
        "kind": "APPLICATION", "version": "10.11.3", "author": "LinuxServer.io",
        "tags": ["linuxserver", "media servers"], "iconUrl": null,
        "documentationUrl": "https://github.com/linuxserver/docker-jellyfin", "marks": [],
        "networkPorts": [], "lastMachineTest": null, "pinned": "2026-11-21",
        "class": "APPLICATION", "tier": "GENERATED",
        "origin": { "publisher": "LinuxServer.io", "read": "2026-11-17" },
        "reviewedBy": "Ada Lovelace", "reviewedOn": "2026-11-21",
        "streams": [{ "port": 7359, "protocols": ["udp"] }],
        "optionalParts": [{ "kind": "DEVICE", "path": "/dev/dri", "on": false,
          "needsMachineSetting": true }],
        "digests": [DIGEST], "install": refused(), "environmentVariables": ["TZ"],
        "composeFile": COMPOSE
    } })
}

async fn anima(hub: &FakeHub, words: &[&str]) -> crate::kit::Outcome {
    let mut line = words.to_vec();
    line.extend(["--hub", hub.origin.as_str()]);
    Anima::new().with("ANIMA_TOKEN", TOKEN).run(&line).await
}

#[tokio::test]
async fn h38_catalogue_list_shows_each_entry_and_says_when_it_is_not_tested_on_a_machine() {
    let hub = FakeHub::start().await;
    hub.answer("CatalogueList", data(listed()));
    let run = anima(&hub, &["catalogue", "list"]).await;
    assert_eq!(run.code, 0, "{run:?}");
    let lines: Vec<&str> = run.stdout.lines().collect();
    assert!(lines[0].starts_with("ID"), "{}", run.stdout);
    assert!(
        lines[1].starts_with("owntracks-recorder") && lines[1].contains("2026-11-20"),
        "{}",
        run.stdout
    );
    assert!(
        lines[2].contains("database") && lines[2].contains("not tested on a machine"),
        "{}",
        run.stdout
    );
    assert!(
        lines[3].contains("generated") && lines[3].contains("shell"),
        "{}",
        run.stdout
    );
}

#[tokio::test]
async fn h39_catalogue_show_gives_the_tier_the_streams_and_the_optional_parts() {
    let hub = FakeHub::start().await;
    hub.answer("CatalogueShow", data(generated()));
    let run = anima(&hub, &["catalogue", "show", "jellyfin"]).await;
    assert_eq!(run.code, 0, "{run:?}");
    for shown in [
        "generated from LinuxServer.io (data of 2026-11-17)",
        "7359 (udp)",
        "device /dev/dri (off, needs a machine setting)",
        "by Ada Lovelace on 2026-11-21",
        DIGEST,
    ] {
        assert!(run.stdout.contains(shown), "{shown}: {}", run.stdout);
    }
}

#[tokio::test]
async fn h38_catalogue_show_gives_the_refusal_the_variable_names_and_the_compose_file() {
    let hub = FakeHub::start().await;
    hub.answer("CatalogueShow", data(described()));
    let run = anima(&hub, &["catalogue", "show", "postgresql"]).await;
    assert_eq!(run.code, 0, "{run:?}");
    assert!(run.stdout.contains("task H6"), "{}", run.stdout);
    assert!(run.stdout.contains("POSTGRES_PASSWORD"), "{}", run.stdout);
    assert!(run.stdout.ends_with(COMPOSE), "{}", run.stdout);
    let sent = hub.seen().pop().expect("a request");
    assert_eq!(sent.variables, json!({ "id": "postgresql" }));

    let hub = FakeHub::start().await;
    hub.answer("CatalogueShow", data(json!({ "catalogueEntry": null })));
    let run = anima(&hub, &["catalogue", "show", "pihole"]).await;
    assert_ne!(run.code, 0, "{run:?}");
    assert!(
        run.stderr.contains("The catalogue lists no entry pihole."),
        "{}",
        run.stderr
    );
}

#[tokio::test]
async fn h38_catalogue_install_prints_the_refusal_and_ends_with_1() {
    let hub = FakeHub::start().await;
    hub.answer(
        "CatalogueInstallAsk",
        data(json!({ "installCatalogueEntry": refused() })),
    );
    let run = anima(
        &hub,
        &[
            "catalogue",
            "install",
            "postgresql",
            "--device",
            DEVICE,
            "--digest",
            DIGEST,
        ],
    )
    .await;
    assert_eq!(run.code, 1, "{run:?}");
    assert!(run.stderr.contains("task H6"), "{}", run.stderr);
    let sent = hub.seen().pop().expect("a request");
    assert_eq!(
        sent.variables,
        json!({ "entryId": "postgresql", "deviceId": DEVICE, "digest": DIGEST })
    );
}

#[tokio::test]
async fn r_234_an_agent_with_a_write_key_asks_for_an_install_and_gets_the_same_refusal() {
    let hub = hub().await;
    hub.answer("CatalogueShow", data(described()));
    hub.answer(
        "CatalogueInstallAsk",
        data(json!({ "installCatalogueEntry": refused() })),
    );
    let desk = Desk::new();
    let (mut session, _) = desk.start(&hub, API_KEY, &["--allow-write"]).await;
    let shown = session
        .call("anima_catalogue_show", json!({ "id": "postgresql" }))
        .await;
    assert_eq!(shown["isError"], false, "{shown}");
    let found = &shown["structuredContent"];
    let file = found["composeFile"].as_str().unwrap_or_default();
    assert!(file.starts_with("services:\n  postgres:"), "{found}");
    assert_eq!(
        found["entry"]["environmentVariables"],
        json!(["POSTGRES_PASSWORD"])
    );
    let asked = session
        .call(
            "anima_catalogue_install",
            json!({ "id": "postgresql", "device": DEVICE }),
        )
        .await;
    assert_eq!(asked["isError"], false, "{asked}");
    let answer = &asked["structuredContent"];
    assert_eq!(answer["refusal"], "INSTALL_NOT_BUILT");
    assert!(
        answer["reason"]
            .as_str()
            .unwrap_or_default()
            .contains("task H6")
    );
    let (printed, stderr) = session.finish().await;
    assert_protocol_only(&printed, &stderr, API_KEY);
}
