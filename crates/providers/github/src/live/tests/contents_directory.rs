//! PRJ-030: the listing of a directory of the repository.

use super::{fixture, ok};
use crate::live::contents::{DirectoryRead, EntryKind, directory_parse};

fn listing(body: &[u8]) -> crate::live::contents::Directory {
    match directory_parse(&ok(body)).unwrap() {
        DirectoryRead::Directory(directory) => directory,
        DirectoryRead::NotADirectory => panic!("a listing was expected"),
    }
}

#[test]
fn prj_030_a_listing_has_directories_first_then_by_name() {
    let body = fixture("live_contents_directory.json");

    let directory = listing(&body);

    let names: Vec<_> = directory.entries.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "assets",
            "src",
            "Alpha.md",
            "alpha.md",
            "latest",
            "vendor-lib",
            "zeta.txt"
        ]
    );
    assert!(!directory.partial);
}

#[test]
fn prj_030_a_listing_entry_has_its_kind_size_path_and_sha() {
    let body = fixture("live_contents_directory.json");

    let directory = listing(&body);

    let find = |name: &str| directory.entries.iter().find(|e| e.name == name).unwrap();
    let file = find("Alpha.md");
    assert_eq!(file.kind, EntryKind::File);
    assert_eq!((file.size, file.path.as_str()), (340, "Alpha.md"));
    assert_eq!(file.sha, "b3".repeat(20));
    assert_eq!(find("src").kind, EntryKind::Directory);
    assert_eq!(find("src").size, 0);
}

#[test]
fn prj_030_a_submodule_typed_file_by_the_2022_version_is_a_submodule() {
    let body = fixture("live_contents_directory.json");

    let directory = listing(&body);

    let link = directory
        .entries
        .iter()
        .find(|e| e.name == "vendor-lib")
        .unwrap();
    assert_eq!(link.kind, EntryKind::Submodule);
    assert_eq!(link.size, 0);
    let symlink = directory
        .entries
        .iter()
        .find(|e| e.name == "latest")
        .unwrap();
    assert_eq!(symlink.kind, EntryKind::Symlink);
    assert_eq!(symlink.size, 0, "only a file has a size");
}

#[test]
fn prj_030_a_listing_of_a_thousand_entries_may_be_cut() {
    let entry = |n: usize| {
        serde_json::json!({
            "type": "file", "name": format!("f{n:04}"), "path": format!("f{n:04}"),
            "size": 1, "sha": "d".repeat(40), "download_url": "https://raw.example/x"
        })
    };
    let full: Vec<_> = (0..1_000).map(entry).collect();
    let short: Vec<_> = (0..999).map(entry).collect();

    assert!(listing(&serde_json::to_vec(&full).unwrap()).partial);
    assert!(!listing(&serde_json::to_vec(&short).unwrap()).partial);
    assert!(!listing(b"[]").partial);
}

#[test]
fn prj_030_a_file_answer_to_a_directory_request_is_a_typed_outcome() {
    let body = fixture("live_contents_file.json");

    assert_eq!(
        directory_parse(&ok(&body)).unwrap(),
        DirectoryRead::NotADirectory
    );
    let link = fixture("live_contents_symlink.json");
    assert_eq!(
        directory_parse(&ok(&link)).unwrap(),
        DirectoryRead::NotADirectory
    );
}

#[test]
fn prj_030_a_listing_that_is_not_readable_is_a_problem() {
    use source_exchange::{Body, Response};

    assert_eq!(
        directory_parse(&ok(b"nope")).unwrap_err().code,
        "reply_unreadable"
    );
    let too_large = Response {
        status: 200,
        body: Body::TooLarge,
    };
    assert_eq!(
        directory_parse(&too_large).unwrap_err().code,
        "reply_too_large"
    );
    let missing = Response {
        status: 404,
        body: Body::Read(b"{}"),
    };
    assert_eq!(
        directory_parse(&missing).unwrap_err().code,
        "unexpected_status"
    );
}
