//! The installed app that carries the `anima` program, and what a link or launcher must point at
//! (INS-004, INS-005). Pure: the shell reads the executable path, the `AppImage` variables and
//! whether `anima` exists, and passes them in.

use std::{
    ffi::OsStr,
    path::{Component, Path, PathBuf},
};

use super::{error::CommandLineError, launcher};

/// The program's file name: in the bundle beside the app's executable, and on the `PATH`.
pub const PROGRAM: &str = "anima";

/// What the operator's `anima` must run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Carrier {
    /// macOS: `anima` in `Contents/MacOS` of the app bundle; the link points at it.
    Bundle { program: PathBuf },
    /// Linux: the `AppImage` file; the launcher runs it. Its mount moves on every start.
    AppImage { file: PathBuf },
}

/// The `anima` beside `executable`, the app's own binary: Tauri's place for an external binary.
pub fn beside(executable: &Path) -> Option<PathBuf> {
    executable.parent().map(|folder| folder.join(PROGRAM))
}

/// Whether `path` is `<name>.app/Contents/MacOS/anima`, the program of some Anima app bundle.
pub fn is_bundle_program(path: &Path) -> bool {
    let names: Vec<&OsStr> = path
        .components()
        .filter_map(|part| match part {
            Component::Normal(name) => Some(name),
            _ => None,
        })
        .collect();
    path.is_absolute()
        && matches!(
            names.as_slice(),
            [.., app, contents, macos, program]
                if Path::new(app).extension() == Some(OsStr::new("app"))
                    && *contents == "Contents"
                    && *macos == "MacOS"
                    && *program == PROGRAM
        )
}

/// The macOS carrier for the app whose real executable path is `executable`. A disk image
/// (`/Volumes/...`) and App Translocation are refused: their path is gone after the next start,
/// so a link into them would break.
///
/// # Errors
/// [`CommandLineError::DiskImage`], [`CommandLineError::Translocated`], or
/// [`CommandLineError::NotBundled`] when `anima` is not in the bundle.
pub fn bundle(executable: &Path, program_exists: bool) -> Result<Carrier, CommandLineError> {
    if executable
        .components()
        .any(|part| part.as_os_str() == "AppTranslocation")
    {
        return Err(CommandLineError::Translocated);
    }
    if executable.starts_with("/Volumes") {
        return Err(CommandLineError::DiskImage);
    }
    let program = beside(executable)
        .filter(|program| is_bundle_program(program) && program_exists)
        .ok_or(CommandLineError::NotBundled)?;
    Ok(Carrier::Bundle { program })
}

/// The Linux carrier: `appimage` (`$APPIMAGE`, the file's real path, set by the `AppImage`
/// runtime) when this executable runs inside the mount `appdir` (`$APPDIR`).
///
/// # Errors
/// [`CommandLineError::NotAppImage`] outside an `AppImage`, [`CommandLineError::Unquotable`] for a
/// path the launcher cannot hold, [`CommandLineError::NotBundled`] when `anima` is not inside.
pub fn app_image(
    appimage: Option<&OsStr>,
    appdir: Option<&OsStr>,
    executable: &Path,
    program_exists: bool,
) -> Result<Carrier, CommandLineError> {
    let (Some(file), Some(mount)) = (appimage.map(Path::new), appdir.map(Path::new)) else {
        return Err(CommandLineError::NotAppImage);
    };
    if !file.is_absolute() || !mount.is_absolute() || !executable.starts_with(mount) {
        return Err(CommandLineError::NotAppImage);
    }
    launcher::script(file).ok_or(CommandLineError::Unquotable)?;
    if !program_exists {
        return Err(CommandLineError::NotBundled);
    }
    Ok(Carrier::AppImage {
        file: file.to_owned(),
    })
}
