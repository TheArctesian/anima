use anima_daemon::model::install::{installed, place_archive};
use daemon_core::model_work::cause::Cause;

use super::kit::{Entry, FAKE_RUNTIME, PROGRAM, archive, entry, folders, release};

#[test]
fn a_whole_archive_is_installed_with_plain_modes() {
    let root = tempfile::tempdir().expect("a folder");
    let folders = folders(root.path());
    let file = archive(&[
        (PROGRAM, 0o4775, Some(FAKE_RUNTIME.as_bytes())),
        ("README", 0o666, Some(b"x")),
    ]);
    place_archive(file, &folders, &release(), &entry()).expect("installed");
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
}

#[test]
fn an_archive_that_breaks_a_rule_is_refused_and_leaves_nothing() {
    let cases: [&[Entry<'_>]; 4] = [
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
    ];
    for (index, entries) in cases.iter().enumerate() {
        let root = tempfile::tempdir().expect("a folder");
        let folders = folders(root.path());
        let refused = place_archive(archive(entries), &folders, &release(), &entry());
        assert_eq!(refused, Err(Cause::ProgramMismatch), "case {index}");
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
