//! Whether a file or folder the unit relies on can be changed only by root or the user who
//! installs it (MET-019): pure; the caller reads the modes and the account files.

use super::plan::Target;

/// A file's mode, owner and group, as `stat` gives them.
#[derive(Clone, Copy, Debug)]
pub(super) struct Stat {
    pub mode: u32,
    pub owner: u32,
    pub group: u32,
}

/// Who installs, and which unit.
#[derive(Clone, Copy, Debug)]
pub(super) struct Installer {
    pub target: Target,
    pub uid: u32,
    /// The user's private group (see [`private_group`]); `None` when there is none.
    pub private_group: Option<u32>,
}

/// One account line of `/etc/passwd`.
pub(super) struct Account<'a> {
    pub name: &'a str,
    pub uid: u32,
    pub gid: u32,
}

/// The accounts in `passwd` (the text of `/etc/passwd`); a line that does not read is skipped.
pub(super) fn accounts(passwd: &str) -> impl Iterator<Item = Account<'_>> {
    passwd.lines().filter_map(|line| {
        let mut fields = line.split(':');
        let name = fields.next()?;
        let uid = fields.nth(1)?.parse().ok()?;
        let gid = fields.next()?.parse().ok()?;
        Some(Account { name, uid, gid })
    })
}

/// The group only user `uid` is in, from `passwd` and `group` (the text of `/etc/group`): the
/// user's primary group, named as the user, listing no other member, and no other account's
/// primary group. `umask 002` homes rely on such a group; any other group may hold other users.
pub(super) fn private_group(uid: u32, passwd: &str, group: &str) -> Option<u32> {
    let user = accounts(passwd).find(|account| account.uid == uid)?;
    if accounts(passwd).any(|account| account.gid == user.gid && account.uid != uid) {
        return None;
    }
    let mut found = false;
    for line in group.lines() {
        let mut fields = line.split(':');
        let (Some(name), Some(gid), members) = (fields.next(), fields.nth(1), fields.next()) else {
            continue;
        };
        if gid.parse() != Ok(user.gid) {
            continue;
        }
        let only_user = members
            .unwrap_or("")
            .split(',')
            .all(|member| member.is_empty() || member == user.name);
        if name != user.name || !only_user {
            return None;
        }
        found = true;
    }
    found.then_some(user.gid)
}

/// The group may write to what `stat` describes: it does not, or it is owned by the user and
/// its group is the user's private one.
fn group_write_safe(stat: Stat, by: Installer) -> bool {
    stat.mode & 0o020 == 0 || (stat.owner == by.uid && by.private_group == Some(stat.group))
}

/// Whether a folder on the binary's or the unit's path may stay there. For the user unit:
/// owned by root or the user, and either sticky or not writable by others, and its group may
/// write only when it is the user's folder in the user's private group. For the system unit,
/// which runs the binary as another account: owned by root and writable by no one else, sticky
/// or not.
pub(super) fn folder_trusted(stat: Stat, by: Installer) -> bool {
    match by.target {
        Target::User => {
            let owner = stat.owner == by.uid || stat.owner == 0;
            let sticky = stat.mode & 0o1000 != 0;
            owner && (sticky || (stat.mode & 0o002 == 0 && group_write_safe(stat, by)))
        }
        Target::System => stat.owner == 0 && stat.mode & 0o022 == 0,
    }
}

