//! Whether a folder on the agent binary's path can be changed only by root or the user who
//! installs it (MET-019). Pure; the caller reads the modes, the ACL and the account files.
//!
//! This is T6a's user-unit rule (`crates/agent/src/units/trust.rs`, `folder_trusted` with
//! `Target::User`, and `private_group`), copied because the shell cannot depend on the agent
//! crate. THE TWO RULES MUST STAY THE SAME: the shell hands the code to the binary before the
//! agent's own `install` check runs, so a looser rule here leaks the code to a swapped binary.
//! The one difference is stricter: a sticky folder others can write (`/tmp`) is refused here,
//! since the agent's units hide `/tmp` and its `install` would refuse after the code was spent.

/// A folder's mode, owner and group, as `stat` gives them, and whether it has a POSIX access
/// ACL. With an ACL the group bits of the mode are the ACL's mask, which named users and groups
/// get too. macOS ACLs (`chmod +a`) do not show in the mode and are not read; T6a has the same
/// limit.
#[derive(Clone, Copy, Debug)]
pub struct Stat {
    pub mode: u32,
    pub owner: u32,
    pub group: u32,
    pub acl: bool,
}

/// The user who installs: their ID and their private group (see [`private_group`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct User {
    pub uid: u32,
    pub private_group: Option<u32>,
}

/// One account line of `/etc/passwd`.
struct Account<'a> {
    name: &'a str,
    uid: u32,
    gid: u32,
}

/// The account on one `/etc/passwd` line; `None` when it does not read. The IDs are trimmed.
fn account(line: &str) -> Option<Account<'_>> {
    let mut fields = line.split(':');
    let name = fields.next()?;
    let uid = fields.nth(1)?.trim().parse().ok()?;
    let gid = fields.next()?.trim().parse().ok()?;
    Some(Account { name, uid, gid })
}

fn accounts(passwd: &str) -> impl Iterator<Item = Account<'_>> {
    passwd.lines().filter_map(account)
}

/// The group only user `uid` is in, from `passwd` and `group` (the texts of `/etc/passwd` and
/// `/etc/group`; `None` when unreadable): the user's primary group, named as the user, listing
/// no other member, and no other account's primary group. Any line that does not read means no
/// private group, so group write is then refused (fails closed).
pub fn private_group(uid: u32, passwd: Option<&str>, group: Option<&str>) -> Option<u32> {
    let (passwd, group) = (passwd?, group?);
    if passwd
        .lines()
        .any(|line| !line.is_empty() && account(line).is_none())
    {
        return None;
    }
    let user = accounts(passwd).find(|account| account.uid == uid)?;
    if accounts(passwd).any(|account| account.gid == user.gid && account.uid != uid) {
        return None;
    }
    let mut found = false;
    for line in group.lines().filter(|line| !line.is_empty()) {
        let mut fields = line.split(':');
        let (Some(name), Some(gid), members) = (fields.next(), fields.nth(1), fields.next()) else {
            return None;
        };
        let Ok(gid) = gid.trim().parse::<u32>() else {
            return None;
        };
        if gid != user.gid {
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

/// The group may write: it does not, or the folder is the user's, in the user's private group,
/// and no ACL lets anyone else in through the mask.
fn group_write_safe(stat: Stat, by: User) -> bool {
    stat.mode & 0o020 == 0
        || (stat.owner == by.uid && by.private_group == Some(stat.group) && !stat.acl)
}

/// Whether a folder may hold, or lie above, the binary the user's unit starts at each login:
/// owned by the user or root, not writable by others (sticky or not), and writable by its group
/// only when it is the user's folder in the user's private group with no ACL.
pub fn folder_trusted(stat: Stat, by: User) -> bool {
    (stat.owner == by.uid || stat.owner == 0)
        && stat.mode & 0o002 == 0
        && group_write_safe(stat, by)
}
