//! The Linux launcher (INS-005, OD-18): a short `sh` script named `anima` that runs the `AppImage`
//! file with [`super::dispatch::FLAG`], so the app starts its bundled `anima` in place of the
//! window. The `AppImage` mounts itself at a new path on every start, so the launcher names the
//! file, never the mount. The updater writes a new `AppImage` to the same path (`$APPIMAGE`), so
//! the launcher keeps working after an update. Pure.

use std::path::{Path, PathBuf};

use super::dispatch::FLAG;

/// The second line of every launcher this app writes: how a launcher is known as the app's own.
const MARK: &str = "# anima launcher, written by the Anima desktop application (INS-005).";
/// The line that names the `AppImage`, before its quoted path.
const FILE_LINE: &str = "appimage=";
/// The largest launcher this app writes is far below this; a larger file is not one.
pub const SIZE_LIMIT: u64 = 4096;

/// `text` in single quotes for `sh`: a `'` inside becomes `'\''`.
fn quoted(text: &str) -> String {
    format!("'{}'", text.replace('\'', r"'\''"))
}

/// The text [`quoted`] made, or `None` for anything it would not make.
fn unquoted(text: &str) -> Option<String> {
    let mut out = String::new();
    let mut rest = text;
    while !rest.is_empty() {
        if let Some(after) = rest.strip_prefix(r"\'") {
            out.push('\'');
            rest = after;
            continue;
        }
        let inner = rest.strip_prefix('\'')?;
        let end = inner.find('\'')?;
        out.push_str(&inner[..end]);
        rest = &inner[end + 1..];
    }
    Some(out)
}

/// The launcher for the `AppImage` at `appimage`, or `None` when the path is not absolute UTF-8
/// free of control characters: such a path could not be shown or checked as written.
pub fn script(appimage: &Path) -> Option<String> {
    let path = appimage.to_str()?;
    if !appimage.is_absolute() || path.chars().any(char::is_control) {
        return None;
    }
    Some(format!(
        "#!/bin/sh\n\
         {MARK}\n\
         # It runs the anima program inside the AppImage below. Remove it in Anima:\n\
         # Settings > Updates > Command line tool.\n\
         {FILE_LINE}{}\n\
         if [ ! -f \"$appimage\" ]; then\n  \
         echo \"anima: the Anima AppImage is not at $appimage. It was moved or deleted. Open Anima and install the command line tool again (Settings > Updates).\" >&2\n  \
         exit 127\n\
         fi\n\
         exec \"$appimage\" {FLAG} \"$@\"\n",
        quoted(path)
    ))
}

/// The `AppImage` a launcher this app wrote runs, or `None` when `text` is not such a launcher.
/// The mark and the `appimage=` line decide, so a launcher an older version wrote with other
/// words is still the app's own to replace or remove.
pub fn appimage_of(text: &str) -> Option<PathBuf> {
    let mut lines = text.lines();
    if lines.next() != Some("#!/bin/sh") || lines.next() != Some(MARK) {
        return None;
    }
    let path = lines.find_map(|line| line.strip_prefix(FILE_LINE))?;
    Some(PathBuf::from(unquoted(path)?))
}
