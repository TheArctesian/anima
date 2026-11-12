//! Where a build leaves its files, and the checks that a file there is the build's own (BLD-043,
//! BLD-047). The build wrote the folder, so a path is used only when it is a regular file inside
//! the clone: a link to `~/.ssh` or elsewhere is refused before it is signed.

use std::{
    fs::{self, File},
    io::{Read, Write},
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
};

use super::{
    error::BuilderError,
    job::{Target, Version},
};

/// `productName` in `tauri.conf.json`; the file names in Section 3.10.4 follow it.
const PRODUCT: &str = "Anima";

/// The folder `tauri bundle` writes for `target`.
fn bundle_folder(clone: &Path, target: Target) -> PathBuf {
    match target {
        Target::LinuxX86_64 => clone.join("target/release/bundle"),
        Target::DarwinUniversal => clone.join("target/universal-apple-darwin/release/bundle"),
    }
}

/// The file the updater downloads: the `AppImage` on Linux, the archived app on macOS.
pub fn updater_file(clone: &Path, target: Target, version: &Version) -> PathBuf {
    let folder = bundle_folder(clone, target);
    match target {
        Target::LinuxX86_64 => folder.join(format!(
            "appimage/{PRODUCT}_{}_amd64.AppImage",
            version.as_str()
        )),
        Target::DarwinUniversal => folder.join(format!("macos/{PRODUCT}.app.tar.gz")),
    }
}

/// The DMG people download, made on macOS only (BLD-063).
pub fn dmg_file(clone: &Path, target: Target, version: &Version) -> Option<PathBuf> {
    match target {
        Target::LinuxX86_64 => None,
        Target::DarwinUniversal => Some(
            bundle_folder(clone, target)
                .join(format!("dmg/{PRODUCT}_{}_universal.dmg", version.as_str())),
        ),
    }
}

/// The macOS app folder.
pub fn mac_app(clone: &Path) -> PathBuf {
    bundle_folder(clone, Target::DarwinUniversal).join(format!("macos/{PRODUCT}.app"))
}

/// `path` when it is a regular file, not a link, inside `clone` once every link in its folders
/// is followed.
pub fn own_file(clone: &Path, path: &Path) -> Result<PathBuf, BuilderError> {
    let kind = fs::symlink_metadata(path).map_err(|_| BuilderError::Bundle)?;
    let real = fs::canonicalize(path).map_err(|_| BuilderError::Bundle)?;
    let root = fs::canonicalize(clone).map_err(|_| BuilderError::Bundle)?;
    (kind.is_file() && real.starts_with(&root))
        .then_some(real)
        .ok_or(BuilderError::Bundle)
}

/// Opens the build's own file at `path` for reading, once (D-13): never through a link, and only
/// when it has one name, so a hard link to a file outside the clone is refused. Hash and sign
/// from this one descriptor, so the bytes signed are the bytes checked.
pub fn open_own(clone: &Path, path: &Path) -> Result<File, BuilderError> {
    let real = own_file(clone, path)?;
    let file = fs::OpenOptions::new()
        .read(true)
        .custom_flags(nix::libc::O_NOFOLLOW)
        .open(real)
        .map_err(|_| BuilderError::Bundle)?;
    let kind = file.metadata().map_err(|_| BuilderError::Bundle)?;
    (kind.is_file() && kind.nlink() == 1)
        .then_some(file)
        .ok_or(BuilderError::Bundle)
}

/// The one executable in the app's `Contents/MacOS`.
pub fn mac_binary(clone: &Path, app: &Path) -> Result<PathBuf, BuilderError> {
    let entries = fs::read_dir(app.join("Contents/MacOS")).map_err(|_| BuilderError::Bundle)?;
    let files: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .collect();
    match files.as_slice() {
        [only] => own_file(clone, only),
        _ => Err(BuilderError::Bundle),
    }
}

/// Whether `lipo -archs` printed both `x86_64` and `arm64` (UPD-011).
pub fn universal(archs: &[String]) -> bool {
    let words: Vec<&str> = archs
        .iter()
        .flat_map(|line| line.split_whitespace())
        .collect();
    words.contains(&"x86_64") && words.contains(&"arm64")
}

/// Whether `notarytool submit --wait` printed Apple's final answer as Accepted (BLD-085). Its
/// progress lines say `Current status: ...`; only the final `status:` line counts.
pub fn notarized(lines: &[String]) -> bool {
    lines.iter().any(|line| line.trim() == "status: Accepted")
}

/// A `.sig` file is a few hundred bytes.
const SIGNATURE_MAX_BYTES: u64 = 4096;

fn signature_path(updater: &Path) -> PathBuf {
    let mut path = updater.as_os_str().to_owned();
    path.push(".sig");
    PathBuf::from(path)
}

/// Writes `<updater>.sig` as a new file. Whatever the build left at that path, a link included,
/// is removed first and never followed.
pub fn write_signature(updater: &Path, text: &str) -> Result<(), BuilderError> {
    let path = signature_path(updater);
    match fs::remove_file(&path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => return Err(BuilderError::Bundle),
    }
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|_| BuilderError::Bundle)?;
    file.write_all(text.as_bytes())
        .map_err(|_| BuilderError::Bundle)
}

/// Reads `<updater>.sig` when it is a regular file of at most `SIGNATURE_MAX_BYTES`.
pub fn read_signature(updater: &Path) -> Result<String, BuilderError> {
    let path = signature_path(updater);
    let kind = fs::symlink_metadata(&path).map_err(|_| BuilderError::Bundle)?;
    if !kind.is_file() || kind.len() > SIGNATURE_MAX_BYTES {
        return Err(BuilderError::Bundle);
    }
    let mut text = String::new();
    fs::File::open(&path)
        .and_then(|file| file.take(SIGNATURE_MAX_BYTES).read_to_string(&mut text))
        .map_err(|_| BuilderError::Bundle)?;
    Ok(text)
}
