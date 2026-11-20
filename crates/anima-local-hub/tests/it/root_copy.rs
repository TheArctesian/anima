//! The copy of the root of a stopped hub, with the real programs (ROL-053, ROL-054; R-189).

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt as _;
use std::path::Path;
use std::process::{Command, Output};

use super::{Rig, hub_address, hub_is_healthy, local_hub, platform, stop, tree};

fn copy(rig: &Rig, to: &Path) -> Output {
    rig.command("copy")
        .arg("--tree")
        .arg(tree())
        .arg("--to")
        .arg(to)
        .output()
        .unwrap()
}

fn check(copy: &Path) -> Output {
    let mut command = Command::new(local_hub());
    platform::environment(&mut command);
    command
        .arg("check-copy")
        .arg(copy)
        .arg("--tree")
        .arg(tree())
        .output()
        .unwrap()
}

#[test]
fn rol_054_a_hub_that_runs_is_not_copied() {
    let rig = Rig::new();
    let running = rig.run();
    let to = rig.folder.path().join("copy");

    let refused = copy(&rig, &to);

    assert_eq!(refused.status.code(), Some(10), "{refused:?}");
    assert!(!to.exists(), "nothing was made");
    assert!(
        hub_is_healthy(hub_address(&rig.status())),
        "the hub serves on"
    );
    assert!(stop(running).success());
}

#[test]
fn rol_054_a_copy_of_a_stopped_hub_passes_its_check_and_a_hub_reads_its_data_from_it() {
    let rig = Rig::new();
    assert!(stop(rig.run()).success(), "{}", rig.logs());
    let key = std::fs::read(rig.root().join("vault.key")).unwrap();
    let to = rig.folder.path().join("copy");

    let made = copy(&rig, &to);

    assert!(made.status.success(), "{made:?}");
    let summary: serde_json::Value = serde_json::from_slice(&made.stdout).unwrap();
    assert!(summary["entries"].as_u64().unwrap() > 100, "{summary}");
    assert!(to.join("database.dump").is_file());
    assert!(!to.join("root/programs").exists());
    assert_eq!(
        rig.status()["running"],
        false,
        "the database was stopped again"
    );
    let checked = check(&to);
    assert!(checked.status.success(), "{checked:?}");
    assert_eq!(checked.stdout, made.stdout);

    // The way back: the copy takes the place of the root. The files of the database name the
    // socket folder, so the hub must get the same one. ROL-053: the copy has no programs, and
    // the hub runs on it with programs from outside the root.
    std::fs::rename(
        rig.root(),
        rig.folder.path().join("root of the failed update"),
    )
    .unwrap();
    std::fs::rename(to.join("root"), rig.root()).unwrap();
    assert!(!rig.root().join("programs").exists());
    let again = rig.run();
    assert!(hub_is_healthy(hub_address(&rig.status())), "{}", rig.logs());
    assert_eq!(std::fs::read(rig.root().join("vault.key")).unwrap(), key);
    assert!(stop(again).success());
}

#[test]
fn rol_054_a_copy_with_a_changed_file_or_a_dump_that_does_not_read_fails_its_check() {
    let rig = Rig::new();
    assert!(stop(rig.run()).success(), "{}", rig.logs());
    let to = rig.folder.path().join("copy");
    assert!(copy(&rig, &to).status.success());

    let key = to.join("root/vault.key");
    let whole = std::fs::read(&key).unwrap();
    std::fs::remove_file(&key).unwrap();
    let missing = check(&to);
    assert_eq!(missing.status.code(), Some(1), "{missing:?}");
    let said = String::from_utf8_lossy(&missing.stderr).into_owned();
    assert!(
        said.contains("vault.key") && said.contains("is not there"),
        "{said}"
    );
    std::fs::write(&key, &whole).unwrap();
    #[cfg(unix)]
    {
        let opened = check(&to);
        let said = String::from_utf8_lossy(&opened.stderr).into_owned();
        assert!(
            said.contains("different mode"),
            "a key that others can read: {said}"
        );
        std::fs::set_permissions(&key, std::fs::Permissions::from_mode(0o600)).unwrap();
    }
    assert!(check(&to).status.success(), "the copy is whole again");

    // A dump of the right size and digest in the list, which is no dump: only pg_restore says.
    let dump = to.join("database.dump");
    let not_a_dump = vec![b'x'; 4096];
    std::fs::write(&dump, &not_a_dump).unwrap();
    let list = to.join("copy.json");
    let mut listed: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&list).unwrap()).unwrap();
    listed["dump"]["bytes"] = 4096.into();
    listed["dump"]["sha256"] = sha256_of_4096_x().into();
    std::fs::write(&list, serde_json::to_vec(&listed).unwrap()).unwrap();
    let unreadable = check(&to);
    assert_eq!(unreadable.status.code(), Some(1), "{unreadable:?}");
    let said = String::from_utf8_lossy(&unreadable.stderr).into_owned();
    assert!(said.contains("cannot read the dump"), "{said}");
}

/// The SHA-256 of 4096 bytes `x`, from `shasum -a 256`.
fn sha256_of_4096_x() -> &'static str {
    "a2e659dacb4691e887ac0139f8893d04764ee197d70fb73d3190d56113d18e3e"
}
