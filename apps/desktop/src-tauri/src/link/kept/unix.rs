//! Unix file placement for the persistent device link.
use super::{DAEMON_SOCKET, LONGEST_SOCKET};
use std::{
    ffi::OsString,
    fs::{self, DirBuilder},
    os::unix::fs::{DirBuilderExt, MetadataExt},
    path::{Path, PathBuf},
};

/// The folder of the daemon of this user, where the daemon itself looks for it: on macOS
/// `~/Library/Application Support/anima-daemon`, else `$XDG_CONFIG_HOME/anima-daemon` or
/// `~/.config/anima-daemon`.
pub fn daemon_folder(home: &Path, macos: bool, config_home: Option<&Path>) -> PathBuf {
    if macos {
        return home.join("Library/Application Support/anima-daemon");
    }
    config_home
        .filter(|folder| folder.is_absolute())
        .map_or_else(|| home.join(".config"), Path::to_path_buf)
        .join("anima-daemon")
}

/// The socket for the daemon in `folder`, once the folder is there and private: it is made
/// with mode 0700 when it is absent. None when the folder is a link, belongs to a different
/// user or gives its group or others any access, and when the path is too long for a socket:
/// the link would refuse it, and the link must still start.
pub fn private_socket(folder: &Path) -> Option<PathBuf> {
    let _ = DirBuilder::new().recursive(true).mode(0o700).create(folder);
    let found = fs::symlink_metadata(folder).ok()?;
    let private = found.is_dir()
        && found.uid() == nix::unistd::getuid().as_raw()
        // No bit of the group or of others: the last two octal digits are zero.
        && found.mode() % 0o100 == 0;
    let socket = folder.join(DAEMON_SOCKET);
    (private && socket.as_os_str().len() <= LONGEST_SOCKET).then_some(socket)
}

/// What `anima-link unit` may see of the app's environment: the home folder, and on Linux the
/// folders that name the configuration and the session of the user.
pub fn environment(read: impl Fn(&str) -> Option<OsString>) -> Vec<(&'static str, OsString)> {
    ["HOME", "TMPDIR", "XDG_CONFIG_HOME", "XDG_RUNTIME_DIR"]
        .into_iter()
        .filter_map(|name| Some((name, read(name).filter(|value| !value.is_empty())?)))
        .collect()
}
