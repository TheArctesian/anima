//! Where the application keeps this user's files: the hub root of `anima-local-hub` and the
//! device link's folder are below it. The same rule as `anima-database`'s `Layout::for_user`,
//! which the local hub uses; a test holds the two paths of `docs/procedures/setup/local-hub.md`.

use std::path::{Path, PathBuf};

/// The application's folder: `~/Library/Application Support/ing.anima.desktop` on macOS,
/// `$XDG_DATA_HOME/anima` on Linux (`~/.local/share/anima` when the variable is unset or not
/// an absolute path).
pub fn application(macos: bool, home: &Path, xdg_data_home: Option<&Path>) -> PathBuf {
    if macos {
        return home.join("Library/Application Support/ing.anima.desktop");
    }
    xdg_data_home
        .filter(|folder| folder.is_absolute())
        .map_or_else(|| home.join(".local/share"), Path::to_path_buf)
        .join("anima")
}

/// The home folder of the user of this app; none without an absolute `HOME`.
pub fn home() -> Option<PathBuf> {
    Some(PathBuf::from(std::env::var_os(if cfg!(windows) {
        "USERPROFILE"
    } else {
        "HOME"
    })?))
    .filter(|home| home.is_absolute())
}

/// The application's folder for the user of this app; none without an absolute `HOME`.
pub fn of_this_user() -> Option<PathBuf> {
    if cfg!(windows) {
        return std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
            .map(|path| path.join("Anima"));
    }
    let home = home()?;
    let xdg_data_home = std::env::var_os("XDG_DATA_HOME").map(PathBuf::from);
    Some(application(
        cfg!(target_os = "macos"),
        &home,
        xdg_data_home.as_deref(),
    ))
}

/// The hub root of the hub on this computer.
pub fn hub_root(application: &Path) -> PathBuf {
    application.join("hub")
}

#[cfg(all(test, unix))]
mod tests {
    use std::path::Path;

    use super::{application, hub_root};

    #[test]
    fn the_hub_root_is_where_the_local_hub_keeps_it() {
        let home = Path::new("/home/ada");

        assert_eq!(
            hub_root(&application(true, Path::new("/Users/ada"), None)),
            Path::new("/Users/ada/Library/Application Support/ing.anima.desktop/hub")
        );
        assert_eq!(
            hub_root(&application(false, home, None)),
            Path::new("/home/ada/.local/share/anima/hub")
        );
        assert_eq!(
            hub_root(&application(false, home, Some(Path::new("/data/ada")))),
            Path::new("/data/ada/anima/hub")
        );
        // A relative XDG_DATA_HOME is not used, as the XDG rule says.
        assert_eq!(
            hub_root(&application(false, home, Some(Path::new("data")))),
            Path::new("/home/ada/.local/share/anima/hub")
        );
    }
}
