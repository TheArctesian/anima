//! The Windows application registers the same commands as the other desktop applications
//! (WIN1). The gates of a branch run on Linux, where `desktop/windows.rs` is not compiled; a
//! command that only `desktop.rs` registers leaves its module unused on Windows, and that build
//! fails with `-D warnings` only in the Windows job of `dev` (seen 2026-11-20 and 2026-11-21).

use std::collections::BTreeSet;

/// Commands that only the macOS and Linux application registers, each with its reason.
const NOT_ON_WINDOWS: [(&str, &str); 1] = [(
    "system_settings::sharing_settings_open",
    "opens the sharing settings of macOS or Linux; module system_settings is Unix only",
)];

/// Commands that only the Windows application registers, each with its reason.
const ONLY_ON_WINDOWS: [(&str, &str); 0] = [];

/// The paths of the commands in the `generate_handler!` list of `source`.
fn commands(source: &str) -> BTreeSet<&str> {
    let start = source.find("generate_handler![").expect("a command list");
    let list = &source[start..];
    let end = list.find(']').expect("the end of the command list");
    list["generate_handler![".len()..end]
        .split(',')
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .collect()
}

#[test]
fn win1_windows_registers_each_desktop_command_but_the_named_exceptions() {
    let desktop = commands(include_str!("../desktop.rs"));
    let windows = commands(include_str!("windows.rs"));
    let missing: Vec<_> = desktop
        .difference(&windows)
        .filter(|path| !NOT_ON_WINDOWS.iter().any(|(name, _)| name == *path))
        .collect();
    let extra: Vec<_> = windows
        .difference(&desktop)
        .filter(|path| !ONLY_ON_WINDOWS.iter().any(|(name, _)| name == *path))
        .collect();
    assert!(
        missing.is_empty(),
        "register these in desktop/windows.rs, or name them in NOT_ON_WINDOWS with the reason: {missing:?}"
    );
    assert!(
        extra.is_empty(),
        "register these in desktop.rs, or name them in ONLY_ON_WINDOWS with the reason: {extra:?}"
    );
}
