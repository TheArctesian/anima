//! The folder rule, held to the same answers as T6a's for the user unit
//! (`crates/agent/src/units/trust.rs`) by one table, `crates/agent/tests/trust-cases.txt`, that
//! both test suites read. The shell may be stricter, never looser, and differs in two ways, both
//! pinned there (rows marked `DIFF`): it refuses a sticky folder others can write (`/tmp`, 1777),
//! and a sticky folder whose group may write when the group is not the user's private one
//! (1775 in a shared group), where the agent's user rule allows both.

use crate::agent::trust::{Stat, User, folder_trusted, private_group};

const TABLE: &str = include_str!("../../../../../../crates/agent/tests/trust-cases.txt");

const PASSWD: &str = "root:x:0:0::/root:/bin/bash\nada:x:1000:1000::/home/ada:/bin/bash\nbob:x:1001:1001::/home/bob:/bin/bash\nweb:x:33:100::/:/bin/false\neve:x:1002:100::/home/eve:/bin/bash\n";
const GROUP: &str = "root:x:0:\nadm:x:4:ada,bob\nada:x:1000:\nbob:x:1001:bob,ada\nusers:x:100:\n";

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

/// One row of the table of trust cases, with its account files resolved.
#[expect(
    clippy::struct_excessive_bools,
    reason = "a row of the table: each flag is one of its columns"
)]
struct Case {
    name: String,
    mode: u32,
    owner: u32,
    group: u32,
    acl: bool,
    uid: u32,
    passwd: Option<String>,
    group_file: Option<String>,
    private: Option<u32>,
    agent: bool,
    shell: bool,
    differs: bool,
}

/// The rows of `trust-cases.txt`; a malformed row panics, so the table cannot rot quietly.
fn cases() -> Vec<Case> {
    let mut sets = std::collections::HashMap::new();
    let mut rows = Vec::new();
    for line in TABLE
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
    {
        if let Some(rest) = line
            .strip_prefix("passwd ")
            .map(|r| ("passwd", r))
            .or_else(|| line.strip_prefix("group ").map(|r| ("group", r)))
        {
            let (name, text) = rest.1.split_once(' ').expect("set name and text");
            let text = (text != "none").then(|| text.replace(';', "\n") + "\n");
            sets.insert((rest.0, name.to_owned()), text);
        } else {
            rows.push(line);
        }
    }
    let set = |kind: &str, name: &str| {
        sets.get(&(kind, name.to_owned()))
            .unwrap_or_else(|| panic!("no {kind} set {name}"))
            .clone()
    };
    let octal = |s: &str| u32::from_str_radix(s, 8).expect("octal");
    rows.into_iter()
        .map(|row| {
            let f: Vec<&str> = row.split('|').map(str::trim).collect();
            assert_eq!(f.len(), 12, "row {row}");
            Case {
                name: f[0].to_owned(),
                mode: octal(f[1]),
                owner: f[2].parse().expect("owner"),
                group: f[3].parse().expect("group"),
                acl: f[4] == "1",
                uid: f[5].parse().expect("uid"),
                passwd: set("passwd", f[6]),
                group_file: set("group", f[7]),
                private: (f[8] != "-").then(|| f[8].parse().expect("private")),
                agent: f[9] == "trust",
                shell: f[10] == "trust",
                differs: f[11] == "DIFF",
            }
        })
        .collect()
}

#[test]
fn met_019_every_row_of_the_trust_table_gets_the_shell_s_answer() {
    let cases = cases();
    assert!(cases.len() >= 20, "the table is read");
    for case in cases {
        let private = private_group(case.uid, case.passwd.as_deref(), case.group_file.as_deref());
        assert_eq!(private, case.private, "{}: private group", case.name);
        let by = User {
            uid: case.uid,
            private_group: private,
        };
        let stat = Stat {
            mode: case.mode,
            owner: case.owner,
            group: case.group,
            acl: case.acl,
        };
        assert_eq!(folder_trusted(stat, by), case.shell, "{}", case.name);
    }
}

#[test]
fn met_019_the_shell_never_trusts_what_the_agent_refuses() {
    for case in cases() {
        assert!(
            !case.shell || case.agent,
            "{}: the shell is looser than the agent's install check",
            case.name
        );
        assert_eq!(
            case.differs,
            case.agent != case.shell,
            "{}: DIFF mark",
            case.name
        );
    }
}
