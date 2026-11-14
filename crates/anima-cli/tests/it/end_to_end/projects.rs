//! The engine and Projects together (XP-02; CMD-030): `anima` sets Projects on, connects a
//! GitHub App with a key from a 0600 file, syncs, lists the projects and leaves one out, and
//! the MCP read tool sees the same projects. The hub reads a loopback GitHub, never GitHub.

use std::os::unix::fs::PermissionsExt as _;
use std::time::Duration;

use rsa::{
    RsaPrivateKey,
    pkcs1::{EncodeRsaPrivateKey as _, LineEnding},
    rand_core::OsRng,
};
use serde_json::{Value, json};
use sqlx::PgPool;

use super::words;
use crate::kit::{Anima, github, real_hub::RealHub};
use crate::mcp::session::Desk;

/// A throwaway 2048-bit App key in a 0600 file of the test's own (CMD-025).
fn key_file(folder: &tempfile::TempDir) -> std::path::PathBuf {
    let key = RsaPrivateKey::new(&mut OsRng, 2048).expect("a key");
    let pem = key.to_pkcs1_pem(LineEnding::LF).expect("PEM");
    let path = folder.path().join("app.pem");
    std::fs::write(&path, pem.as_bytes()).expect("the key file");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).expect("0600");
    path
}

/// The slugs `anima project list --json` prints, in its order.
async fn listed(anima: &Anima) -> Vec<String> {
    let list = anima.run(&words("project list --json")).await;
    assert_eq!(list.code, 0, "{list:?}");
    slugs(&list.json()["edges"], |edge| &edge["node"])
}

fn slugs(items: &Value, node: impl Fn(&Value) -> &Value) -> Vec<String> {
    let items = items
        .as_array()
        .unwrap_or_else(|| panic!("a list: {items}"));
    items
        .iter()
        .map(|item| node(item)["slug"].as_str().expect("a slug").to_owned())
        .collect()
}

/// Waits until the account has a finished run and its projects are listed.
async fn synced(anima: &Anima, want: usize) -> Vec<String> {
    for _ in 0..200 {
        let account = anima.run(&words("source get mine --json")).await;
        let syncing = account.code != 0 || account.json()["syncing"] != false;
        let projects = listed(anima).await;
        if !syncing && projects.len() == want {
            return projects;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("the sync did not import the repositories");
}

#[sqlx::test(migrations = "../hub/migrations")]
async fn cmd_030_the_command_line_and_mcp_see_the_projects_the_engine_imported(pool: PgPool) {
    let (hub, stub) = RealHub::with_github(pool).await;
    github::repositories(&stub, &["me/site", "me/blog"]);
    let token = hub.login_token().await;
    let anima = hub.anima().with("ANIMA_TOKEN", &token);

    let on = anima.run(&words("extension on projects")).await;
    assert_eq!(on.code, 0, "{on:?}");
    let folder = tempfile::tempdir().expect("a folder");
    let key = key_file(&folder);
    let key = key.to_str().expect("a UTF-8 path");
    let connect = format!(
        "source connect github --label mine --app-id 4242 --installation-id 77 --from-file {key} --json"
    );
    let connected = anima.run(&words(&connect)).await;
    assert_eq!(connected.code, 0, "{connected:?}");
    assert_eq!(connected.json()["credentialStored"], true);
    let sync = anima.run(&words("source sync mine")).await;
    assert_eq!(sync.code, 0, "{sync:?}");
    assert_eq!(synced(&anima, 2).await, ["blog", "site"]);
    assert!(
        stub.gets()
            .iter()
            .any(|path| path.starts_with("/repos/me/site/git/trees/")),
        "the hub read the files through the stub"
    );

    let left = anima.run(&words("project leave-out me/blog")).await;
    assert_eq!(left.code, 0, "{left:?}");
    assert_eq!(listed(&anima).await, ["site"]);
    let out = anima.run(&words("project list --left-out --json")).await;
    assert_eq!(slugs(&out.json()["edges"], |edge| &edge["node"]), ["blog"]);

    let desk = Desk::on(hub.anima());
    let (mut session, _) = desk.start_at(&hub.origin, &token, &[]).await;
    let found = session.call("anima_projects_search", json!({})).await;
    assert_eq!(found["isError"], false, "{found}");
    assert_eq!(
        slugs(&found["structuredContent"]["projects"], |project| project),
        ["site"]
    );
    session.finish().await;
}
