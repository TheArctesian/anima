//! The agent's answers to the table of trust cases it shares with the desktop shell
//! (`tests/trust-cases.txt`); the shell's side is `apps/desktop/src-tauri/src/agent/tests/trust.rs`.

use super::plan::Target;
use super::trust::{Installer, Stat, folder_trusted, private_group};

include!("../../tests/trust-cases/parse.rs");

#[test]
fn met_019_every_row_of_the_trust_table_gets_the_agent_s_answer() {
    let cases = cases();
    assert!(cases.len() >= 20, "the table is read");
    for case in cases {
        // `install` stops with an error when /etc/passwd cannot be read, so no verdict is asked of
        // it then; an empty file is the nearest thing this check can be given.
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
