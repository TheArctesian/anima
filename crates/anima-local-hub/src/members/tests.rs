use std::ffi::OsString;
use std::path::{Path, PathBuf};

use super::{
    Carried, DAEMON, OwnDaemon, Pool, SOCKET_PATH_LIMIT, database, gateway, gateway_address,
    hosted, hub, hub_address, link, migration,
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
    assert_eq!(
        value(&spec.env, "ANIMA_DATA_KEY_DIR"),
        Some("/srv/hub root/data-keys")
    );
    assert_eq!(
        value(&spec.env, "ANIMA_FILES_DIR"),
        Some("/srv/hub root/files")
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

#[test]
fn r_129_a_run_with_the_cloud_key_file_puts_the_hub_in_the_hosted_mode() {
    let key = Path::new("/etc/anima/cloud.key");
    let local = hub(
        &carried(),
        &folders(),
        "127.0.0.1:8443".parse().unwrap(),
        "127.0.0.1:50123".parse().unwrap(),
    );
    assert_eq!(value(&local.env, "ANIMA_HOSTED_CLOUD_KEY_FILE"), None);
    let still_local = hosted(local.clone(), &Pool::default());
    assert_eq!(still_local.env, local.env);
    let with_key = Pool {
        cloud_key_file: Some(key.to_path_buf()),
        ..Pool::default()
    };
    let pool = hosted(local, &with_key);
    assert_eq!(
        value(&pool.env, "ANIMA_HOSTED_CLOUD_KEY_FILE"),
        Some("/etc/anima/cloud.key")
    );
    assert_eq!(value(&pool.env, "ANIMA_PUBLIC_ORIGIN"), None);
    assert_eq!(value(&pool.env, "ANIMA_WEB_ROOT"), None);
}

#[test]
fn hmd_069_a_run_of_the_pool_gives_the_hub_its_public_origin_and_its_web_client() {
    let local = hub(
        &carried(),
        &folders(),
        "127.0.0.1:8443".parse().unwrap(),
        "127.0.0.1:50123".parse().unwrap(),
    );
    assert_eq!(
        value(&local.env, "ANIMA_PUBLIC_ORIGIN"),
        None,
        "the default of the hub"
    );
    let told = Pool {
        cloud_key_file: None,
        public_origin: Some("https://hub.anima.ing".to_owned()),
        web_root: Some(PathBuf::from("/opt/anima/web")),
    };
    let pool = hosted(local, &told);
    assert_eq!(
        value(&pool.env, "ANIMA_PUBLIC_ORIGIN"),
        Some("https://hub.anima.ing")
    );
    assert_eq!(value(&pool.env, "ANIMA_WEB_ROOT"), Some("/opt/anima/web"));
}

#[test]
fn hmd_071_a_hosted_hub_with_the_daemon_program_has_a_daemon_of_its_own() {
    let hosted = Pool {
        cloud_key_file: Some(PathBuf::from("/etc/anima/cloud.key")),
        ..Pool::default()
    };
    let (carried, folders) = (carried(), folders());
    assert_eq!(
        OwnDaemon::of(&carried, &folders, &Pool::default(), true),
        None
    );
    assert_eq!(OwnDaemon::of(&carried, &folders, &hosted, false), None);
    let own = OwnDaemon::of(&carried, &folders, &hosted, true).unwrap();

    let waiting = own.hub_environment(false);
    assert_eq!(
        value(&waiting, "ANIMA_OWN_DAEMON_SOCKET"),
        Some("/srv/s/own-daemon.sock")
    );
    assert_eq!(
        value(&waiting, "ANIMA_OWN_DAEMON_CODE_FILE"),
        Some("/srv/s/own-daemon.code")
    );
    let enrolled = own.hub_environment(true);
    assert_eq!(value(&enrolled, "ANIMA_OWN_DAEMON_CODE_FILE"), None);

    let enrolment = own.enrolment();
    assert_eq!(
        enrolment.args,
        ["enrol", "-", "--hub", "unix:/srv/s/own-daemon.sock"]
    );
    let collection = own.collection();
    assert_eq!(collection.args, ["run", "--hosted-hub", "/srv/hub root"]);
    for spec in [enrolment, collection] {
        assert_eq!(spec.program, carried.program(DAEMON));
        assert_eq!(
            spec.env,
            [("ANIMA_DAEMON_HOME", "/srv/hub root/daemon".into())]
        );
    }
}
