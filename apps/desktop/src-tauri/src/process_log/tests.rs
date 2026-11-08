//! The folder tests run a harmless program in place of the file manager (`true`, `false`, `ls`
//! or a small script), so no test opens a Finder or file-manager window.

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use tauri_plugin_log::log::{Level, Record};

use crate::os_program::OPENER_PATIENCE as PATIENCE;

use super::{
    ProcessLogError,
    folder::open_with,
    line::{Stamp, line_of},
};

/// A fresh, missing folder under the system temp folder for one test.
fn scratch(name: &str) -> PathBuf {
    let root =
        std::env::temp_dir().join(format!("anima-process-log-{}-{name}", std::process::id()));
    // A folder left by an earlier run is removed; a missing one is the expected case.
    let _ = fs::remove_dir_all(&root);
    root
}

fn open(folder: &Path, opener: &str) -> Result<(), ProcessLogError> {
    open_with(folder, opener, PATIENCE)
}

/// Writes a `sh` script to `path` with the given mode and returns it as an opener.
fn script(path: &Path, body: &str, mode: u32) -> String {
    fs::write(path, format!("#!/bin/sh\n{body}\n")).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
    path.to_str().unwrap().to_owned()
}

const STAMP: Stamp = Stamp {
    date: (2026, 11, 4),
    time: (9, 5, 7),
};

/// The line for a record built the way the `log` macros and the `plugin:log|log` command do.
fn line(level: Level, target: &str, message: &str) -> String {
    let args = format_args!("{message}");
    line_of(
        &STAMP,
        &Record::builder()
            .level(level)
            .target(target)
            .args(args)
            .build(),
        &args,
    )
}

#[test]
fn log_008_a_missing_folder_is_created_then_given_to_the_opener() {
    let root = scratch("created");
    // The space shows the opener gets the folder as one argument.
    let folder = root.join("ing.anima.desktop logs");
    // `ls` fails unless the folder exists and arrives as one argument.
    assert_eq!(open(&folder, "ls"), Ok(()));
    assert!(folder.is_dir());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn log_008_an_existing_folder_opens_and_keeps_its_files() {
    let root = scratch("existing");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("Anima.log"), "line\n").unwrap();
    assert_eq!(open(&root, "true"), Ok(()));
    assert_eq!(
        fs::read_to_string(root.join("Anima.log")).unwrap(),
        "line\n"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn log_008_an_opener_that_reports_failure_is_failed() {
    let root = scratch("refused");
    assert_eq!(open(&root, "false"), Err(ProcessLogError::Failed));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn log_008_a_missing_opener_is_unavailable() {
    let root = scratch("no-opener");
    assert_eq!(
        open(&root, "anima-no-such-file-manager"),
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
        open(&file.join("logs"), "true"),
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
        open(&file.join("logs"), "true").unwrap_err(),
        open(&root, "anima-no-such-file-manager").unwrap_err(),
        open(&root, "false").unwrap_err(),
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
fn log_008_an_opener_that_is_not_executable_is_failed() {
    let root = scratch("not-executable");
    fs::create_dir_all(&root).unwrap();
    let opener = script(&root.join("opener"), "exit 0", 0o644);
    assert_eq!(open(&root, &opener), Err(ProcessLogError::Failed));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn log_008_the_exit_status_of_a_slow_opener_within_patience_decides() {
    let root = scratch("slow");
    fs::create_dir_all(&root).unwrap();
    let refuses = script(&root.join("refuses"), "sleep 0.2; exit 1", 0o755);
    let accepts = script(&root.join("accepts"), "sleep 0.2; exit 0", 0o755);
    assert_eq!(open(&root, &refuses), Err(ProcessLogError::Failed));
    assert_eq!(open(&root, &accepts), Ok(()));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn log_008_an_opener_still_running_after_patience_succeeds_without_waiting_for_it() {
    let root = scratch("foreground");
    fs::create_dir_all(&root).unwrap();
    let foreground = script(&root.join("file manager"), "exec sleep 10", 0o755);
    let started = Instant::now();
    assert_eq!(
        open_with(&root, &foreground, Duration::from_millis(100)),
        Ok(())
    );
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "{:?}",
        started.elapsed()
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn log_008_a_line_has_the_utc_stamp_level_target_and_message() {
    assert_eq!(
        line(Level::Info, "webview", "update check failed"),
        "2026-11-04T09:05:07Z [INFO] [webview] update check failed"
    );
}

#[test]
fn log_008_a_web_view_record_cannot_forge_a_second_line() {
    let rendered = line_of(
        &STAMP,
        &Record::builder()
            .level(Level::Warn)
            .target("webview::a\nb")
            .args(format_args!("x\ny"))
            .build(),
        &format_args!("x\ny"),
    );
    assert_eq!(
        rendered,
        r"2026-11-04T09:05:07Z [WARN] [webview::a\nb] x\ny"
    );
}

#[test]
fn log_008_a_message_cannot_forge_a_second_line() {
    let forged =
        "ok\n2026-11-04T09:05:08Z [ERROR] [anima] fake\r\u{1b}[2J\u{85}\u{9b}2J\u{2028}\u{2029}x";
    let rendered = line(Level::Warn, "webview", forged);
    assert!(
        !rendered.contains([
            '\n', '\r', '\u{1b}', '\u{85}', '\u{9b}', '\u{2028}', '\u{2029}'
        ]),
        "{rendered}"
    );
    assert!(rendered.ends_with(
        r"ok\n2026-11-04T09:05:08Z [ERROR] [anima] fake\r\u{1b}[2J\u{85}\u{9b}2J\u{2028}\u{2029}x"
    ));
}

#[test]
fn log_008_bidi_controls_are_escaped_so_a_line_cannot_be_reordered() {
    let bidi = "\u{202a}\u{202b}\u{202c}\u{202d}\u{202e}\u{2066}\u{2067}\u{2068}\u{2069}";
    let rendered = line(Level::Info, bidi, bidi);
    let escaped = r"\u{202a}\u{202b}\u{202c}\u{202d}\u{202e}\u{2066}\u{2067}\u{2068}\u{2069}";
    assert_eq!(
        rendered,
        format!("2026-11-04T09:05:07Z [INFO] [{escaped}] {escaped}")
    );
}

#[test]
fn log_008_a_web_view_target_cannot_forge_a_second_line() {
    assert_eq!(
        line(Level::Info, "webview::a\nb\tc", "message"),
        r"2026-11-04T09:05:07Z [INFO] [webview::a\nb\tc] message"
    );
}

#[test]
fn log_008_printable_text_is_kept_as_written() {
    let text = r#"Anima 0.1.42 is installed — "quoted" \ back-slash, 日本語, tabs-free"#;
    assert!(line(Level::Error, "anima_desktop", text).ends_with(text));
}
