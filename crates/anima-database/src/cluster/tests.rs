use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use super::{
    ACCESS_RULES, ClusterError, Fit, create, fit, initdb_reason, major_version, user_map,
    was_stopped_cleanly,
};
use crate::scratch::local_tree;
use crate::{Layout, Program, Tree, lock};

fn layout_in(folder: &Path) -> Layout {
    Layout::new(folder.join("db root"), folder.join("s")).unwrap()
}

fn mode_of(path: &Path) -> u32 {
    fs::metadata(path).unwrap().permissions().mode() & 0o777
}

/// What `pg_controldata` prints for the cluster, as `label` to value.
fn control_value(layout: &Layout, label: &str) -> String {
    let output = local_tree()
        .command(Program::PgControldata)
        .arg(layout.cluster())
        .output()
        .unwrap();
    String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .find_map(|line| line.strip_prefix(label))
        .expect("pg_controldata prints the label")
        .trim_start_matches(':')
        .trim()
        .to_owned()
}

#[test]
fn a_tree_runs_a_cluster_of_its_own_major_version_only() {
    assert_eq!(fit(17, 17), Fit::Same);
    assert_eq!(
        fit(16, 17),
        Fit::ClusterOlder {
            cluster: 16,
            tree: 17
        }
    );
    assert_eq!(
        fit(18, 17),
        Fit::ClusterNewer {
            cluster: 18,
            tree: 17
        }
    );
}

#[test]
fn a_root_with_no_cluster_has_no_version() {
    let folder = tempfile::tempdir().unwrap();

    assert_eq!(major_version(&layout_in(folder.path())).unwrap(), None);
}

#[test]
fn a_version_file_without_a_number_is_an_error() {
    let folder = tempfile::tempdir().unwrap();
    let layout = layout_in(folder.path());
    fs::create_dir_all(layout.cluster()).unwrap();
    fs::write(layout.cluster().join("PG_VERSION"), "seventeen\n").unwrap();

    assert!(matches!(
        major_version(&layout),
        Err(ClusterError::VersionFile)
    ));
}

#[test]
fn db_003_a_new_cluster_is_private_checksummed_and_of_the_trees_version() {
    let folder = tempfile::tempdir().unwrap();
    let layout = layout_in(folder.path());

    create(&lock(&layout).unwrap(), &local_tree(), &layout).unwrap();

    assert_eq!(major_version(&layout).unwrap(), Some(17));
    assert_eq!(control_value(&layout, "Data page checksum version"), "1");
    assert_eq!(mode_of(layout.root()), 0o700);
    assert_eq!(mode_of(&layout.cluster()), 0o700);
    assert!(was_stopped_cleanly(&local_tree(), &layout).unwrap());
    assert!(!layout.cluster_being_made().exists());
}

#[test]
fn db_003_only_this_user_is_the_superuser_and_every_other_login_needs_scram() {
    let folder = tempfile::tempdir().unwrap();
    let layout = layout_in(folder.path());

    create(&lock(&layout).unwrap(), &local_tree(), &layout).unwrap();

    let rules = fs::read_to_string(layout.cluster().join("pg_hba.conf")).unwrap();
    let rules: Vec<&str> = rules
        .lines()
        .filter(|line| !line.starts_with('#'))
        .collect();
    assert_eq!(
        rules,
        [
            "local all anima_super peer map=anima",
            "local all all scram-sha-256"
        ]
    );
    let me = nix::unistd::User::from_uid(nix::unistd::Uid::effective())
        .unwrap()
        .unwrap()
        .name;
    let map = fs::read_to_string(layout.cluster().join("pg_ident.conf")).unwrap();
    let map: Vec<&str> = map.lines().filter(|line| !line.starts_with('#')).collect();
    assert_eq!(map, [format!("anima \"{me}\" anima_super")]);
    assert_eq!(mode_of(&layout.cluster().join("pg_hba.conf")), 0o600);
}

