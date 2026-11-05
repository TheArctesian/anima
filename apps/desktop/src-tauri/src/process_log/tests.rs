//! The folder tests run a harmless program in place of the file manager (`true`, `false`, `ls`),
//! so no test opens a Finder or file-manager window.

use std::{fs, path::PathBuf};

use tauri_plugin_log::log::Level;

use super::{
    ProcessLogError,
    folder::open_with,
    line::{Stamp, render},
};

/// A fresh, missing folder under the system temp folder for one test. Its name has a space,
/// so a test can show that the opener gets the folder as one argument.
fn scratch(name: &str) -> PathBuf {
    let root =
        std::env::temp_dir().join(format!("anima-process-log-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    root
}

const STAMP: Stamp = Stamp {
    date: (2026, 11, 4),
    time: (9, 5, 7),
};

#[test]
fn log_008_a_missing_folder_is_created_then_given_to_the_opener() {
    let root = scratch("created");
    let folder = root.join("ing.anima.desktop logs");
    // `ls` fails unless the folder exists and arrives as one argument.
    assert_eq!(open_with(&folder, "ls"), Ok(()));
    assert!(folder.is_dir());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn log_008_an_existing_folder_opens_and_keeps_its_files() {
    let root = scratch("existing");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("Anima.log"), "line\n").unwrap();
    assert_eq!(open_with(&root, "true"), Ok(()));
    assert_eq!(
        fs::read_to_string(root.join("Anima.log")).unwrap(),
        "line\n"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn log_008_an_opener_that_reports_failure_is_failed() {
    let root = scratch("refused");
    assert_eq!(open_with(&root, "false"), Err(ProcessLogError::Failed));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn log_008_a_missing_opener_is_unavailable() {
    let root = scratch("no-opener");
    assert_eq!(
        open_with(&root, "anima-no-such-file-manager"),
        Err(ProcessLogError::Unavailable)
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn log_008_a_folder_that_cannot_be_created_is_failed_and_the_opener_does_not_run() {
    let root = scratch("blocked");
    fs::create_dir_all(&root).unwrap();
    let file = root.join("a file");
    fs::write(&file, "").unwrap();
    // `true` would succeed if it ran; the create step must stop first.
    assert_eq!(
        open_with(&file.join("logs"), "true"),
        Err(ProcessLogError::Failed)
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn log_007_errors_serialise_as_code_and_fixed_message() {
    assert_eq!(
        serde_json::to_value(ProcessLogError::Unavailable).unwrap(),
        serde_json::json!({
            "code": "unavailable",
            "message": "the log folder cannot be opened on this system"
        })
    );
    assert_eq!(
        serde_json::to_value(ProcessLogError::Failed).unwrap(),
        serde_json::json!({ "code": "failed", "message": "the log folder did not open" })
    );
}

#[test]
fn log_007_an_error_from_a_real_path_carries_no_path_or_os_text() {
    let root = scratch("leak");
    fs::create_dir_all(&root).unwrap();
    let file = root.join("a file");
    fs::write(&file, "").unwrap();
    let errors = [
        open_with(&file.join("logs"), "true").unwrap_err(),
        open_with(&root, "anima-no-such-file-manager").unwrap_err(),
        open_with(&root, "false").unwrap_err(),
    ];
    for error in errors {
        let json = serde_json::to_string(&error).unwrap();
        let shown = format!("{error} {error:?}");
        for text in [json.as_str(), shown.as_str()] {
            assert!(!text.contains("anima-process-log"), "path leaked: {text}");
            assert!(!text.contains('/'), "path leaked: {text}");
            assert!(
                !text.to_lowercase().contains("os error"),
                "OS text leaked: {text}"
            );
        }
    }
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn log_008_a_line_has_the_utc_stamp_level_target_and_message() {
    assert_eq!(
        render(&STAMP, Level::Info, "webview", "update check failed"),
        "2026-11-04T09:05:07Z [INFO] [webview] update check failed"
    );
}

#[test]
fn log_008_a_message_cannot_forge_a_second_line() {
    let forged = "ok\n2026-11-04T09:05:08Z [ERROR] [anima] fake\r\u{1b}[2J\u{2028}x";
    let line = render(&STAMP, Level::Warn, "webview", forged);
    assert!(!line.contains(['\n', '\r', '\u{1b}', '\u{2028}']), "{line}");
    assert!(line.ends_with(r"ok\n2026-11-04T09:05:08Z [ERROR] [anima] fake\r\u{1b}[2J\u{2028}x"));
}

#[test]
fn log_008_a_web_view_target_cannot_forge_a_second_line() {
    let line = render(&STAMP, Level::Info, "webview::a\nb\tc", "message");
    assert_eq!(
        line,
        r"2026-11-04T09:05:07Z [INFO] [webview::a\nb\tc] message"
    );
}

#[test]
fn log_008_printable_text_is_kept_as_written() {
    let text = r#"Anima 0.1.42 is installed — "quoted" \ back-slash, 日本語, emoji-free"#;
    assert!(render(&STAMP, Level::Error, "anima_desktop", text).ends_with(text));
}
