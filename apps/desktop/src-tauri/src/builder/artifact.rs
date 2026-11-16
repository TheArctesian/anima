//! Where a build leaves its files, and the checks that a file there is the build's own (BLD-043,
//! BLD-047). The build wrote the folder, so a path is used only when it is a regular file inside
//! the clone: a link to `~/.ssh` or elsewhere is refused before it is uploaded.

use std::{
    ffi::OsString,
    fs::{self, File},
    io::{self, Read},
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
};

use super::{
    carried,
    error::BuilderError,
    job::{Target, Version},
};

/// Stable artefact prefix the hub, the site and Section 3.10.4 use (`Anima.dmg`,
/// `Anima_{version}_amd64.AppImage`, `Anima.app.tar.gz`).
const PRODUCT: &str = "Anima";
/// `productName` in `tauri.conf.json`. Tauri names the `.app` folder and the first
/// component of the DMG and `AppImage` after it. Official artefact file names stay
/// `Anima` (UPD-010), so [`stabilize_names`] renames the DMG and `AppImage` after bundling.
const DISPLAY: &str = "Anima.ing";
/// The app's own executable in `Contents/MacOS`: the Cargo binary name.
const MAC_EXECUTABLE: &str = "anima-desktop";

/// Where cargo writes what it builds in `clone`. Every step is told this folder
/// (`CARGO_TARGET_DIR`), so a `target-dir` in the builder's own cargo configuration cannot move
/// the bundle to a place the later steps do not look in.
pub fn target_folder(clone: &Path) -> PathBuf {
    clone.join("target")
}