#[test]
fn the_rules_have_no_line_for_a_network_connection() {
    for line in ACCESS_RULES.lines().filter(|line| !line.starts_with('#')) {
        assert!(line.starts_with("local "), "{line}");
    }
}

#[test]
fn a_user_name_cannot_add_a_second_mapping() {
    assert_eq!(
        user_map("ada").lines().last(),
        Some("anima \"ada\" anima_super")
    );
    assert_eq!(
        user_map("a b\" anima_super\nanima \"eve").lines().nth(1),
        Some("anima \"a b\"\" anima_super")
    );
}

/// A tree whose `initdb` fails as the real one does: its error line on standard error.
fn tree_with_a_failing_initdb(folder: &Path) -> Tree {
    let bin = folder.join("tree/bin");
    fs::create_dir_all(&bin).unwrap();
    for program in ["postgres", "pg_ctl", "pg_isready"] {
        fs::write(bin.join(program), "").unwrap();
    }
    let script = "#!/bin/sh\necho 'initdb: error: made to fail' >&2\nexit 1\n";
    fs::write(bin.join("initdb"), script).unwrap();
    fs::set_permissions(bin.join("initdb"), fs::Permissions::from_mode(0o755)).unwrap();
    Tree::at(&folder.join("tree")).unwrap()
}

#[test]
fn a_failed_initdb_says_why_and_leaves_no_cluster() {
    let folder = tempfile::tempdir().unwrap();
    let layout = layout_in(folder.path());
    let tree = tree_with_a_failing_initdb(folder.path());

    let error = create(&lock(&layout).unwrap(), &tree, &layout).unwrap_err();

    assert!(
        matches!(&error, ClusterError::Initdb { reason } if reason == "made to fail"),
        "{error}"
    );
    assert_eq!(major_version(&layout).unwrap(), None);
}

#[test]
fn log_007_only_initdbs_own_error_line_is_passed_on() {
    let printed = "initdb: warning: something small\n\
        STATEMENT:  ALTER USER x WITH PASSWORD E'0707';\n\
        initdb: error: directory \"/x\" exists but is not empty\n\
        initdb: hint: If you want to create a new database system, remove it.\n";

    assert_eq!(
        initdb_reason(printed),
        "directory \"/x\" exists but is not empty"
    );
    assert_eq!(
        initdb_reason("STATEMENT:  ALTER USER x PASSWORD E'0707';\nchild process exited\n"),
        "it printed no cause"
    );
}

#[test]
fn a_cluster_folder_that_is_there_is_never_made_over() {
    let folder = tempfile::tempdir().unwrap();
    let layout = layout_in(folder.path());
    let lock = lock(&layout).unwrap();
    create(&lock, &local_tree(), &layout).unwrap();
    let identifier = control_value(&layout, "Database system identifier");

    let error = create(&lock, &local_tree(), &layout).unwrap_err();

    assert!(matches!(error, ClusterError::AlreadyThere), "{error}");
    assert_eq!(
        control_value(&layout, "Database system identifier"),
        identifier
    );
}

#[test]
fn a_half_made_cluster_left_by_a_crash_is_replaced() {
    let folder = tempfile::tempdir().unwrap();
    let layout = layout_in(folder.path());
    fs::create_dir_all(layout.cluster_being_made().join("base")).unwrap();
    fs::write(layout.cluster_being_made().join("PG_VERSION"), "17\n").unwrap();

    create(&lock(&layout).unwrap(), &local_tree(), &layout).unwrap();

    assert!(!layout.cluster_being_made().exists());
    assert_eq!(control_value(&layout, "Data page checksum version"), "1");
}

#[test]
fn db_003_a_root_that_was_open_to_others_is_closed_before_the_cluster_is_made() {
    let folder = tempfile::tempdir().unwrap();
    let layout = layout_in(folder.path());
    fs::create_dir_all(layout.root()).unwrap();
    fs::set_permissions(layout.root(), fs::Permissions::from_mode(0o755)).unwrap();

    create(&lock(&layout).unwrap(), &local_tree(), &layout).unwrap();

    assert_eq!(mode_of(layout.root()), 0o700);
}
