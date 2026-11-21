//! The files of a server's tree (R-NEW-containment; SF-NEW-l1workw1 to SF-NEW-l1workw7): reads
//! that refuse a link, a FIFO or an oversized file, and records written in place, never through one.

use std::process::Command;
use std::time::Duration;

use anima_daemon::model::tree::{RECORD, Record, sweep};

#[test]
fn a_file_that_the_runtime_can_replace_is_read_only_when_plain_and_small() {
    use anima_daemon::model::tree::read_capped;
    let root = tempfile::tempdir().expect("a folder");
    let plain = root.path().join("plain");
    std::fs::write(&plain, b"123").expect("written");
    assert_eq!(read_capped(&plain, 16), Some(b"123".to_vec()));
    std::fs::write(root.path().join("large"), vec![b'1'; 17]).expect("written");
    assert_eq!(
        read_capped(&root.path().join("large"), 16),
        None,
        "over the cap"
    );
    std::os::unix::fs::symlink(&plain, root.path().join("link")).expect("a link");
    assert_eq!(read_capped(&root.path().join("link"), 16), None, "a link");
    let fifo = root.path().join("fifo");
    let made = Command::new("/usr/bin/mkfifo")
        .arg(&fifo)
        .status()
        .expect("mkfifo");
    assert!(made.success());
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || sender.send(read_capped(&fifo, 16)));
    assert_eq!(
        receiver.recv_timeout(Duration::from_secs(5)),
        Ok(None),
        "a FIFO, with no wait"
    );
    std::fs::write(root.path().join("exact"), vec![b'1'; 16]).expect("written");
    assert_eq!(
        read_capped(&root.path().join("exact"), 16).map(|bytes| bytes.len()),
        Some(16),
        "exactly the cap"
    );
    std::os::unix::fs::symlink(root.path(), root.path().join("folder")).expect("a link");
    assert_eq!(
        read_capped(&root.path().join("folder/plain"), 16),
        None,
        "the folder is a link"
    );
}

#[test]
fn a_record_that_is_a_link_is_not_swept() {
    let root = tempfile::tempdir().expect("a folder");
    let server = root.path().join("one");
    std::fs::create_dir(&server).expect("a folder");
    let elsewhere = root.path().join("elsewhere.json");
    Record::default().save(&elsewhere);
    std::os::unix::fs::symlink(&elsewhere, server.join(RECORD)).expect("a link");
    assert_eq!(sweep(root.path()).expect("swept"), 0);
    assert!(elsewhere.exists(), "the file the link names survives");
}

#[test]
fn a_server_folder_that_is_a_link_is_neither_read_nor_emptied() {
    let root = tempfile::tempdir().expect("a folder");
    let servers = root.path().join("servers");
    let other = root.path().join("other");
    std::fs::create_dir_all(&servers).expect("a folder");
    std::fs::create_dir_all(&other).expect("a folder");
    Record::default().save(&other.join(RECORD));
    std::os::unix::fs::symlink(&other, servers.join("one")).expect("a link");
    assert_eq!(sweep(&servers).expect("swept"), 0);
    assert!(other.join(RECORD).exists());
}

#[test]
fn a_record_is_written_in_place_of_a_link_or_a_fifo_and_never_through_one() {
    let root = tempfile::tempdir().expect("a folder");
    let target = root.path().join("target");
    std::fs::write(&target, b"keep").expect("written");
    let record = root.path().join(RECORD);
    std::fs::hard_link(&target, &record).expect("a hard link");
    Record::default().save(&record);
    assert_eq!(std::fs::read(&target).expect("read"), b"keep");
    std::fs::remove_file(&record).expect("removed");
    let made = Command::new("/usr/bin/mkfifo")
        .arg(&record)
        .status()
        .expect("mkfifo");
    assert!(made.success());
    let (sender, receiver) = std::sync::mpsc::channel();
    let path = record.clone();
    std::thread::spawn(move || {
        Record::default().save(&path);
        sender.send(())
    });
    assert_eq!(receiver.recv_timeout(Duration::from_secs(5)), Ok(()));
    assert!(std::fs::symlink_metadata(&record).expect("there").is_file());
}

#[test]
fn records_and_failure_files_over_their_real_caps_are_refused() {
    use anima_daemon::model::tree::RECORD_BYTES;
    let root = tempfile::tempdir().expect("a folder");
    let padded = |size: usize| {
        let mut text = b"{\"known\":[],\"cgroup\":null}".to_vec();
        text.resize(size, b' ');
        text
    };
    for (size, swept) in [(RECORD_BYTES, 1), (RECORD_BYTES + 1, 0)] {
        let servers = root.path().join(format!("servers-{size}"));
        std::fs::create_dir_all(servers.join("one")).expect("a folder");
        std::fs::write(servers.join("one").join(RECORD), padded(size)).expect("written");
        assert_eq!(sweep(&servers).expect("swept"), swept, "{size}");
    }
}