/// The folder `tauri bundle` writes for `target`.
fn bundle_folder(clone: &Path, target: Target) -> PathBuf {
    match target {
        Target::LinuxX86_64 => target_folder(clone).join("release/bundle"),
        Target::DarwinUniversal => {
            target_folder(clone).join("universal-apple-darwin/release/bundle")
        }
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

/// The macOS app folder Tauri writes for `productName`.
pub fn mac_app(clone: &Path) -> PathBuf {
    bundle_folder(clone, Target::DarwinUniversal).join(format!("macos/{DISPLAY}.app"))
}

/// Deletes the folder `tauri bundle` writes for `target`, so no file of an earlier attempt can be
/// taken for this one's (D1 of the projects and builds proposal): the checkout keeps cargo's
/// `target/`. The build wrote the folders above it, so a link among them is refused, never
/// followed, and a link at the folder itself is removed as a link.
pub fn clear_bundle(clone: &Path, target: Target) -> Result<(), BuilderError> {
    let folder = bundle_folder(clone, target);
    let kind = match fs::symlink_metadata(&folder) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        other => other.map_err(|_| BuilderError::Bundle)?,
    };
    let parent = folder.parent().ok_or(BuilderError::Bundle)?;
    let root = fs::canonicalize(clone).map_err(|_| BuilderError::Bundle)?;
    let real = fs::canonicalize(parent).map_err(|_| BuilderError::Bundle)?;
    if real
        != root.join(
            parent
                .strip_prefix(clone)
                .map_err(|_| BuilderError::Bundle)?,
        )
    {
        return Err(BuilderError::Bundle);
    }
    if kind.is_dir() {
        fs::remove_dir_all(&folder)
    } else {
        fs::remove_file(&folder)
    }
    .map_err(|_| BuilderError::Bundle)
}

/// Renames Tauri's `productName`-based DMG and `AppImage` to the stable `Anima_*` names
/// the hub and the site expect (UPD-010). The `.app` folder keeps the display name;
/// the updater archive is still written to `Anima.app.tar.gz`. When Tauri wrote no
/// `productName` file, the stable name must be there already (a commit whose `productName`
/// was still `Anima`).
pub fn stabilize_names(
    clone: &Path,
    target: Target,
    version: &Version,
) -> Result<(), BuilderError> {
    let produced = match target {
        Target::LinuxX86_64 => bundle_folder(clone, target).join(format!(
            "appimage/{DISPLAY}_{}_amd64.AppImage",
            version.as_str()
        )),
        Target::DarwinUniversal => bundle_folder(clone, target)
            .join(format!("dmg/{DISPLAY}_{}_universal.dmg", version.as_str())),
    };
    let wanted = match target {
        Target::LinuxX86_64 => updater_file(clone, target, version),
        Target::DarwinUniversal => dmg_file(clone, target, version).ok_or(BuilderError::Bundle)?,
    };
    if wanted == produced || (!produced.exists() && wanted.is_file()) {
        return Ok(());
    }
    fs::rename(&produced, &wanted).map_err(|_| BuilderError::Bundle)
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
/// when it has one name, so a hard link to a file outside the clone is refused. Hash and read
/// from this one descriptor, so the bytes recorded are the bytes checked.
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

/// The app's own executable, once its `Contents/MacOS` holds exactly it and the six carried
/// programs (INS-002, S12 2.3). Any other set of files fails, so nothing unchecked is notarized
/// and shipped.
pub fn mac_executable(clone: &Path, app: &Path) -> Result<PathBuf, BuilderError> {
    let folder = app.join("Contents/MacOS");
    let entries = fs::read_dir(&folder).map_err(|_| BuilderError::Bundle)?;
    let mut names: Vec<OsString> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.file_name())
        .collect();
    names.sort();
    let mut expected: Vec<&str> = carried::PROGRAMS.to_vec();
    expected.push(MAC_EXECUTABLE);
    expected.sort_unstable();
    if names != expected {
        return Err(BuilderError::Bundle);
    }
    own_file(clone, &folder.join(MAC_EXECUTABLE))
}

/// Opens cargo's output at `path` for reading: a regular file inside the clone, never through a
/// link. Cargo gives its output a second name in `deps/`, so the one-name rule of [`open_own`]
/// cannot hold; the staged file must then pass the `lipo` or ELF check, which no key or other
/// text file passes. The Mach-O check of the app reads each file's first bytes through it too.
pub fn open_built(clone: &Path, path: &Path) -> Result<File, BuilderError> {
    let file = fs::OpenOptions::new()
        .read(true)
        .custom_flags(nix::libc::O_NOFOLLOW)
        .open(own_file(clone, path)?)
        .map_err(|_| BuilderError::Bundle)?;
    let kind = file.metadata().map_err(|_| BuilderError::Bundle)?;
    kind.is_file().then_some(file).ok_or(BuilderError::Bundle)
}

/// Copies cargo's output `from` to `to`, a new file in a folder inside the clone, mode 0755.
/// What is at `to` is removed first and never followed, so a link a build planted there cannot
/// make the copy write outside the clone.
pub fn stage(clone: &Path, from: &Path, to: &Path) -> Result<(), BuilderError> {
    let mut input = open_built(clone, from)?;
    let folder = to.parent().ok_or(BuilderError::Bundle)?;
    fs::create_dir_all(folder).map_err(|_| BuilderError::Bundle)?;
    let root = fs::canonicalize(clone).map_err(|_| BuilderError::Bundle)?;
    let real = fs::canonicalize(folder).map_err(|_| BuilderError::Bundle)?;
    if !real.starts_with(&root) {
        return Err(BuilderError::Bundle);
    }
    match fs::remove_file(to) {
        Err(error) if error.kind() != io::ErrorKind::NotFound => return Err(BuilderError::Bundle),
        _ => {}
    }
    let mut output = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o755)
        .open(to)
        .map_err(|_| BuilderError::Bundle)?;
    io::copy(&mut input, &mut output).map_err(|_| BuilderError::Bundle)?;
    output.sync_all().map_err(|_| BuilderError::Bundle)
}

/// The first bytes of the build's own file at `path`, at most `count`.
pub fn header(clone: &Path, path: &Path, count: u64) -> Result<Vec<u8>, BuilderError> {
    let mut bytes = Vec::new();
    open_own(clone, path)?
        .take(count)
        .read_to_end(&mut bytes)
        .map_err(|_| BuilderError::Bundle)?;
    Ok(bytes)
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
