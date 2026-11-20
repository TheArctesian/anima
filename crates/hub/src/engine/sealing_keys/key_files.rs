//! Private key files: enumerate, read, and atomically create each named key.
use super::{KEY_LEN, KEY_SUFFIX, SealingKey, SealingKeyError, hex, is_name, key_id};
use base64::{Engine as _, engine::general_purpose::STANDARD};
#[cfg(unix)]
use std::{fs::OpenOptions, os::unix::fs::OpenOptionsExt as _};
use std::{
    io::{self, Write as _},
    path::{Path, PathBuf},
};
use zeroize::Zeroizing;
pub(super) fn key_names(folder: &Path) -> Result<Vec<String>, SealingKeyError> {
    let entries =
        std::fs::read_dir(folder).map_err(|error| SealingKeyError::Folder(error.kind()))?;
    let mut names = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| SealingKeyError::Folder(error.kind()))?;
        let file = entry.file_name();
        let name = file.to_str().and_then(|file| file.strip_suffix(KEY_SUFFIX));
        if let Some(name) = name.filter(|name| is_name(name)) {
            names.push(name.to_owned());
        }
    }
    names.sort();
    Ok(names)
}

fn key_path(folder: &Path, name: &str) -> PathBuf {
    folder.join(format!("{name}{KEY_SUFFIX}"))
}

pub(super) fn read_key(folder: &Path, name: &str) -> Result<SealingKey, SealingKeyError> {
    let unreadable = |error: io::Error| SealingKeyError::KeyFile {
        name: name.to_owned(),
        kind: error.kind(),
    };
    #[cfg(unix)]
    let text = Zeroizing::new(std::fs::read(key_path(folder, name)).map_err(unreadable)?);
    #[cfg(windows)]
    let text = {
        use std::io::Read;
        let mut text = Zeroizing::new(Vec::new());
        anima_windows::files::read(&key_path(folder, name), true)
            .map_err(unreadable)?
            .take(128)
            .read_to_end(&mut text)
            .map_err(unreadable)?;
        text
    };
    let mut key = Zeroizing::new([0_u8; KEY_LEN]);
    // The decode error is dropped unread: it can quote a byte of the key.
    let decoded = STANDARD.decode_slice(text.trim_ascii(), key.as_mut_slice());
    if decoded.ok() != Some(KEY_LEN) {
        return Err(SealingKeyError::KeyFormat {
            name: name.to_owned(),
        });
    }
    let id = key_id(&key);
    Ok(SealingKey {
        name: name.to_owned(),
        key,
        id,
    })
}

/// Writes a new key file that only the hub user can read: first under a temporary name, then
/// linked to its name, so that no reader sees a file that is half written. A file that another
/// request made first is kept: the caller reads it.
pub(super) fn make_key(folder: &Path, name: &str) -> Result<(), SealingKeyError> {
    let failed = |error: io::Error| SealingKeyError::KeyFile {
        name: name.to_owned(),
        kind: error.kind(),
    };
    let mut random = Zeroizing::new([0_u8; KEY_LEN + 8]);
    getrandom::fill(random.as_mut_slice()).map_err(|_| SealingKeyError::KeyFile {
        name: name.to_owned(),
        kind: io::ErrorKind::Other,
    })?;
    let (key, marker) = random.split_at(KEY_LEN);
    let text = Zeroizing::new(format!("{}\n", STANDARD.encode(key)));
    let temporary = folder.join(format!(".{name}.{}.tmp", hex(marker)));
    let written = write_private(&temporary, text.as_bytes())
        .and_then(|()| std::fs::hard_link(&temporary, key_path(folder, name)));
    let removed = std::fs::remove_file(&temporary);
    match written {
        Ok(()) => removed.map_err(failed),
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => Ok(()),
        Err(error) => Err(failed(error)),
    }
}

/// A new file with mode 0600 that holds `bytes`, flushed to the disk.
fn write_private(path: &Path, bytes: &[u8]) -> io::Result<()> {
    #[cfg(unix)]
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)?;
    #[cfg(windows)]
    let mut file = anima_windows::files::create_new(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}
