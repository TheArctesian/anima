use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use super::{ClusterError, Fit, create, fit, major_version};
use crate::local_tree::local_tree;
use crate::{Layout, Password, Program};

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

    create(&local_tree(), &layout, &Password::from_bytes(&[7; 24])).unwrap();

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

    create(&local_tree(), &layout, &password).unwrap();

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
    assert!(left.is_empty(), "{left:?}");
    assert_no_file_holds(layout.root(), password.expose());
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

#[test]
fn a_failed_initdb_leaves_no_password_file_and_says_why() {
    let folder = tempfile::tempdir().unwrap();
    let layout = layout_in(folder.path());
    // initdb refuses a folder that already has files.
    fs::create_dir_all(layout.cluster()).unwrap();
    fs::write(layout.cluster().join("occupied"), "").unwrap();

    let error = create(&local_tree(), &layout, &Password::from_bytes(&[7; 24])).unwrap_err();

    assert!(matches!(error, ClusterError::Initdb { .. }), "{error}");
    assert!(!layout.urls_folder().join("initdb.password").exists());
    assert!(layout.cluster().join("occupied").exists());
}
