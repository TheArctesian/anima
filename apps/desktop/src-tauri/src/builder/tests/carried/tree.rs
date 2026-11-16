//! The PostgreSQL tree in the release (S3 T11, S12 2.2 step 4, D-1, D-2, D-5): the recipe's
//! command, its work folder made anew, the signing identity read from the clone, and the tools
//! a Linux builder needs for it (S12 T3).

use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
    path::{Path, PathBuf},
    sync::Arc,
};

use super::args;
use crate::builder::{
    carried::{on_disk, tree},
    child::Running,
    environment::Environment,
    error::BuilderError,
    job::Target,
    plan::{Program, Repository},
    steps::Workshop,
    tools::{self, Tool},
};

const IDENTITY: &str = "Developer ID Application: Stephen Okita (7GDLG48PV6)";

#[test]
fn s3_t11_the_recipe_runs_by_bin_sh_from_the_clone_into_the_bundles_resource_folder() {
    let clone = Path::new("/data/builds/anima");
    let work = tree::work_folder(clone).unwrap();
    assert_eq!(work, Path::new("/data/builds/database-tree"));
    let linux = tree::build(clone, Target::LinuxX86_64, &work, None).unwrap();
    assert_eq!(linux.program, Program::Sh);
    assert_eq!(linux.cwd, clone);
    assert_eq!(
        args(&linux),
        [
            "apps/desktop/src-tauri/database-tree/build.sh",
            "linux",
            "/data/builds/database-tree",
            "/data/builds/anima/apps/desktop/src-tauri/database"
        ]
    );
    assert_eq!(linux.env, Vec::new(), "Linux signs nothing");
    let mac = tree::build(clone, Target::DarwinUniversal, &work, Some(IDENTITY)).unwrap();
    assert_eq!(args(&mac)[1], "darwin-universal");
    assert_eq!(args(&mac)[2..], args(&linux)[2..]);
    assert_eq!(mac.env, [("SIGN_IDENTITY", IDENTITY.to_owned())]);
}

#[test]
fn s3_t11_a_mac_tree_is_never_built_ad_hoc_for_a_release() {
    let clone = Path::new("/c/anima");
    let work = Path::new("/c/database-tree");
    for identity in [None, Some("")] {
        assert_eq!(
            tree::build(clone, Target::DarwinUniversal, work, identity),
            Err(BuilderError::Bundle)
        );
    }
}

#[test]
fn s12_d_2_bin_sh_is_never_searched_for_on_the_steps_path() {
    let root = tempfile::tempdir().unwrap();
    let planted = root.path().join("bin");
    fs::create_dir_all(&planted).unwrap();
    for program in ["sh", "git"] {
        fs::write(planted.join(program), b"#!/bin/sh\n").unwrap();
        fs::set_permissions(planted.join(program), fs::Permissions::from_mode(0o755)).unwrap();
    }
    let running = Running::default();
    let workshop = Workshop {
        claim: running.claim(Arc::new(|_| {})).unwrap(),
        env: Environment::from_parent([("PATH".into(), planted.clone().into())]),
        repository: Repository::local(Path::new("/data/anima.git")),
        clone: root.path().join("clone"),
    };
    assert_eq!(workshop.program(Program::Sh), Ok(PathBuf::from("/bin/sh")));
    assert_eq!(workshop.program(Program::Git), Ok(planted.join("git")));
}

#[test]
fn s12_d_1_the_work_folder_is_made_anew_with_mode_0700_and_a_planted_link_is_not_followed() {
    let dir = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(dir.path()).unwrap();
    let work = root.join("database-tree");
    fs::create_dir_all(work.join("tree")).unwrap();
    fs::write(work.join("tree/STAMP"), b"an old stamp").unwrap();
    on_disk::renew(&work).unwrap();
    assert_eq!(fs::read_dir(&work).unwrap().count(), 0);
    let mode = fs::symlink_metadata(&work).unwrap().permissions().mode();
    assert_eq!(mode & 0o777, 0o700);

    let outside = root.join("outside");
    fs::create_dir_all(&outside).unwrap();
    fs::write(outside.join("keep"), b"not the build's").unwrap();
    fs::remove_dir(&work).unwrap();
    symlink(&outside, &work).unwrap();
    on_disk::renew(&work).unwrap();
    assert!(fs::symlink_metadata(&work).unwrap().is_dir());
    assert_eq!(fs::read(outside.join("keep")).unwrap(), b"not the build's");

    fs::remove_dir(&work).unwrap();
    fs::write(&work, b"a file").unwrap();
    on_disk::renew(&work).unwrap();
    assert!(work.is_dir());
}

#[test]
fn s12_s_7_a_work_folder_reached_through_a_link_or_in_a_shared_folder_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(dir.path()).unwrap();
    let real = root.join("elsewhere");
    fs::create_dir_all(real.join("database-tree")).unwrap();
    fs::write(real.join("database-tree/keep"), b"not the build's").unwrap();
    let linked = root.join("builds");
    symlink(&real, &linked).unwrap();
    assert_eq!(
        on_disk::renew(&linked.join("database-tree")),
        Err(BuilderError::Bundle)
    );
    assert_eq!(
        fs::read(real.join("database-tree/keep")).unwrap(),
        b"not the build's"
    );
    let shared = root.join("shared");
    fs::create_dir_all(&shared).unwrap();
    for mode in [0o775, 0o757] {
        fs::set_permissions(&shared, fs::Permissions::from_mode(mode)).unwrap();
        assert_eq!(
            on_disk::renew(&shared.join("database-tree")),
            Err(BuilderError::Bundle),
            "{mode:o}"
        );
    }
    fs::set_permissions(&shared, fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(on_disk::renew(&shared.join("database-tree")), Ok(()));
}

#[test]
fn s12_t3_a_linux_builder_without_a_tool_of_the_tree_recipe_names_it() {
    let env = Environment::from_parent([("PATH".into(), "/usr/local/bin".into())]);
    let tree_tool = |path: &Path| {
        let text = path.to_str().unwrap();
        text == "/usr/include/zlib.h"
            || ["make", "bison", "flex", "patchelf", "readelf", "curl"]
                .iter()
                .any(|program| text == format!("/usr/bin/{program}"))
    };
    let has_tree_tools = |exists: &dyn Fn(&Path) -> bool| {
        !tools::missing(Target::LinuxX86_64, &env, exists).contains(&Tool::TreeTools)
    };
    assert!(has_tree_tools(&tree_tool));
    for absent in [
        "/usr/include/zlib.h",
        "/usr/bin/make",
        "/usr/bin/bison",
        "/usr/bin/flex",
        "/usr/bin/patchelf",
        "/usr/bin/readelf",
        "/usr/bin/curl",
    ] {
        let without = |path: &Path| tree_tool(path) && path != Path::new(absent);
        assert!(!has_tree_tools(&without), "{absent}");
    }
    // The recipe runs with `PATH=/usr/bin:/bin`: a tool elsewhere on the step's path is missing.
    let elsewhere = |path: &Path| {
        tree_tool(path) && path != Path::new("/usr/bin/bison")
            || path == Path::new("/usr/local/bin/bison")
    };
    assert!(!has_tree_tools(&elsewhere));
    assert!(!tools::missing(Target::DarwinUniversal, &env, |_| false).contains(&Tool::TreeTools));
    let json = serde_json::to_string(&[Tool::TreeTools]).unwrap();
    assert_eq!(json, r#"["tree-tools"]"#);
}