/// Whether the binary itself may be started by the unit: owned by root or (user unit) the
/// user, not writable by others, and writable by its group only as [`folder_trusted`] allows.
pub(super) fn binary_trusted(stat: Stat, by: Installer) -> bool {
    match by.target {
        Target::User => {
            (stat.owner == by.uid || stat.owner == 0)
                && stat.mode & 0o002 == 0
                && group_write_safe(stat, by)
        }
        Target::System => stat.owner == 0 && stat.mode & 0o022 == 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PASSWD: &str = "root:x:0:0::/root:/bin/bash\nada:x:1000:1000::/home/ada:/bin/bash\nbob:x:1001:1001::/home/bob:/bin/bash\nweb:x:33:100::/:/bin/false\neve:x:1002:100::/home/eve:/bin/bash\n";
    const GROUP: &str =
        "root:x:0:\nadm:x:4:ada,bob\nada:x:1000:\nbob:x:1001:bob,ada\nusers:x:100:\n";

    const fn stat(mode: u32, owner: u32, group: u32) -> Stat {
        Stat { mode, owner, group }
    }

    const ADA: Installer = Installer {
        target: Target::User,
        uid: 1000,
        private_group: Some(1000),
    };

    #[test]
    fn met_019_a_private_group_is_the_user_s_own_named_and_shared_with_no_one() {
        assert_eq!(private_group(1000, PASSWD, GROUP), Some(1000));
        assert_eq!(private_group(1001, PASSWD, GROUP), None, "lists ada too");
        assert_eq!(private_group(1002, PASSWD, GROUP), None, "users is shared");
        assert_eq!(private_group(4242, PASSWD, GROUP), None, "no account");
        assert_eq!(
            private_group(1000, PASSWD, "root:x:0:\n"),
            None,
            "no group line"
        );
        let twice = format!("{GROUP}staff:x:1000:bob\n");
        assert_eq!(
            private_group(1000, PASSWD, &twice),
            None,
            "another name, same gid"
        );
    }

    #[test]
    fn met_019_group_write_is_trusted_only_in_the_user_s_private_group() {
        assert!(
            folder_trusted(stat(0o040_775, 1000, 1000), ADA),
            "~/.local/bin, umask 002"
        );
        assert!(
            !folder_trusted(stat(0o040_775, 1000, 4), ADA),
            "group adm may write the user's folder"
        );
        let no_private = Installer {
            private_group: None,
            ..ADA
        };
        assert!(!folder_trusted(stat(0o040_775, 1000, 1000), no_private));
        assert!(
            !folder_trusted(stat(0o042_775, 0, 1000), ADA),
            "root's, not the user's"
        );
    }

    #[test]
    fn met_019_a_binary_from_umask_002_is_trusted_and_one_others_can_write_is_not() {
        assert!(
            binary_trusted(stat(0o100_775, 1000, 1000), ADA),
            "cargo under umask 002"
        );
        assert!(!binary_trusted(stat(0o100_775, 1000, 4), ADA), "group adm");
        assert!(binary_trusted(stat(0o100_755, 1000, 4), ADA));
        assert!(binary_trusted(stat(0o100_755, 0, 0), ADA));
        assert!(!binary_trusted(stat(0o100_777, 1000, 1000), ADA));
        assert!(!binary_trusted(stat(0o100_755, 1001, 1001), ADA));
        let root = Installer {
            target: Target::System,
            uid: 0,
            private_group: Some(0),
        };
        assert!(binary_trusted(stat(0o100_755, 0, 0), root));
        assert!(!binary_trusted(stat(0o100_775, 0, 0), root));
        assert!(!binary_trusted(stat(0o100_755, 1000, 1000), root));
    }

    #[test]
    fn met_019_a_folder_others_can_replace_in_is_not_trusted_and_the_system_unit_wants_root_s() {
        assert!(folder_trusted(stat(0o040_755, 1000, 1000), ADA));
        assert!(
            folder_trusted(stat(0o041_777, 0, 0), ADA),
            "sticky, like /tmp"
        );
        assert!(!folder_trusted(stat(0o040_777, 0, 0), ADA));
        assert!(!folder_trusted(stat(0o040_702, 1000, 1000), ADA));
        assert!(
            !folder_trusted(stat(0o040_755, 1001, 1001), ADA),
            "another user's"
        );
        assert!(
            !folder_trusted(stat(0o041_777, 1001, 1001), ADA),
            "another user's sticky folder: its owner may still rename what is in it"
        );
        let root = Installer {
            target: Target::System,
            uid: 0,
            private_group: Some(0),
        };
        assert!(folder_trusted(stat(0o040_755, 0, 0), root));
        assert!(
            !folder_trusted(stat(0o040_755, 1000, 0), root),
            "not root's"
        );
        assert!(
            !folder_trusted(stat(0o040_775, 0, 0), root),
            "group may write"
        );
        assert!(
            !folder_trusted(stat(0o041_777, 0, 0), root),
            "sticky is not enough"
        );
    }
}
