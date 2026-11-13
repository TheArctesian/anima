//! `anima files ls` and `anima files get` (K4 Section 9): each opens an SSH session and its
//! terminal, then the files tunnel, and ends the session after.

use serde_json::json;

use super::{DEVICE, LOGIN_TOKEN, ended, files_opened, started};
use crate::kit::Anima;
use crate::kit::tunnel_hub::{FILES_TICKET, Files, TERMINAL_TICKET, Terminal, TunnelHub};

async fn hub_with(files: Files) -> TunnelHub {
    let hub = TunnelHub::start(Terminal::Hold, files).await;
    hub.answer("SessionStart", started(false))
        .answer("SessionOpenFiles", files_opened())
        .answer("SessionEnd", ended("OPERATOR"));
    hub
}

fn signed_in() -> Anima {
    Anima::new().with("ANIMA_TOKEN", LOGIN_TOKEN)
}

#[tokio::test]
async fn k4_9_files_ls_lists_a_folder_and_cleans_device_names() {
    let listing = json!({ "type": "entries", "path": "/home/op", "entries": [
        { "name": "notes.txt", "kind": "file", "size": 12, "modifiedAt": "2026-11-01T09:00:00Z" },
        { "name": "evil\u{1b}]0;owned\u{7}", "kind": "dir", "size": null, "modifiedAt": null },
    ]});
    let hub = hub_with(Files {
        listing,
        ..Files::default()
    })
    .await;
    let remote = format!("{DEVICE}:/home/op");
    let run = signed_in()
        .run(&["files", "ls", &remote, "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert!(run.stdout.contains("notes.txt"), "{run:?}");
    assert!(!run.stdout.contains('\u{1b}'), "{run:?}");
    let received = hub.received();
    assert!(
        received.contains(&r#"{"type":"list","path":"/home/op"}"#.to_owned()),
        "{received:?}"
    );
    assert_eq!(
        hub.operations(),
        vec!["SessionStart", "SessionOpenFiles", "SessionEnd"]
    );
    assert_eq!(
        hub.queries(),
        vec![
            format!("token={TERMINAL_TICKET}"),
            format!("token={FILES_TICKET}")
        ]
    );
    run.assert_never_shows(FILES_TICKET);
}

#[tokio::test]
async fn k4_9_files_get_saves_the_file_and_never_overwrites_without_force() {
    let hub = hub_with(Files {
        file: b"line one\nline two\n".to_vec(),
        ..Files::default()
    })
    .await;
    let folder = tempfile::tempdir().expect("a folder");
    let target = folder.path().join("hosts");
    let remote = format!("{DEVICE}:/etc/hosts");
    let local = folder.path().to_string_lossy().to_string();
    let run = signed_in()
        .run(&["files", "get", &remote, &local, "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert_eq!(
        std::fs::read(&target).expect("saved"),
        b"line one\nline two\n"
    );

    std::fs::write(&target, b"mine").expect("written");
    let again = signed_in()
        .run(&["files", "get", &remote, &local, "--hub", &hub.origin])
        .await;
    assert_eq!(again.code, 6, "{again:?}");
    assert!(again.stderr.contains("--force"), "{again:?}");
    assert_eq!(std::fs::read(&target).expect("kept"), b"mine");
    assert_eq!(
        hub.operations().len(),
        3,
        "the second run asks the hub nothing"
    );
}

#[tokio::test]
async fn k4_9_a_device_name_that_climbs_out_is_refused_before_any_request() {
    let hub = hub_with(Files::default()).await;
    let folder = tempfile::tempdir().expect("a folder");
    let local = folder.path().to_string_lossy().to_string();
    let remote = format!("{DEVICE}:/srv/..");
    let run = signed_in()
        .run(&["files", "get", &remote, &local, "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 2, "{run:?}");
    assert_eq!(hub.operations(), Vec::<String>::new());
}

#[tokio::test]
async fn ra_032_a_policy_refusal_is_exit_5_and_the_session_still_ends() {
    let hub = hub_with(Files {
        refuse: Some("DOWNLOAD_OFF"),
        ..Files::default()
    })
    .await;
    let folder = tempfile::tempdir().expect("a folder");
    let local = folder.path().join("out").to_string_lossy().to_string();
    let remote = format!("{DEVICE}:/etc/hosts");
    let run = signed_in()
        .run(&["files", "get", &remote, &local, "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 5, "{run:?}");
    assert!(run.stderr.contains("does not allow downloads"), "{run:?}");
    assert!(!folder.path().join("out").exists());
    assert_eq!(
        hub.operations().last().map(String::as_str),
        Some("SessionEnd")
    );
}

#[tokio::test]
async fn k4_9_files_put_sends_exactly_the_declared_size_into_the_folder() {
    let hub = hub_with(Files::default()).await;
    let folder = tempfile::tempdir().expect("a folder");
    let source = folder.path().join("report.txt");
    let content = "x".repeat(70_000);
    std::fs::write(&source, &content).expect("written");
    let local = source.to_string_lossy().to_string();
    let remote = format!("{DEVICE}:/tmp/");
    let run = signed_in()
        .run(&[
            "files",
            "put",
            &local,
            &remote,
            "--json",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert_eq!(run.json()["path"], "/tmp/report.txt");
    let received = hub.received();
    assert!(
        received.contains(&r#"{"type":"upload","path":"/tmp/report.txt","size":70000}"#.to_owned()),
        "{received:?}"
    );
    let sent: usize = received
        .iter()
        .filter_map(|message| message.strip_prefix("binary:"))
        .map(str::len)
        .sum();
    assert_eq!(sent, 70_000);
}
