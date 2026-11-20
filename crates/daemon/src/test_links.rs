//! Platform spelling of test-only filesystem links. Windows link tests need Developer Mode
//! or the symbolic-link privilege, and exercise the same refusal as Unix tests.
use std::{io, path::Path};
pub(crate) fn symlink(target: impl AsRef<Path>, link: impl AsRef<Path>) -> io::Result<()> {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, link)
    }
    #[cfg(windows)]
    {
        if target.as_ref().is_dir() {
            std::os::windows::fs::symlink_dir(target, link)
        } else {
            std::os::windows::fs::symlink_file(target, link)
        }
    }
}
