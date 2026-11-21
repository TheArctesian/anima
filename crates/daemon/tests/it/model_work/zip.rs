//! The unpack of a `.zip` (R-NEW-archivezip) with the rules of the `.tar.gz`, against hostile
//! archives made in the test.

use std::io::{Seek as _, Write as _};

use anima_daemon::model::install::place_archive;
use anima_daemon::model::unpack::{Limits, unpack_within};
use daemon_core::model_work::cause::Cause;
use daemon_core::model_work::releases::ArchiveFormat;
use zip::write::SimpleFileOptions;

use super::kit::{FAKE_RUNTIME, PROGRAM, entry, folders, release};

/// One entry of a test zip.
enum Part<'a> {
    File(&'a str, u32, &'a [u8]),
    Folder(&'a str),
    Link(&'a str),
}

fn zip_of(parts: &[Part<'_>], method: zip::CompressionMethod) -> std::fs::File {
    let mut file = tempfile::tempfile().expect("a file");
    {
        let mut zip = zip::ZipWriter::new(&mut file);
        let options = SimpleFileOptions::default().compression_method(method);
        for part in parts {
            match part {
                Part::File(name, mode, bytes) => {
                    zip.start_file(*name, options.unix_permissions(*mode))
                        .expect("an entry");
                    zip.write_all(bytes).expect("bytes");
                }
                Part::Folder(name) => zip.add_directory(*name, options).expect("a folder"),
                Part::Link(name) => zip
                    .add_symlink(*name, "/etc/passwd", options)
                    .expect("a link"),
            }
        }
        zip.finish().expect("finished");
    }
    file.rewind().expect("rewind");
    file
}

fn stored(parts: &[Part<'_>]) -> std::fs::File {
    zip_of(parts, zip::CompressionMethod::Stored)
}

fn target(root: &std::path::Path) -> anima_daemon::model::install::Target {
    let mut entry = entry();
    "https://github.com/example/none.zip".clone_into(&mut entry.address);
    anima_daemon::model::install::Target {
        folders: folders(root),
        release: release(),
        entry,
        reserve: 0,
    }
}

#[test]
fn a_whole_zip_is_installed_with_plain_modes() {
    let root = tempfile::tempdir().expect("a folder");
    let target = target(root.path());
    let file = stored(&[
        Part::Folder("bin/"),
        Part::File(PROGRAM, 0o4777, FAKE_RUNTIME.as_bytes()),
        Part::File("README", 0o666, b"x"),
    ]);
    place_archive(file, &target).expect("installed");
    let mode = |name: &str| {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::metadata(target.folders.release(&release()).join(name))
            .expect("a file")
            .permissions()
            .mode()
            & 0o7777
    };
    assert_eq!(
        (mode(PROGRAM), mode("README"), mode("bin")),
        (0o755, 0o644, 0o755)
    );
}

#[test]
fn a_hostile_zip_is_refused_and_leaves_nothing() {
    let cases: [&[Part<'_>]; 5] = [
        &[Part::File(PROGRAM, 0o755, b"x"), Part::Link("bin/link")],
        &[
            Part::File(PROGRAM, 0o755, b"x"),
            Part::File("../escape", 0o644, b"y"),
        ],
        &[
            Part::File(PROGRAM, 0o755, b"x"),
            Part::File("/tmp/absolute", 0o644, b"y"),
        ],
        &[
            Part::File(PROGRAM, 0o755, b"x"),
            Part::File("BIN/other", 0o644, b"y"),
        ],
        &[
            Part::File(PROGRAM, 0o755, b"x"),
            Part::File("bin\\..\\..\\x", 0o644, b"y"),
        ],
    ];
    for (index, parts) in cases.iter().enumerate() {
        let root = tempfile::tempdir().expect("a folder");
        let target = target(root.path());
        assert_eq!(
            place_archive(stored(parts), &target),
            Err(Cause::ArchiveRejected),
            "case {index}"
        );
        assert!(!target.folders.release(&release()).exists(), "case {index}");
        assert!(
            !root.path().join("escape").exists() && !std::path::Path::new("/tmp/absolute").exists()
        );
    }
}

#[test]
fn a_zip_bomb_and_too_many_entries_stop_at_the_limits() {
    let into = tempfile::tempdir().expect("a folder");
    let zeros = vec![0u8; 64 << 20];
    let bomb = zip_of(
        &[Part::File("bin/zeros", 0o644, &zeros)],
        zip::CompressionMethod::Deflated,
    );
    assert!(
        bomb.metadata().expect("meta").len() < 1 << 20,
        "the bomb is small when packed"
    );
    let limits = Limits {
        bytes: 1 << 20,
        entries: 100,
    };
    assert_eq!(
        unpack_within(bomb, (ArchiveFormat::Zip, limits), into.path(), || true),
        Err(Cause::LimitDisk)
    );
    let names: Vec<String> = (0..11).map(|index| format!("f{index}")).collect();
    let parts: Vec<Part<'_>> = names
        .iter()
        .map(|name| Part::File(name, 0o644, b"x"))
        .collect();
    let many = Limits {
        bytes: 1 << 20,
        entries: 10,
    };
    let fresh = tempfile::tempdir().expect("a folder");
    assert_eq!(
        unpack_within(
            stored(&parts),
            (ArchiveFormat::Zip, many),
            fresh.path(),
            || true
        ),
        Err(Cause::LimitDisk)
    );
}

#[test]
fn an_entry_whose_bytes_break_their_check_is_a_rejected_archive() {
    let mut file = stored(&[Part::File("README", 0o644, b"the bytes of the entry")]);
    let mut bytes = Vec::new();
    std::io::Read::read_to_end(&mut file, &mut bytes).expect("read");
    let at = bytes
        .windows(5)
        .position(|window| window == b"bytes")
        .expect("the data");
    bytes[at] = b'B';
    let mut broken = tempfile::tempfile().expect("a file");
    broken.write_all(&bytes).expect("written");
    broken.rewind().expect("rewind");
    let into = tempfile::tempdir().expect("a folder");
    assert_eq!(
        unpack_within(
            broken,
            (ArchiveFormat::Zip, Limits::SECTION_6),
            into.path(),
            || true
        ),
        Err(Cause::ArchiveRejected)
    );
}

#[test]
fn the_disk_is_asked_again_while_one_large_entry_is_written() {
    let large = vec![0_u8; 65 << 20];
    let file = zip_of(
        &[Part::File("large", 0o644, &large)],
        zip::CompressionMethod::Deflated,
    );
    let asked = std::cell::Cell::new(0);
    let into = tempfile::tempdir().expect("a folder");
    let result = unpack_within(
        file,
        (ArchiveFormat::Zip, Limits::SECTION_6),
        into.path(),
        || {
            asked.set(asked.get() + 1);
            asked.get() == 1
        },
    );
    assert_eq!((result, asked.get()), (Err(Cause::LimitDisk), 2));
}
