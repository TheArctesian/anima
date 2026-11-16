use std::ffi::OsString;
use std::path::{Path, PathBuf};

use super::{
    Carried, SOCKET_PATH_LIMIT, database, gateway, gateway_address, hub, hub_address, link,
    migration,
};
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
fn f_6_the_link_keeps_its_state_below_the_hub_root_and_sends_to_the_hubs_socket() {
    let spec = link(&carried(), &folders()).unwrap();

    assert_eq!(
        spec.program,
        PathBuf::from("/srv/hub root/programs/current/bin/anima-link")
    );
    assert_eq!(
        spec.args,
        [
            "hub",
            "-state",
            "/srv/hub root/link",
            "-target",
            "/srv/s/link.sock"
        ]
    );
    assert_eq!(spec.env, []);
}

#[test]
fn f_6_a_socket_path_that_the_system_cannot_have_is_refused_before_the_link_starts() {
    // The database's socket fits; the link's control socket, deeper below the root, does not.
    let root = |length: usize| {
        let root = PathBuf::from(format!("/{}", "r".repeat(length - 1)));
        Folders::new(root, PathBuf::from("/srv/s")).unwrap()
    };
    let longest = SOCKET_PATH_LIMIT - "/link/control.sock".len();

    let refused = link(&carried(), &root(longest + 1)).unwrap_err();

    assert_eq!(refused.length, SOCKET_PATH_LIMIT + 1);
    assert!(refused.to_string().contains("control.sock"), "{refused}");
    assert!(link(&carried(), &root(longest)).is_ok());
}

#[test]
fn s5_in_the_link_mode_the_hub_is_told_both_sockets_of_the_link() {
    let spec = hub(
        &carried(),
        &folders(),
        "127.0.0.1:8443".parse().unwrap(),
        "127.0.0.1:50123".parse().unwrap(),
    );

    assert_eq!(
        value(&spec.env, "ANIMA_LINK_SOCKET"),
        Some("/srv/s/link.sock")
    );
    assert_eq!(
        value(&spec.env, "ANIMA_LINK_CONTROL"),
        Some("/srv/hub root/link/control.sock")
    );
}

#[test]
fn net_003_the_hub_listens_on_loopback_and_no_network_is_permitted_by_address() {
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
    // NET-001: the link mode permits this computer and the link's devices; the hub refuses
    // a list of networks beside the link's socket.
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
