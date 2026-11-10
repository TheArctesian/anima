//! Checks the system unit's token as root (MET-019): the file is opened without following a
//! link, without blocking on a pipe and without taking a terminal, and is then judged by what
//! the open file is, so the account cannot swap it between the check and the read.

use std::fs::{self, File, OpenOptions};
use std::io;
use std::os::unix::fs::{MetadataExt as _, OpenOptionsExt as _};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use rustix::fs::OFlags;
use rustix::io::Errno;

use super::ready::{self, NotReady};
use crate::config::private_file::{self, LIMIT};

#[derive(Debug, thiserror::Error)]
pub enum SystemTokenError {
    #[error("{0}")]
    NotReady(#[from] NotReady),
    #[error("reading {}: {source}", path.display())]
    Read { path: PathBuf, source: io::Error },
}

/// Checks the token file `token` for the system unit: the account in `passwd` must exist and
/// own it as a regular 0600 file, and it must hold a record that expires after `now`. `program`
/// is printed in the enrol command. The token's text never reaches an error (LOG-007).
///
/// # Errors
/// [`SystemTokenError::NotReady`] for what is not in order, `Read` when it cannot be read.
pub(super) fn check(
    token: &Path,
    passwd: &str,
    now: DateTime<Utc>,
    program: &str,
) -> Result<(), SystemTokenError> {
    let read = |source| SystemTokenError::Read {
        path: token.to_owned(),
        source,
    };
    let (file, found) = match open(token) {
        Ok(file) => {
            let metadata = file.metadata().map_err(read)?;
            (Some(file), Some((metadata.mode(), metadata.uid())))
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => (None, None),
        // A link: described as the link itself, which is no regular file.
        Err(error) if error.raw_os_error() == Some(Errno::LOOP.raw_os_error()) => {
            let link = fs::symlink_metadata(token).map_err(read)?;
            (None, Some((link.mode(), link.uid())))
        }
        Err(source) => return Err(read(source)),
    };
    ready::system(passwd, found, program)?;
    let Some(file) = file else {
        return Err(NotReady::TokenOwner.into());
    };
    let record = match private_file::read_limited(file, LIMIT) {
        Ok(record) => record,
        Err(error) if error.kind() == io::ErrorKind::FileTooLarge => {
            return Err(NotReady::SystemTokenCorrupt.into());
        }
        Err(source) => return Err(read(source)),
    };
    ready::system_token(&record, now, program)?;
    Ok(())
}

/// Opens `token` for reading: never through a link at its end, never waiting on a pipe, never
/// as the controlling terminal.
fn open(token: &Path) -> io::Result<File> {
    let flags = OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::NOCTTY;
    OpenOptions::new()
        .read(true)
        .custom_flags(flags.bits().cast_signed())
        .open(token)
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    use agent_core::wire::AgentToken;
    use rustix::fs::{CWD, FileType, Mode};

    use super::*;

    const PROGRAM: &str = "/usr/local/bin/anima-agent";

    /// A folder for the token, and `/etc/passwd` text that names the running user as the
    /// account, so a file this test writes is the account's own.
    fn rig() -> (tempfile::TempDir, PathBuf, String) {
        let dir = tempfile::tempdir().expect("a folder");
        let token = dir.path().join("token");
        let uid = private_file::running_user().expect("a uid");
        let passwd = format!("anima-agent:x:{uid}:{uid}::/var/lib/anima-agent:/bin/false\n");
        (dir, token, passwd)
    }

    fn now() -> DateTime<Utc> {
        DateTime::from_timestamp(1_800_000_000, 0).expect("a time")
    }

    fn record() -> String {
        let token = AgentToken::from_random([9; 32]);
        format!(
            r#"{{"token":"{}","expiresAt":"2028-01-01T00:00:00Z","hub":"https://hub.example"}}"#,
            token.expose()
        )
    }

    fn checked(token: &Path, passwd: &str) -> Result<(), SystemTokenError> {
        check(token, passwd, now(), PROGRAM)
    }

    fn not_ready(result: Result<(), SystemTokenError>) -> NotReady {
        match result {
            Err(SystemTokenError::NotReady(not_ready)) => not_ready,
            other => panic!("not a NotReady: {other:?}"),
        }
    }

    #[test]
    fn met_019_the_account_s_own_token_is_ready_and_a_missing_one_is_not() {
        let (dir, token, passwd) = rig();
        assert!(matches!(
            not_ready(checked(&token, &passwd)),
            NotReady::SystemNotEnrolled { .. }
        ));
        private_file::write(dir.path(), "token", record().as_bytes()).expect("written");
        checked(&token, &passwd).expect("ready");
        assert_eq!(
            not_ready(checked(&token, "root:x:0:0::/root:/bin/bash\n")),
            NotReady::MissingAccount
        );
    }

    #[test]
    fn met_019_a_token_that_is_a_link_is_refused_not_followed() {
        let (dir, token, passwd) = rig();
        private_file::write(dir.path(), "real", record().as_bytes()).expect("written");
        std::os::unix::fs::symlink(dir.path().join("real"), &token).expect("link");
        assert_eq!(not_ready(checked(&token, &passwd)), NotReady::TokenOwner);
    }

    #[test]
    fn met_019_a_token_that_is_a_pipe_is_refused_without_blocking() {
        let (_dir, token, passwd) = rig();
        rustix::fs::mknodat(CWD, &token, FileType::Fifo, Mode::RUSR | Mode::WUSR, 0)
            .expect("a pipe");
        let (sent, answer) = mpsc::channel();
        thread::spawn(move || sent.send(not_ready(checked(&token, &passwd))));
        let result = answer.recv_timeout(Duration::from_secs(5));
        assert_eq!(
            result.expect("the open did not block"),
            NotReady::TokenOwner
        );
    }

    #[test]
    fn met_019_an_oversize_token_is_refused_unread() {
        let (dir, token, passwd) = rig();
        private_file::write(dir.path(), "token", &vec![b' '; LIMIT + 1]).expect("written");
        assert_eq!(
            not_ready(checked(&token, &passwd)),
            NotReady::SystemTokenCorrupt
        );
    }
}
