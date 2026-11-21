//! Unix file placement for the persistent device link.
use super::{DAEMON_SOCKET, LONGEST_SOCKET};
use std::{
    ffi::OsString,
    fs::{self, DirBuilder},
    os::unix::fs::{DirBuilderExt, FileTypeExt, MetadataExt},
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
/// with mode 0700 when it is absent. None when the link would refuse the socket, so that the
/// link still starts: the folder is a link, belongs to a different user or gives its group or
/// others any access; a folder above it can be replaced by another user (`foldersAbove` of the
/// link); a file at the path is not a socket of this user with no access for others; or the
/// path is too long for a socket.
pub fn private_socket(folder: &Path) -> Option<PathBuf> {
    let _ = DirBuilder::new().recursive(true).mode(0o700).create(folder);
    let me = nix::unistd::getuid().as_raw();
    let found = fs::symlink_metadata(folder).ok()?;
    let private = found.is_dir() && found.uid() == me && no_access_for_others(&found);
    let socket = folder.join(DAEMON_SOCKET);
    let usable = match fs::symlink_metadata(&socket) {
        Ok(file) => file.file_type().is_socket() && file.uid() == me && no_access_for_others(&file),
        Err(error) => error.kind() == std::io::ErrorKind::NotFound,
    };
    (private && usable && kept_above(folder, me) && socket.as_os_str().len() <= LONGEST_SOCKET)
        .then_some(socket)
}

/// No bit of the group or of others: the last two octal digits are zero.
fn no_access_for_others(found: &fs::Metadata) -> bool {
    found.mode().is_multiple_of(0o100)
}

/// Whether no other user can replace a folder above `folder` on its real path: each is root's or
/// this user's, and others cannot write to it, or it has the sticky bit. The link checks the
/// same before it makes the socket.
fn kept_above(folder: &Path, me: u32) -> bool {
    let Ok(real) = fs::canonicalize(folder) else {
        return false;
    };
    real.ancestors().skip(1).all(|above| {
        fs::symlink_metadata(above).is_ok_and(|found| {
            let open = found.mode() & 0o022 != 0 && found.mode() & 0o1000 == 0;
            !open && (found.uid() == 0 || found.uid() == me)
        })
    })
}

/// The socket for the daemon in the folder of the daemon of the user of `home`
/// ([`daemon_folder`], [`private_socket`]): the unit and the link that the app runs itself both
/// make it there, so a daemon finds the link whichever runs.
pub fn socket_of_user(home: &Path) -> Option<PathBuf> {
    let config_home = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from);
    private_socket(&daemon_folder(
        home,
        cfg!(target_os = "macos"),
        config_home.as_deref(),
    ))
}

/// What `anima-link unit` may see of the app's environment: the home folder, and on Linux the
/// folders that name the configuration and the session of the user.
pub fn environment(read: impl Fn(&str) -> Option<OsString>) -> Vec<(&'static str, OsString)> {
    ["HOME", "TMPDIR", "XDG_CONFIG_HOME", "XDG_RUNTIME_DIR"]
        .into_iter()
        .filter_map(|name| Some((name, read(name).filter(|value| !value.is_empty())?)))
        .collect()
}

pub(super) fn socket_present(socket: &Path) -> bool {
    fs::symlink_metadata(socket).is_ok_and(|found| found.file_type().is_socket())
}
