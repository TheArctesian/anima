use anima_daemon::model::install::{installed, place_archive};
use daemon_core::model_work::cause::Cause;

use super::kit::{Entry, FAKE_RUNTIME, PROGRAM, archive, entry, folders, node, release, target};

#[test]
fn a_whole_archive_is_installed_with_plain_modes() {
    let root = tempfile::tempdir().expect("a folder");
    let folders = folders(root.path());
    let file = archive(&[
        (PROGRAM, 0o4775, Some(FAKE_RUNTIME.as_bytes())),
        ("README", 0o666, Some(b"x")),
    ]);
    place_archive(file, &target(&folders, 0)).expect("installed");
    assert!(installed(&folders, &release(), &entry()));
    let meta = |name: &str| {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::metadata(folders.release(&release()).join(name))
            .expect("a file")
            .permissions()
            .mode()
            & 0o7777
    };
    assert_eq!(meta(PROGRAM), 0o755);
    assert_eq!(meta("README"), 0o644);
    assert_eq!(meta("bin"), 0o755);
}

#[test]
fn an_archive_that_breaks_a_rule_is_refused_and_leaves_nothing() {
    let cases: [&[Entry<'_>]; 5] = [
        &[(PROGRAM, 0o755, Some(b"x")), ("bin/link", 0o777, None)],
        &[
            (PROGRAM, 0o755, Some(b"x")),
            ("bin/Llama-Server", 0o755, Some(b"y")),
        ],
        &[
            (PROGRAM, 0o755, Some(b"x")),
            ("../escape", 0o644, Some(b"y")),
        ],
        &[("bin/other", 0o755, Some(b"x"))],
        &[
            (PROGRAM, 0o755, Some(b"x")),
            ("Bin/other", 0o755, Some(b"y")),
        ],
    ];
    for (index, entries) in cases.iter().enumerate() {
        let root = tempfile::tempdir().expect("a folder");
        let folders = folders(root.path());
        let refused = place_archive(archive(entries), &target(&folders, 0));
        assert_eq!(refused, Err(Cause::ArchiveRejected), "case {index}");
        assert!(!folders.release(&release()).exists(), "case {index}");
        assert!(!root.path().join("escape").exists());
    }
}

#[test]
fn the_files_at_the_machine_read_with_their_defaults() {
    use anima_daemon::model::machine_files::{MachineFileError, settings, sources};
    let home = tempfile::tempdir().expect("a folder");
    assert!(!settings(home.path()).expect("defaults").sandbox);
    assert_eq!(sources(home.path()).expect("none").sources, []);
    std::fs::write(home.path().join("model-work.json"), r#"{"sandbox":true}"#).expect("written");
    assert!(settings(home.path()).expect("read").sandbox);
    std::fs::write(home.path().join("model-work.json"), r#"{"port":1}"#).expect("written");
    assert!(matches!(
        settings(home.path()),
        Err(MachineFileError::Settings(_))
    ));
}

#[test]
fn the_disk_reserve_stops_the_unpack_and_the_download() {
    let root = tempfile::tempdir().expect("a folder");
    let folders = folders(root.path());
    let file = archive(&[(PROGRAM, 0o755, Some(b"x"))]);
    assert_eq!(
        place_archive(file, &target(&folders, u64::MAX)),
        Err(Cause::LimitDisk)
    );
    assert!(!folders.release(&release()).exists());
}

#[tokio::test]
async fn an_install_with_no_room_for_the_archive_downloads_nothing() {
    use anima_daemon::model::install::{Place, install};
    let root = tempfile::tempdir().expect("a folder");
    let node = node(root.path());
    let place = Place {
        folders: &node.folders,
        sources: &node.sources,
        os: super::kit::OS.1,
        architecture: super::kit::ARCH.1,
        reserve: u64::MAX,
    };
    // The address of the entry names no real file; the reserve refuses before a request.
    assert_eq!(install(&release(), &place).await, Err(Cause::LimitDisk));
    assert_eq!(entry().size_bytes, 1 << 20);
}

#[test]
fn a_link_in_a_parent_folder_is_not_followed() {
    use anima_daemon::model::folders::private_below;
    let root = tempfile::tempdir().expect("a folder");
    let elsewhere = tempfile::tempdir().expect("a folder");
    let state = root.path().join("model-state");
    std::fs::create_dir(&state).expect("made");
    std::os::unix::fs::symlink(elsewhere.path(), state.join("servers")).expect("a link");
    assert!(private_below(&state, std::path::Path::new("servers/a/tmp")).is_err());
    assert!(!elsewhere.path().join("a").exists());
}
