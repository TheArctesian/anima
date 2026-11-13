//! Where a transfer reads and writes (K4 Section 9). Pure: `<device>:<path>` on the device,
//! and the local file a download goes to.
//!
//! A downloaded file is named by the device only when the operator names no file: then the
//! name is the remote path's last part, refused when it is empty, `.`, `..`, or holds a
//! backslash, NUL or a control character, so device text can never place a file outside the
//! folder the operator chose.

use std::path::{Path, PathBuf};

use crate::failure::{Failure, Hint};

/// A path on a device, as `<device>:<path>` gave it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Remote {
    pub device: String,
    pub path: String,
}

impl Remote {
    /// `spec` split at its first `:` into the device (ID or name) and the path.
    ///
    /// # Errors
    /// [`Failure::Usage`] when either side is empty.
    pub fn parse(spec: &str) -> Result<Self, Failure> {
        match spec.split_once(':') {
            Some((device, path)) if !device.is_empty() && !path.is_empty() => Ok(Self {
                device: device.to_owned(),
                path: path.to_owned(),
            }),
            _ => Err(Failure::Usage {
                message: format!("{spec} is not <device>:<path>."),
                hint: Some(Hint::run("anima files ls web-1:/var/log")),
            }),
        }
    }

    /// The last part of the path, when it is safe as a local file name.
    #[must_use]
    pub fn file_name(&self) -> Option<&str> {
        let name = self.path.trim_end_matches('/').rsplit('/').next()?;
        let unsafe_name = name.is_empty()
            || name == "."
            || name == ".."
            || name
                .chars()
                .any(|character| character == '\\' || character.is_control());
        (!unsafe_name).then_some(name)
    }
}

/// The local file a download of `remote` writes: `local` itself, or inside it when it is a
/// folder (`local_is_dir`), or in the current folder when it is not given.
///
/// # Errors
/// [`Failure::Usage`] when the device's name for the file is needed and not safe.
pub fn download_target(
    remote: &Remote,
    local: Option<&Path>,
    local_is_dir: bool,
) -> Result<PathBuf, Failure> {
    let named = || {
        remote
            .file_name()
            .map(PathBuf::from)
            .ok_or_else(|| Failure::Usage {
                message: format!(
                    "{} does not end in a file name that is safe here; name the local file.",
                    remote.path
                ),
                hint: Some(Hint::run(format!(
                    "anima files get {}:{} ./file",
                    remote.device, remote.path
                ))),
            })
    };
    match local {
        None => named(),
        Some(folder) if local_is_dir => Ok(folder.join(named()?)),
        Some(file) => Ok(file.to_path_buf()),
    }
}

/// The device path an upload of `local` writes: `remote`'s path, or the local file's name
/// inside it when it ends in `/`.
///
/// # Errors
/// [`Failure::Usage`] when the local file's name is not UTF-8.
pub fn upload_target(local: &Path, remote: &Remote) -> Result<String, Failure> {
    if !remote.path.ends_with('/') {
        return Ok(remote.path.clone());
    }
    let name = local
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| Failure::Usage {
            message: "The local file's name is not UTF-8; give the full device path.".to_owned(),
            hint: None,
        })?;
    Ok(format!("{}{name}", remote.path))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn remote(path: &str) -> Remote {
        Remote::parse(&format!("web-1:{path}")).expect("a remote")
    }

    #[test]
    fn k4_9_a_remote_is_device_colon_path() {
        assert_eq!(
            Remote::parse("web-1:/etc/hosts").expect("remote"),
            Remote {
                device: "web-1".into(),
                path: "/etc/hosts".into()
            }
        );
        assert_eq!(remote("C:/x").path, "C:/x");
        for spec in ["web-1", ":/etc", "web-1:"] {
            assert!(Remote::parse(spec).is_err(), "{spec}");
        }
    }

    #[test]
    fn k4_9_a_device_name_never_leaves_the_chosen_folder() {
        let folder = Path::new("/tmp/in");
        assert_eq!(
            download_target(&remote("/var/log/syslog"), Some(folder), true).expect("target"),
            PathBuf::from("/tmp/in/syslog")
        );
        assert_eq!(
            download_target(&remote("/var/log/"), None, false).expect("target"),
            PathBuf::from("log")
        );
        for path in ["/x/..", "..", "/", ".", "/x/a\\..\\b", "/x/\u{1b}]0;t"] {
            assert!(
                download_target(&remote(path), Some(folder), true).is_err(),
                "{path:?}"
            );
        }
        assert_eq!(
            download_target(&remote(".."), Some(Path::new("out.txt")), false).expect("named"),
            PathBuf::from("out.txt")
        );
    }

    #[test]
    fn k4_9_an_upload_into_a_folder_keeps_the_local_name() {
        assert_eq!(
            upload_target(Path::new("/home/me/a.txt"), &remote("/tmp/")).expect("target"),
            "/tmp/a.txt"
        );
        assert_eq!(
            upload_target(Path::new("a.txt"), &remote("/tmp/b.txt")).expect("target"),
            "/tmp/b.txt"
        );
    }
}
