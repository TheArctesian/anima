use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use super::{ClusterError, Fit, create, fit, initdb_reason, major_version, was_stopped_cleanly};
use crate::scratch::local_tree;
use crate::{Layout, Password, Program, Role, Tree, lock};

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

    create(
        &lock(&layout).unwrap(),
        &local_tree(),
        &layout,
        &Password::from_bytes(&[7; 24]),
    )
    .unwrap();

    assert_eq!(major_version(&layout).unwrap(), Some(17));
    assert_eq!(control_value(&layout, "Data page checksum version"), "1");
    assert_eq!(mode_of(layout.root()), 0o700);
    assert_eq!(mode_of(&layout.urls_folder()), 0o700);
    assert_eq!(mode_of(&layout.cluster()), 0o700);
}

#[test]
fn log_007_every_login_needs_scram_and_no_password_file_is_left() {
    let folder = tempfile::tempdir().unwrap();
    let layout = layout_in(folder.path());
    let password = Password::from_bytes(&[7; 24]);

    create(&lock(&layout).unwrap(), &local_tree(), &layout, &password).unwrap();

    let rules = fs::read_to_string(layout.cluster().join("pg_hba.conf")).unwrap();
    let methods: Vec<&str> = rules
        .lines()
        .filter(|line| !line.starts_with('#') && !line.trim().is_empty())
        .filter_map(|line| line.split_whitespace().last())
        .collect();
    assert_eq!(methods.first(), Some(&"scram-sha-256"));
    assert!(
        methods.iter().all(|method| *method == "scram-sha-256"),
        "{methods:?}"
    );

    let left: Vec<_> = fs::read_dir(layout.urls_folder())
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert_eq!(
        left,
        ["superuser.url"],
        "the password file of initdb is gone"
    );
    assert_no_file_holds(&layout.cluster(), password.expose());
}

/// No file below `folder` holds `secret` as text: the cluster stores only a SCRAM verifier.
fn assert_no_file_holds(folder: &Path, secret: &str) {
    for entry in fs::read_dir(folder).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            assert_no_file_holds(&path, secret);
        } else if let Ok(bytes) = fs::read(&path) {
            let found = bytes
                .windows(secret.len())
                .any(|window| window == secret.as_bytes());
            assert!(!found, "{} holds the password", path.display());
        }
    }
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
fn log_007_a_failed_initdb_says_why_and_leaves_no_password_file_and_no_cluster() {
    let folder = tempfile::tempdir().unwrap();
    let layout = layout_in(folder.path());
    let tree = tree_with_a_failing_initdb(folder.path());

    let error = create(
        &lock(&layout).unwrap(),
        &tree,
        &layout,
        &Password::from_bytes(&[7; 24]),
    )
    .unwrap_err();

    assert!(
        matches!(&error, ClusterError::Initdb { reason } if reason == "made to fail"),
        "{error}"
    );
    assert!(!layout.urls_folder().join("initdb.password").exists());
    assert!(!layout.url_file(Role::Superuser).exists());
    assert_eq!(major_version(&layout).unwrap(), None);
}

#[test]
fn a_cluster_folder_that_is_there_is_never_made_over() {
    let folder = tempfile::tempdir().unwrap();
    let layout = layout_in(folder.path());
    let lock = lock(&layout).unwrap();
    create(
        &lock,
        &local_tree(),
        &layout,
        &Password::from_bytes(&[7; 24]),
    )
    .unwrap();
    let superuser_file = fs::read(layout.url_file(Role::Superuser)).unwrap();

    let error = create(
        &lock,
        &local_tree(),
        &layout,
        &Password::from_bytes(&[8; 24]),
    )
    .unwrap_err();

    assert!(matches!(error, ClusterError::AlreadyThere), "{error}");
    assert_eq!(
        fs::read(layout.url_file(Role::Superuser)).unwrap(),
        superuser_file
    );
}

#[test]
fn a_half_made_cluster_left_by_a_crash_is_replaced() {
    let folder = tempfile::tempdir().unwrap();
    let layout = layout_in(folder.path());
    fs::create_dir_all(layout.cluster_being_made().join("base")).unwrap();
    fs::write(layout.cluster_being_made().join("PG_VERSION"), "17\n").unwrap();

    create(
        &lock(&layout).unwrap(),
        &local_tree(),
        &layout,
        &Password::from_bytes(&[7; 24]),
    )
    .unwrap();

    assert!(!layout.cluster_being_made().exists());
    assert_eq!(control_value(&layout, "Data page checksum version"), "1");
    assert!(was_stopped_cleanly(&local_tree(), &layout).unwrap());
}

#[test]
fn log_007_only_initdbs_own_error_line_is_passed_on() {
    let printed = "initdb: warning: something small\n\
        STATEMENT:  ALTER USER \"anima_super\" WITH PASSWORD E'0707';\n\
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
fn db_003_a_root_that_was_open_to_others_is_closed_before_the_cluster_is_made() {
    let folder = tempfile::tempdir().unwrap();
    let layout = layout_in(folder.path());
    fs::create_dir_all(layout.urls_folder()).unwrap();
    fs::set_permissions(layout.root(), fs::Permissions::from_mode(0o755)).unwrap();
    fs::set_permissions(layout.urls_folder(), fs::Permissions::from_mode(0o777)).unwrap();

    create(
        &lock(&layout).unwrap(),
        &local_tree(),
        &layout,
        &Password::from_bytes(&[7; 24]),
    )
    .unwrap();

    assert_eq!(mode_of(layout.root()), 0o700);
    assert_eq!(mode_of(&layout.urls_folder()), 0o700);
}

#[test]
fn log_007_a_password_file_left_by_a_crash_is_replaced_and_removed() {
    let folder = tempfile::tempdir().unwrap();
    let layout = layout_in(folder.path());
    fs::create_dir_all(layout.urls_folder()).unwrap();
    fs::write(
        layout.urls_folder().join("initdb.password"),
        "left by a crash",
    )
    .unwrap();

    create(
        &lock(&layout).unwrap(),
        &local_tree(),
        &layout,
        &Password::from_bytes(&[7; 24]),
    )
    .unwrap();

    assert!(!layout.urls_folder().join("initdb.password").exists());
    assert_eq!(major_version(&layout).unwrap(), Some(17));
}
