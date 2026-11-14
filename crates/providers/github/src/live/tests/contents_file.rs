//! PRJ-030: the reading of a file of the repository.

use base64::{Engine as _, engine::general_purpose::STANDARD};
use source_exchange::{Body, Response};

use super::{fixture, ok};
use crate::live::contents::{FileRead, file_parse};

/// A file object as GitHub sends it, with `content` as given.
fn file_body(size: u64, content: &str) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({
        "type": "file", "encoding": "base64", "name": "f.txt", "path": "d/f.txt",
        "size": size, "sha": "e".repeat(40), "content": content,
        "download_url": "https://raw.example/d/f.txt"
    }))
    .unwrap()
}

fn read(body: &[u8]) -> FileRead {
    file_parse(&ok(body)).unwrap()
}

#[test]
fn prj_030_a_text_file_is_decoded_from_wrapped_base64() {
    let body = fixture("live_contents_file.json");

    let FileRead::Text { file, text } = read(&body) else {
        panic!("a text file was expected");
    };

    assert_eq!(file.name, "README.md");
    assert_eq!(file.path, "docs/README.md");
    assert_eq!(file.sha, "a1".repeat(20));
    assert_eq!(file.size, text.len() as u64);
    assert!(text.starts_with("# Sample\n\nHello, world. Caf\u{e9} \u{2603} notes.\n"));
    assert!(text.ends_with("long enough to wrap.\n"));
}

#[test]
fn prj_030_base64_with_newlines_and_returns_decodes_the_same() {
    let plain = STANDARD.encode("one\ntwo");
    let wrapped = format!("{}\r\n{}\n", &plain[..4], &plain[4..]);

    let FileRead::Text { text, .. } = read(&file_body(7, &wrapped)) else {
        panic!("a text file was expected");
    };

    assert_eq!(text, "one\ntwo");
}

#[test]
fn prj_030_an_empty_file_is_empty_text() {
    let FileRead::Text { file, text } = read(&file_body(0, "")) else {
        panic!("a text file was expected");
    };

    assert_eq!((file.size, text.as_str()), (0, ""));
}

#[test]
fn prj_030_bytes_that_are_not_utf8_or_hold_a_nul_are_binary() {
    let not_utf8 = STANDARD.encode([0xff, 0xfe, 0x41]);
    let with_nul = STANDARD.encode(b"text\0more");

    for content in [not_utf8, with_nul] {
        let FileRead::Binary(file) = read(&file_body(9, &content)) else {
            panic!("a binary file was expected");
        };
        assert_eq!(file.path, "d/f.txt");
    }
}

#[test]
fn prj_030_a_file_over_a_million_bytes_is_too_large_without_decoding() {
    let content = STANDARD.encode("small");

    let FileRead::TooLarge(Some(file)) = read(&file_body(1_000_001, &content)) else {
        panic!("too large was expected");
    };
    assert_eq!(file.size, 1_000_001);
    assert!(matches!(
        read(&file_body(1_000_000, &content)),
        FileRead::Text { .. }
    ));
}

#[test]
fn prj_030_empty_content_with_a_size_is_too_large() {
    assert!(matches!(
        read(&file_body(5_000_000, "")),
        FileRead::TooLarge(Some(_))
    ));
}

#[test]
fn prj_030_a_reply_cut_by_the_transport_is_too_large() {
    let cut = Response {
        status: 200,
        body: Body::TooLarge,
    };

    assert_eq!(file_parse(&cut).unwrap(), FileRead::TooLarge(None));
}

#[test]
fn prj_030_a_content_that_lies_about_its_size_is_still_too_large() {
    let content = STANDARD.encode(vec![b'a'; 1_000_001]);

    assert!(matches!(
        read(&file_body(10, &content)),
        FileRead::TooLarge(Some(_))
    ));
}

#[test]
fn prj_030_a_directory_answer_to_a_file_request_is_a_typed_outcome() {
    let body = fixture("live_contents_directory.json");

    assert_eq!(read(&body), FileRead::Directory);
}

#[test]
fn prj_030_a_symlink_is_reported_with_its_target_and_never_followed() {
    let body = fixture("live_contents_symlink.json");

    assert_eq!(
        read(&body),
        FileRead::Symlink {
            name: "latest".to_owned(),
            path: "latest".to_owned(),
            target: "zeta.txt".to_owned(),
        }
    );
}

#[test]
fn prj_030_a_submodule_is_reported_with_its_commit() {
    let body = fixture("live_contents_submodule.json");

    assert_eq!(
        read(&body),
        FileRead::Submodule {
            name: "vendor-lib".to_owned(),
            path: "vendor-lib".to_owned(),
            sha: "b4".repeat(20),
            git_url: Some("https://example.invalid/sample/vendor-lib.git".to_owned()),
        }
    );
}

#[test]
fn prj_030_content_that_cannot_be_read_is_a_problem_with_fixed_words() {
    let garbage = file_body(4, "!!!! not base64 !!!!");
    let other_encoding = serde_json::to_vec(&serde_json::json!({
        "type": "file", "encoding": "utf-16", "name": "f", "path": "f", "size": 1,
        "sha": "e".repeat(40), "content": "QQ==", "download_url": "https://raw.example/f"
    }))
    .unwrap();
    let unknown_type = br#"{"type":"blob","name":"f","path":"f","sha":"x"}"#;

    for body in [garbage.as_slice(), other_encoding.as_slice(), unknown_type] {
        let problem = file_parse(&ok(body)).unwrap_err();
        assert_eq!(problem.code, "reply_unreadable");
        assert!(!problem.message.contains("base64 !!!!"));
    }
    let missing = Response {
        status: 404,
        body: Body::Read(b"{}"),
    };
    assert_eq!(file_parse(&missing).unwrap_err().code, "unexpected_status");
}
