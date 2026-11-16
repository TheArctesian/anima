//! Whether somebody else could swap what the unit runs at each login: the unit file, or the
//! program it names.

use std::fs;
use std::io;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

const STICKY: u32 = 0o1000;
const GROUP_OR_OTHERS_WRITE: u32 = 0o022;

/// The first folder, from `folder` up to `/`, that a user other than `uid` and root can
/// rename an entry of. `None` when there is none.
///
/// A folder with the sticky bit passes: there a user can rename only an entry of their own.
/// Access control lists are not read (the agent reads them on Linux only, MET-019).
///
/// # Errors
/// When `folder` or a folder above it cannot be looked at.
pub fn open_to_others(folder: &Path, uid: u32) -> io::Result<Option<PathBuf>> {
    // The real path: a link in the path is a name that its own folder's rules protect.
    let folder = fs::canonicalize(folder)?;
    for folder in folder.ancestors() {
        let found = fs::metadata(folder)?;
        let owner_is_trusted = found.uid() == uid || found.uid() == 0;
        let others_can_rename =
            found.mode() & GROUP_OR_OTHERS_WRITE != 0 && found.mode() & STICKY == 0;
        if !found.is_dir() || !owner_is_trusted || others_can_rename {
            return Ok(Some(folder.to_path_buf()));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    use super::open_to_others;

    fn me() -> u32 {
        nix::unistd::Uid::effective().as_raw()
    }

    #[test]
    fn a_folder_that_only_this_user_and_root_can_change_up_to_the_top_passes() {
        let folder = tempfile::tempdir().unwrap();
        let deep = folder.path().join("a/b");
        fs::create_dir_all(&deep).unwrap();

        assert_eq!(open_to_others(&deep, me()).unwrap(), None);
    }

    #[test]
    fn a_folder_above_that_others_can_write_is_found() {
        let folder = tempfile::tempdir().unwrap();
        let (open, deep) = (folder.path().join("a"), folder.path().join("a/b"));
        fs::create_dir_all(&deep).unwrap();
        for mode in [0o777, 0o775, 0o757] {
            fs::set_permissions(&open, fs::Permissions::from_mode(mode)).unwrap();

            let found = open_to_others(&deep, me()).unwrap();

            assert_eq!(found, Some(fs::canonicalize(&open).unwrap()), "{mode:o}");
        }
    }

    #[test]
    fn a_shared_folder_with_the_sticky_bit_passes() {
        let folder = tempfile::tempdir().unwrap();
        let (shared, deep) = (folder.path().join("a"), folder.path().join("a/b"));
        fs::create_dir_all(&deep).unwrap();
        fs::set_permissions(&shared, fs::Permissions::from_mode(0o1777)).unwrap();

        assert_eq!(open_to_others(&deep, me()).unwrap(), None);
    }

    #[test]
    fn a_folder_of_another_user_is_found() {
        let folder = tempfile::tempdir().unwrap();

        let found = open_to_others(folder.path(), me() + 1).unwrap();

        assert_eq!(found, Some(fs::canonicalize(folder.path()).unwrap()));
    }
}
