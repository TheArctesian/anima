//! The client messages (K4 Section 9): each accepted message, every refused shape, and a
//! property test that no input panics and only the Section 9 set is accepted; the hub's
//! messages; and which directory entries a listing shows.

use proptest::prelude::*;
use russh_sftp::protocol::{File, FileAttributes};
use serde_json::{Value, json};

use crate::sessions::files::{
    device::{entry, kind},
    protocol::{
        BadRequest, DONE, DevicePath, Entry, Kind, Listing, MAX_PATH_BYTES, MAX_TEXT_BYTES,
        Refusal, Request, download, entries, parse, refused,
    },
};

fn path(text: &str) -> DevicePath {
    match parse(&format!(r#"{{"type":"list","path":{}}}"#, json!(text))) {
        Ok(Request::List(path)) => path,
        other => panic!("{other:?}"),
    }
}

#[test]
fn ra_032_each_client_message_is_accepted() {
    assert_eq!(
        parse(r#"{"type":"list","path":"/home"}"#),
        Ok(Request::List(path("/home")))
    );
    assert_eq!(
        parse(r#"{"type":"download","path":"/a.txt"}"#),
        Ok(Request::Download(path("/a.txt")))
    );
    assert_eq!(
        parse(r#" { "type" : "upload", "size" : 12, "path" : "/b" } "#),
        Ok(Request::Upload {
            path: path("/b"),
            size: 12
        })
    );
    assert_eq!(parse(r#"{"type":"cancel"}"#), Ok(Request::Cancel));
    assert_eq!(parse(r#"{"type":"ping"}"#), Ok(Request::Ping));
    assert_eq!(
        parse(&format!(
            r#"{{"type":"upload","path":"/c","size":{}}}"#,
            u64::MAX
        )),
        Ok(Request::Upload {
            path: path("/c"),
            size: u64::MAX
        })
    );
}

#[test]
fn ra_032_every_other_shape_is_refused() {
    for text in [
        r#"{"type":"resize","cols":80,"rows":24}"#,
        r#"{"type":"list"}"#,
        r#"{"type":"list","path":"/a","extra":1}"#,
        r#"{"type":"list","path":"/a","path":"/b"}"#,
        r#"{"type":"ping","type":"ping"}"#,
        r#"{"type":"cancel","path":"/a"}"#,
        r#"{"type":"upload","path":"/a"}"#,
        r#"{"type":"upload","path":"/a","size":-1}"#,
        r#"{"type":"upload","path":"/a","size":1.5}"#,
        r#"{"type":"upload","path":"/a","size":1e3}"#,
        r#"{"type":"upload","path":"/a","size":"12"}"#,
        r#"{"type":"upload","path":"/a","size":18446744073709551616}"#,
        r#"{"type":"list","path":7}"#,
        r#"["list","/a"]"#,
        r#""ping""#,
        r#"{"type":"PING"}"#,
        "",
        "{",
    ] {
        assert_eq!(parse(text), Err(BadRequest::Unknown), "{text}");
    }
}

#[test]
fn ra_032_paths_are_not_empty_hold_no_nul_and_fit_4096_bytes() {
    let wrapping = r#"{"type":"list","path":""}"#.len();
    let longest = format!("/{}", "a".repeat(MAX_TEXT_BYTES - wrapping - 1));
    assert_eq!(path(&longest).as_str(), longest);
    assert!(longest.len() < MAX_PATH_BYTES);
    let longer = format!(r#"{{"type":"list","path":"{longest}a"}}"#);
    assert_eq!(parse(&longer), Err(BadRequest::TooLong));
    for bad in [String::new(), "/a\u{0}b".to_owned()] {
        let text = format!(r#"{{"type":"download","path":{}}}"#, json!(bad));
        assert_eq!(parse(&text), Err(BadRequest::BadPath));
    }
    // A lone surrogate is not UTF-8.
    assert_eq!(
        parse(r#"{"type":"list","path":"/\ud800"}"#),
        Err(BadRequest::Unknown)
    );
}

#[test]
fn ra_032_a_text_message_over_4_kib_is_refused() {
    let padded = format!(r#"{{"type":"ping"}}{}"#, " ".repeat(MAX_TEXT_BYTES));
    assert_eq!(parse(&padded), Err(BadRequest::TooLong));
    let fits = format!(r#"{{"type":"ping"}}{}"#, " ".repeat(MAX_TEXT_BYTES - 15));
    assert_eq!(fits.len(), MAX_TEXT_BYTES);
    assert_eq!(parse(&fits), Ok(Request::Ping));
}

#[test]
fn log_007_a_path_debugs_as_its_length() {
    assert_eq!(format!("{:?}", path("/etc/secret")), "DevicePath(11 bytes)");
}

#[test]
fn ra_032_the_hub_messages_have_k4_shapes() {
    let listing = Listing {
        path: "/home".to_owned(),
        entries: vec![Entry {
            name: "a".to_owned(),
            kind: Kind::Link,
            size: None,
            modified_at: chrono::DateTime::from_timestamp(0, 0),
        }],
    };
    let sent: Value = serde_json::from_str(&entries(&listing)).unwrap();
    assert_eq!(
        sent,
        json!({"type": "entries", "path": "/home", "entries": [
            {"name": "a", "kind": "link", "size": null, "modifiedAt": "1970-01-01T00:00:00Z"}
        ]})
    );
    assert_eq!(download(7), r#"{"type":"download","size":7}"#);
    assert_eq!(DONE, r#"{"type":"done"}"#);
    for (reason, name) in [
        (Refusal::DownloadOff, "DOWNLOAD_OFF"),
        (Refusal::UploadOff, "UPLOAD_OFF"),
        (Refusal::NotFound, "NOT_FOUND"),
        (Refusal::Denied, "DENIED"),
        (Refusal::Exists, "EXISTS"),
    ] {
        assert_eq!(
            refused(reason),
            format!(r#"{{"type":"refused","reason":"{name}"}}"#)
        );
        assert_eq!(reason.as_str(), name);
    }
}

fn file(name: &str, mode: Option<u32>) -> File {
    let attrs = FileAttributes {
        size: Some(3),
        permissions: mode,
        mtime: Some(86_400),
        ..FileAttributes::default()
    };
    File::new(name, attrs)
}

#[test]
fn ra_032_a_listing_shows_files_directories_and_links_only() {
    assert_eq!(kind(&file("a", Some(0o100_644)).attrs), Some(Kind::File));
    assert_eq!(kind(&file("a", Some(0o040_755)).attrs), Some(Kind::Dir));
    assert_eq!(kind(&file("a", Some(0o120_777)).attrs), Some(Kind::Link));
    for other in [Some(0o140_755), Some(0o010_644), Some(0o020_644), None] {
        assert_eq!(kind(&file("a", other).attrs), None, "{other:?}");
    }
    let shown = entry(file("notes.txt", Some(0o100_644))).unwrap();
    assert_eq!(shown.name, "notes.txt");
    assert_eq!(shown.size, Some(3));
    assert_eq!(
        shown.modified_at,
        chrono::DateTime::from_timestamp(86_400, 0)
    );
    let long = "n".repeat(1025);
    for hidden in [".", "..", "", "a/b", "a\u{0}b", long.as_str()] {
        assert_eq!(entry(file(hidden, Some(0o100_644))), None, "{hidden:?}");
    }
    assert!(entry(file(&"n".repeat(1024), Some(0o100_644))).is_some());
}

/// The client messages, written as a client may write them.
fn message() -> impl Strategy<Value = (String, Request)> {
    let text_path = "/[a-z0-9 ._-]{0,40}";
    prop_oneof![
        text_path.prop_map(|p| (
            format!(r#"{{"type":"list","path":{}}}"#, json!(p)),
            Request::List(path(&p))
        )),
        text_path.prop_map(|p| (
            format!(r#"{{"type":"download","path":{}}}"#, json!(p)),
            Request::Download(path(&p))
        )),
        (text_path, any::<u64>()).prop_map(|(p, size)| (
            format!(r#"{{"size":{size},"path":{},"type":"upload"}}"#, json!(p)),
            Request::Upload {
                path: path(&p),
                size
            }
        )),
        Just((r#"{"type":"cancel"}"#.to_owned(), Request::Cancel)),
        Just((r#"{"type":"ping"}"#.to_owned(), Request::Ping)),
    ]
}

proptest! {
    #[test]
    fn ra_032_no_input_panics_and_only_k4_messages_pass(text in any::<String>()) {
        if parse(&text).is_ok() {
            let value: Value = serde_json::from_str(&text).unwrap();
            let kind = value["type"].as_str().unwrap();
            prop_assert!(["list", "download", "upload", "cancel", "ping"].contains(&kind));
        }
    }

    #[test]
    fn ra_032_written_messages_parse_as_written((text, request) in message()) {
        prop_assert_eq!(parse(&text), Ok(request));
    }
}
