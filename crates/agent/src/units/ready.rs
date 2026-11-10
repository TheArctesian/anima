//! Whether the agent the unit starts will find a token it can use (MET-019, D-10): an agent
//! without one fails and restarts for ever. Pure: the caller reads the files.

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};

use super::plan::{ACCOUNT, SYSTEM_HOME};
use crate::config::private_file;

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum NotReady {
    #[error(
        "this agent is not enrolled: run `anima-agent enrol -` (the code on standard input) first"
    )]
    NotEnrolled,
    #[error(
        "this agent's token has expired: enrol again with `anima-agent enrol -`, then install again"
    )]
    Expired,
    #[error(
        "this shell's agent folder is {}, but the agent the unit starts uses {}: unset ANIMA_AGENT_HOME and XDG_CONFIG_HOME, enrol, then install again",
        found.display(),
        unit.display()
    )]
    OtherHome { found: PathBuf, unit: PathBuf },
    #[error(
        "the system unit runs as the account {ACCOUNT}, which does not exist: run `useradd --system --user-group --no-create-home --home-dir {SYSTEM_HOME} --shell /usr/sbin/nologin {ACCOUNT}`, then install again"
    )]
    MissingAccount,
    #[error(
        "the system unit has no token: as root run `install -d -m 0700 -o {ACCOUNT} -g {ACCOUNT} {SYSTEM_HOME}`, then `sudo -u {ACCOUNT} env ANIMA_AGENT_HOME={SYSTEM_HOME} {program} enrol -` and paste the code at the hidden prompt, then install again"
    )]
    SystemNotEnrolled { program: String },
    #[error(
        "{SYSTEM_HOME}/token must be mode 0600 and owned by the {ACCOUNT} account, which otherwise cannot read it: as root run `chown {ACCOUNT}:{ACCOUNT} {SYSTEM_HOME}/token` and `chmod 0600 {SYSTEM_HOME}/token`"
    )]
    TokenOwner,
}

/// The folder this shell found (`found`) must be the one the unit's agent finds (`unit`): the
/// unit sets no `ANIMA_AGENT_HOME` and the user unit may write only its default folder.
///
/// # Errors
/// [`NotReady::OtherHome`] when they differ.
pub(super) fn same_home(found: &Path, unit: &Path) -> Result<(), NotReady> {
    if found == unit {
        Ok(())
    } else {
        Err(NotReady::OtherHome {
            found: found.to_owned(),
            unit: unit.to_owned(),
        })
    }
}

/// The stored token's expiry (`None` when there is no token) must be after `now`.
///
/// # Errors
/// [`NotReady::NotEnrolled`] or [`NotReady::Expired`].
pub(super) fn token_usable(
    expires_at: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
) -> Result<(), NotReady> {
    match expires_at {
        None => Err(NotReady::NotEnrolled),
        Some(expires_at) if expires_at <= now => Err(NotReady::Expired),
        Some(_) => Ok(()),
    }
}

/// The system unit's agent runs as [`ACCOUNT`]: it must exist in `passwd` (the text of
/// `/etc/passwd`), and its token file (`token`: its mode and owner, `None` when there is none)
/// must be the account's own 0600 file. Root cannot read the account's token as it, so the
/// token itself is not opened. `program` is printed in the enrol command.
///
/// # Errors
/// [`NotReady::MissingAccount`], [`NotReady::SystemNotEnrolled`] or [`NotReady::TokenOwner`].
pub(super) fn system(
    passwd: &str,
    token: Option<(u32, u32)>,
    program: &str,
) -> Result<(), NotReady> {
    let account = account_uid(passwd, ACCOUNT).ok_or(NotReady::MissingAccount)?;
    let (mode, owner) = token.ok_or_else(|| NotReady::SystemNotEnrolled {
        program: program.to_owned(),
    })?;
    if private_file::exposed(mode, owner, account) {
        return Err(NotReady::TokenOwner);
    }
    Ok(())
}

/// The user ID of the account `name` in `passwd`; `None` when it is missing or its line does
/// not read.
fn account_uid(passwd: &str, name: &str) -> Option<u32> {
    passwd.lines().find_map(|line| {
        let mut fields = line.split(':');
        if fields.next() != Some(name) {
            return None;
        }
        fields.nth(1)?.parse().ok()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const PASSWD: &str = "root:x:0:0::/root:/bin/bash\nanima-agent:x:998:998::/var/lib/anima-agent:/usr/sbin/nologin\n";

    #[test]
    fn met_019_an_account_is_found_by_its_whole_name_and_its_uid_read() {
        let passwd = "root:x:0:0::/root:/bin/bash\nanima-agent-old:x:5:5::/:/bin/false\n";
        assert_eq!(account_uid(passwd, "root"), Some(0));
        assert_eq!(account_uid(passwd, "anima-agent"), None);
        assert_eq!(account_uid(PASSWD, "anima-agent"), Some(998));
        assert_eq!(
            account_uid("anima-agent:x:nine:9::/:/bin/false", ACCOUNT),
            None
        );
        assert_eq!(account_uid("anima-agent", ACCOUNT), None);
    }

    #[test]
    fn met_019_system_readiness_wants_the_account_then_its_own_private_token() {
        let program = "/usr/local/bin/anima-agent";
        let missing = system(
            "root:x:0:0::/root:/bin/bash\n",
            Some((0o100_600, 998)),
            program,
        );
        assert_eq!(missing, Err(NotReady::MissingAccount));
        assert!(
            NotReady::MissingAccount
                .to_string()
                .contains("useradd --system --user-group")
        );
        let none = system(PASSWD, None, program).expect_err("no token");
        assert_eq!(
            none,
            NotReady::SystemNotEnrolled {
                program: program.to_owned()
            }
        );
        assert!(none.to_string().contains(&format!("{program} enrol -")));
        assert!(none.to_string().contains("as root run `install -d"));
        assert!(
            !none.to_string().contains("<code>"),
            "the code never goes on a command line"
        );
        assert!(
            none.to_string()
                .contains("paste the code at the hidden prompt")
        );
        assert_eq!(
            system(PASSWD, Some((0o100_600, 0)), program),
            Err(NotReady::TokenOwner),
            "root's token is one the agent cannot read"
        );
        assert_eq!(
            system(PASSWD, Some((0o100_644, 998)), program),
            Err(NotReady::TokenOwner)
        );
        assert_eq!(system(PASSWD, Some((0o100_600, 998)), program), Ok(()));
    }

    #[test]
    fn met_019_a_user_token_must_be_there_and_not_expired() {
        let now = DateTime::from_timestamp(1_800_000_000, 0).expect("a time");
        assert_eq!(token_usable(None, now), Err(NotReady::NotEnrolled));
        assert_eq!(token_usable(Some(now), now), Err(NotReady::Expired));
        let later = now + chrono::TimeDelta::seconds(1);
        assert_eq!(token_usable(Some(later), now), Ok(()));
    }

    #[test]
    fn met_019_the_shell_s_agent_folder_must_be_the_unit_s() {
        let unit = Path::new("/home/ada/.config/anima-agent");
        assert_eq!(same_home(unit, unit), Ok(()));
        let other = same_home(Path::new("/srv/cfg/anima-agent"), unit).expect_err("other");
        assert!(other.to_string().contains("unset ANIMA_AGENT_HOME"));
    }
}
