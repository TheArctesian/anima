//! The folder rule, with T6a's cases for the user unit (`crates/agent/src/units/trust.rs`), so
//! the two rules are held to the same answers; the shell differs only by refusing a sticky
//! folder others can write.

use crate::agent::trust::{Stat, User, folder_trusted, private_group};

const PASSWD: &str = "root:x:0:0::/root:/bin/bash\nada:x:1000:1000::/home/ada:/bin/bash\nbob:x:1001:1001::/home/bob:/bin/bash\nweb:x:33:100::/:/bin/false\neve:x:1002:100::/home/eve:/bin/bash\n";
const GROUP: &str = "root:x:0:\nadm:x:4:ada,bob\nada:x:1000:\nbob:x:1001:bob,ada\nusers:x:100:\n";

const ADA: User = User {
    uid: 1000,
    private_group: Some(1000),
};

const fn stat(mode: u32, owner: u32, group: u32) -> Stat {
    Stat {
        mode,
        owner,
        group,
        acl: false,
    }
}

fn private(uid: u32, passwd: &str, group: Option<&str>) -> Option<u32> {
    private_group(uid, Some(passwd), group)
}

#[test]
fn met_019_a_private_group_is_the_user_s_own_named_and_shared_with_no_one() {
    assert_eq!(private(1000, PASSWD, Some(GROUP)), Some(1000));
    assert_eq!(private(1001, PASSWD, Some(GROUP)), None, "lists ada too");
    assert_eq!(private(1002, PASSWD, Some(GROUP)), None, "users is shared");
    assert_eq!(private(4242, PASSWD, Some(GROUP)), None, "no account");
    assert_eq!(private(1000, PASSWD, Some("root:x:0:\n")), None, "no line");
    let renamed = GROUP.replace("ada:x:1000:", "devs:x:1000:");
    assert_eq!(private(1000, PASSWD, Some(&renamed)), None, "other name");
    let shared = format!("{PASSWD}mallory:x:1003:1000::/:/bin/sh\n");
    assert_eq!(
        private(1000, &shared, Some(GROUP)),
        None,
        "another's primary"
    );
    let twice = format!("{GROUP}staff:x:1000:bob\n");
    assert_eq!(private(1000, PASSWD, Some(&twice)), None, "same gid again");
}

#[test]
fn met_019_no_private_group_when_a_line_or_a_file_does_not_read() {
    assert_eq!(private(1000, PASSWD, None), None, "no /etc/group");
    assert_eq!(
        private_group(1000, None, Some(GROUP)),
        None,
        "no /etc/passwd"
    );
    let spaced = GROUP.replace("ada:x:1000:", "ada:x: 1000 :");
    assert_eq!(private(1000, PASSWD, Some(&spaced)), Some(1000), "trimmed");
    let odd = format!("{GROUP}mallory:x:10x0:ada,mallory\n");
    assert_eq!(private(1000, PASSWD, Some(&odd)), None, "odd gid");
    let short = format!("{GROUP}mallory\n");
    assert_eq!(private(1000, PASSWD, Some(&short)), None, "short line");
    let broken = format!("{PASSWD}mallory:x:1003:1000x::/:/bin/sh\n");
    assert_eq!(private(1000, &broken, Some(GROUP)), None, "odd passwd");
}

#[test]
fn met_019_group_write_is_trusted_only_in_the_user_s_private_group() {
    assert!(
        folder_trusted(stat(0o040_775, 1000, 1000), ADA),
        "umask 002"
    );
    assert!(
        !folder_trusted(stat(0o040_775, 1000, 4), ADA),
        "chgrp adm, 0775: group adm may swap the agent"
    );
    let no_private = User {
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
fn met_019_an_acl_makes_group_write_untrusted_even_in_the_private_group() {
    let with_acl = Stat {
        acl: true,
        ..stat(0o040_775, 1000, 1000)
    };
    assert!(!folder_trusted(with_acl, ADA), "setfacl -m u:nobody:rwx");
    let masked = Stat {
        acl: true,
        ..stat(0o040_755, 1000, 1000)
    };
    assert!(folder_trusted(masked, ADA), "a mask without write");
}

#[test]
fn met_019_a_folder_is_the_user_s_or_root_s_and_others_cannot_write_it_sticky_or_not() {
    assert!(folder_trusted(stat(0o040_755, 1000, 1000), ADA));
    assert!(folder_trusted(stat(0o040_700, 1000, 1000), ADA));
    assert!(folder_trusted(stat(0o040_755, 0, 0), ADA));
    assert!(
        !folder_trusted(stat(0o040_755, 1001, 1001), ADA),
        "another's"
    );
    assert!(!folder_trusted(stat(0o040_777, 1000, 1000), ADA));
    assert!(!folder_trusted(stat(0o040_702, 1000, 1000), ADA));
    // The one difference from T6a: a sticky folder others can write is refused before the code
    // is spent, since the agent's units hide /tmp and its install would refuse it.
    assert!(
        !folder_trusted(stat(0o041_777, 0, 0), ADA),
        "sticky, like /tmp"
    );
}
