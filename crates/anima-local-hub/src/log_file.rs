//! One member's log file: lines appended, mode 0600, and one older file kept.

use std::fs::{self, File};
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};

use anima_database::owner_only;

/// Above this size the log becomes `<name>.1`, in place of the one before it.
pub const LOG_LIMIT: u64 = 8 * 1024 * 1024;

#[derive(Debug)]
pub struct LogFile {
    path: PathBuf,
    limit: u64,
    file: File,
    written: u64,
}

impl LogFile {
    /// The log at `path`, open for appending.
    ///
    /// # Errors
    /// When the file cannot be opened, or `path` is a symbolic link.
    pub fn open(path: &Path, limit: u64) -> io::Result<Self> {
        let file = owner_only::appended_file(path)?;
        let written = file.metadata()?.len();
        Ok(Self {
            path: path.to_path_buf(),
            limit,
            file,
            written,
        })
    }

    /// Appends `line` and a line break. Past the limit the file is put aside first.
    ///
    /// # Errors
    /// When the file cannot be written or put aside.
    pub fn line(&mut self, line: &str) -> io::Result<()> {
        if self.written > self.limit {
            let mut older = self.path.clone().into_os_string();
            older.push(".1");
            fs::rename(&self.path, older)?;
            self.file = owner_only::appended_file(&self.path)?;
            self.written = 0;
        }
        self.file.write_all(line.as_bytes())?;
        self.file.write_all(b"\n")?;
        self.written += line.len() as u64 + 1;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    use super::LogFile;

    #[test]
    fn lines_are_appended_to_what_an_earlier_run_wrote() {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("hub.log");
        LogFile::open(&path, 100).unwrap().line("first").unwrap();

        LogFile::open(&path, 100).unwrap().line("second").unwrap();

        assert_eq!(fs::read_to_string(&path).unwrap(), "first\nsecond\n");
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }

    #[test]
    fn a_full_log_is_put_aside_and_only_one_older_file_is_kept() {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("hub.log");
        let mut log = LogFile::open(&path, 10).unwrap();

        for line in ["0123456789a", "0123456789b", "c"] {
            log.line(line).unwrap();
        }

        assert_eq!(fs::read_to_string(&path).unwrap(), "c\n");
        assert_eq!(
            fs::read_to_string(folder.path().join("hub.log.1")).unwrap(),
            "0123456789b\n"
        );
        assert_eq!(fs::read_dir(folder.path()).unwrap().count(), 2);
    }
}
