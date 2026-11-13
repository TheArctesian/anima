//! Starting the bundled `anima` through the `AppImage` (INS-005, OD-18). The `AppImage` runtime runs
//! `AppRun`, which runs the app's own executable with the arguments; it cannot start another
//! program. So the launcher passes [`FLAG`] first, and the app, before it opens a window or
//! touches a display, replaces itself with the `anima` beside it. `exec` keeps the process:
//! standard input, output and error, signals and the exit code are the command line's own.

use std::{
    ffi::{OsStr, OsString},
    path::PathBuf,
};

/// The first argument that makes the app run its `anima` in place of the window.
pub const FLAG: &str = "--anima-cli";

/// The arguments for `anima` when `args` (the app's own, the program name first) start with
/// [`FLAG`]; `None` for an ordinary start.
pub fn cli_args(args: &[OsString]) -> Option<&[OsString]> {
    match args {
        [_, flag, rest @ ..] if flag == FLAG => Some(rest),
        _ => None,
    }
}

/// The folder the command was typed in. The old `AppRun` changes to `$APPDIR/usr` and keeps the
/// first folder in `OWD`, which the `AppImage` runtime also sets; a relative path in an argument
/// must be read from there.
pub fn typed_in(owd: Option<&OsStr>) -> Option<PathBuf> {
    owd.map(PathBuf::from).filter(|folder| folder.is_absolute())
}

/// Replaces this process with the bundled `anima` when the app was started with [`FLAG`].
/// Returns for an ordinary start. Never returns after the flag: a failure to start `anima` is
/// told on standard error with exit code 127, as `sh` does for a missing program. Only the Linux
/// launcher passes the flag; on macOS the link starts `anima` itself.
pub fn run_if_asked() {
    use std::os::unix::process::CommandExt;

    let args: Vec<OsString> = std::env::args_os().collect();
    let Some(rest) = cli_args(&args) else {
        return;
    };
    let program = std::env::current_exe()
        .ok()
        .and_then(|executable| super::carrier::beside(&executable))
        .filter(|program| program.is_file());
    let Some(program) = program else {
        eprintln!("anima: this copy of Anima carries no command line program.");
        std::process::exit(127);
    };
    let mut command = std::process::Command::new(program);
    command.arg0(super::carrier::PROGRAM).args(rest);
    if let Some(folder) = typed_in(std::env::var_os("OWD").as_deref()) {
        command.current_dir(folder);
    }
    let error = command.exec();
    eprintln!("anima: the command line program did not start: {error}");
    std::process::exit(127);
}
