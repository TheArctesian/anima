//! The agent's answers to the table of trust cases it shares with the desktop shell
//! (`tests/trust-cases.txt`); the shell's side is `apps/desktop/src-tauri/src/agent/tests/trust.rs`.

use super::plan::Target;
use super::trust::{Installer, Stat, folder_trusted, private_group};

const TABLE: &str = include_str!("../../tests/trust-cases.txt");

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
fn met_019_every_row_of_the_trust_table_gets_the_agent_s_answer() {
    let cases = cases();
    assert!(cases.len() >= 20, "the table is read");
    for case in cases {
        let passwd = case.passwd.as_deref().unwrap_or("");
        let private = private_group(case.uid, passwd, case.group_file.as_deref());
        assert_eq!(private, case.private, "{}: private group", case.name);
        let by = Installer {
            target: Target::User,
            uid: case.uid,
            private_group: private,
        };
        let stat = Stat {
            mode: case.mode,
            owner: case.owner,
            group: case.group,
            acl: case.acl,
        };
        assert_eq!(folder_trusted(stat, by), case.agent, "{}", case.name);
        assert_eq!(
            case.differs,
            case.agent != case.shell,
            "{}: DIFF mark",
            case.name
        );
    }
}
