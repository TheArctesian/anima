use std::ffi::OsString;
use std::path::{Path, PathBuf};

use super::{Carried, database, gateway, gateway_address, hub, hub_address, migration};
use crate::Folders;

fn folders() -> Folders {
    Folders::new(PathBuf::from("/srv/hub root"), PathBuf::from("/srv/s")).unwrap()
}

fn carried() -> Carried {
    Carried::installed_in(Path::new("/srv/hub root/programs/current"))
}

fn value<'a>(env: &'a [(&'static str, OsString)], name: &str) -> Option<&'a str> {
    env.iter()
        .find(|(key, _)| *key == name)
        .and_then(|(_, value)| value.to_str())
}

#[test]
fn the_programs_of_a_run_are_the_ones_beside_it() {
    let carried = Carried::around(Path::new("/x/programs/current/bin/anima-local-hub")).unwrap();

    assert_eq!(
        carried.program("anima-hub"),
        PathBuf::from("/x/programs/current/bin/anima-hub")
    );
    assert_eq!(
        carried.tree(),
        Path::new("/x/programs/current/database-tree")
    );
    assert_eq!(Carried::around(Path::new("/anima-local-hub")), None);
}

#[test]
fn dbl_012_the_database_is_told_its_folders_and_needs_no_environment() {
    let spec = database(&carried(), &folders());

    assert_eq!(
        spec.args,
        [
            "serve",
            "--tree",
            "/srv/hub root/programs/current/database-tree",
            "--root",
            "/srv/hub root/database",
            "--socket-folder",
            "/srv/s"
        ]
    );
    assert_eq!(spec.env, []);
}

#[test]
fn r_016_the_migration_is_the_owner_and_the_hub_is_the_runtime_login() {
    let migration = migration(&carried(), &folders());
    let hub = hub(
        &carried(),
        &folders(),
        "127.0.0.1:8443".parse().unwrap(),
        "127.0.0.1:4822".parse().unwrap(),
    );

    assert_eq!(migration.args, ["migrate"]);
    assert_eq!(
        value(&migration.env, "ANIMA_DATABASE_URL_FILE"),
        Some("/srv/hub root/database/urls/owner.url")
    );
    assert_eq!(migration.env.len(), 1);
    assert_eq!(hub.args, [] as [OsString; 0]);
    assert_eq!(
        value(&hub.env, "ANIMA_DATABASE_URL_FILE"),
        Some("/srv/hub root/database/urls/hub.url")
    );
}

#[test]
fn net_003_the_hub_listens_on_loopback_and_s4_sets_no_access_rule() {
    let spec = hub(
        &carried(),
        &folders(),
        "127.0.0.1:8443".parse().unwrap(),
        "127.0.0.1:50123".parse().unwrap(),
    );

    assert_eq!(value(&spec.env, "ANIMA_LISTEN"), Some("127.0.0.1:8443"));
    assert_eq!(value(&spec.env, "ANIMA_GUACD"), Some("127.0.0.1:50123"));
    assert_eq!(
        value(&spec.env, "ANIMA_KEY_FILE"),
        Some("/srv/hub root/vault.key")
    );
    for owned_by_s5 in [
        "ANIMA_PUBLIC_ORIGIN",
        "ANIMA_PERMITTED_NETWORKS",
        "ANIMA_PROBE_DENY",
    ] {
        assert_eq!(value(&spec.env, owned_by_s5), None);
    }
}

#[test]
fn the_gateway_listens_on_loopback_only() {
    assert_eq!(gateway(&carried()).args, ["listen", "127.0.0.1:0"]);
}

#[test]
fn the_ready_lines_give_the_addresses() {
    let hub_line = r#"{"timestamp":"2026-11-15T10:00:00Z","level":"INFO","fields":{"message":"hub started","listen":"127.0.0.1:8443"},"target":"anima_hub"}"#;

    assert_eq!(
        gateway_address("anima-gateway: listening on 127.0.0.1:50123"),
        Some("127.0.0.1:50123".parse().unwrap())
    );
    assert_eq!(gateway_address("anima-gateway: session ended"), None);
    assert_eq!(
        hub_address(hub_line),
        Some("127.0.0.1:8443".parse().unwrap())
    );
    assert_eq!(
        hub_address(r#"{"fields":{"message":"said hub started","listen":"127.0.0.1:1"}}"#),
        None
    );
    assert_eq!(hub_address("hub started"), None);
}
