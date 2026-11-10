//! The whole install against a stand-in agent: a shell script that records its arguments, the
//! variables it got, its folder and its standard input. HOME is a temporary folder in the
//! crate's own `target/`, outside `/tmp` (as T6a's rig), so no real home is written.

use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, Instant},
};

use tempfile::TempDir;

use super::CODE;
use crate::agent::{
    AgentError, Deadlines, Here, install_here,
    request::{EnrolmentCode, HubOrigin},
    run::agent_env,
};

const HUB: &str = "https://hub.test";
const WIDE: Deadlines = Deadlines {
    enrol: Duration::from_secs(30),
    install: Duration::from_secs(30),
};

struct Rig {
    dir: TempDir,
    home: PathBuf,
    source: PathBuf,
    record: PathBuf,
}

/// A HOME and a stand-in agent that records each run, then runs `then` (`exit 0` by default).
fn rig(then: &str) -> Rig {
    let scratch = concat!(env!("CARGO_MANIFEST_DIR"), "/target/agent-rig");
    fs::create_dir_all(scratch).expect("the scratch folder");
    let dir = tempfile::tempdir_in(scratch).expect("a folder");
    let home = dir.path().join("home");
    let app = dir.path().join("app");
    fs::create_dir(&home).expect("home");
    fs::create_dir(&app).expect("app");
    let record = dir.path().join("record");
    let source = app.join("anima-agent");
    let script = format!(
        "#!/bin/sh\n{{ printf 'args:%s\\nhome:%s\\nxdg:%s\\ncargo:%s\\npwd:%s\\nstdin:' \"$*\" \"$HOME\" \"${{XDG_RUNTIME_DIR-unset}}\" \"${{CARGO_PKG_NAME-unset}}\" \"$(pwd)\"; /bin/cat; printf '|\\n'; }} >> '{}'\n{then}\n",
        record.display()
    );
    fs::write(&source, script).expect("the stand-in");
    fs::set_permissions(&source, fs::Permissions::from_mode(0o700)).expect("chmod");
    Rig {
        dir,
        home,
        source,
        record,
    }
}

impl Rig {
    fn here(&self) -> Here {
        let lookup = |name: &str| (name == "XDG_RUNTIME_DIR").then(|| "/run/user/test".to_owned());
        Here {
            source: self.source.clone(),
            home: self.home.clone(),
            uid: nix::unistd::getuid().as_raw(),
            env: agent_env(&self.home, lookup),
        }
    }

    fn install(&self, deadlines: Deadlines) -> Result<PathBuf, AgentError> {
        let code = EnrolmentCode::parse(CODE).expect("a code");
        let hub = HubOrigin::parse(HUB).expect("an origin");
        install_here(&self.here(), &code, &hub, deadlines)
    }

    fn installed(&self) -> PathBuf {
        self.home.join(".local/bin/anima-agent")
    }

    fn runs(&self) -> String {
        fs::read_to_string(&self.record).unwrap_or_default()
    }

    fn run_of(&self, args: &str, stdin: &str) -> String {
        format!(
            "args:{args}\nhome:{}\nxdg:/run/user/test\ncargo:unset\npwd:/\nstdin:{stdin}|\n",
            self.home.display()
        )
    }
}

fn names_in(folder: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(folder)
        .expect("a folder")
        .map(|entry| {
            entry
                .expect("an entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    names
}

#[test]
fn met_019_the_agent_is_copied_then_enrolled_with_the_code_on_stdin_then_installed() {
    let rig = rig("exit 0");
    assert_eq!(rig.install(WIDE), Ok(rig.installed()));
    let installed = rig.installed();
    assert_eq!(fs::read(&installed).ok(), fs::read(&rig.source).ok());
    let mode = fs::metadata(&installed)
        .expect("installed")
        .permissions()
        .mode();
    assert_eq!(mode & 0o7777, 0o755);
    let enrol = rig.run_of("enrol --hub https://hub.test -", &format!("{CODE}\n"));
    assert_eq!(rig.runs(), enrol + &rig.run_of("install", ""));
    // The copy went in by rename: no part file is left beside it.
    assert_eq!(
        names_in(installed.parent().expect("a folder")),
        ["anima-agent"]
    );
}

#[test]
fn met_019_a_failed_enrolment_stops_before_install() {
    let rig = rig("[ \"$1\" = enrol ] && exit 3; exit 0");
    assert_eq!(rig.install(WIDE), Err(AgentError::Enrol));
    assert!(!rig.runs().contains("args:install"), "{}", rig.runs());
}

#[test]
fn met_019_a_failed_install_after_enrolment_is_its_own_error() {
    let rig = rig("[ \"$1\" = install ] && exit 1; exit 0");
    assert_eq!(rig.install(WIDE), Err(AgentError::Install));
    assert!(rig.runs().contains("args:enrol"), "{}", rig.runs());
}

#[test]
fn met_019_an_agent_past_its_deadline_is_killed_and_reaped() {
    let rig = rig("");
    let pid_file = rig.dir.path().join("pid");
    let script = format!(
        "{}\necho $$ > '{}'; exec /bin/sleep 30\n",
        fs::read_to_string(&rig.source).expect("the stand-in"),
        pid_file.display()
    );
    fs::write(&rig.source, script).expect("the slow stand-in");
    let quick = Deadlines {
        enrol: Duration::from_secs(2),
        install: WIDE.install,
    };
    let started = Instant::now();
    assert_eq!(rig.install(quick), Err(AgentError::TimedOut));
    assert!(started.elapsed() < Duration::from_secs(10));
    let pid = fs::read_to_string(&pid_file).expect("the stand-in wrote its pid");
    let listed = Command::new("/bin/ps")
        .args(["-o", "stat=", "-p", pid.trim()])
        .output()
        .expect("ps");
    // A reaped child is gone; one only killed would still be listed as a zombie.
    assert!(listed.stdout.is_empty(), "process {pid} is still there");
    assert!(!rig.runs().contains("args:install"));
}

#[test]
fn met_019_a_folder_others_can_write_is_refused_before_anything_runs() {
    for open in [".local", ".local/bin", ""] {
        let rig = rig("exit 0");
        let folder = rig.home.join(open);
        fs::create_dir_all(&folder).expect("the folder");
        fs::set_permissions(&folder, fs::Permissions::from_mode(0o777)).expect("chmod");
        assert_eq!(rig.install(WIDE), Err(AgentError::Untrusted), "{open}");
        assert!(!rig.installed().exists(), "{open}");
        assert_eq!(rig.runs(), "", "{open}");
    }
}

#[test]
fn met_019_a_missing_or_linked_agent_is_not_bundled() {
    let rig = rig("exit 0");
    let moved = rig.dir.path().join("elsewhere");
    fs::rename(&rig.source, &moved).expect("moved");
    assert_eq!(rig.install(WIDE), Err(AgentError::NotBundled));
    symlink(&moved, &rig.source).expect("a link");
    assert_eq!(rig.install(WIDE), Err(AgentError::NotBundled));
    assert_eq!(rig.runs(), "");
}

#[test]
fn met_019_a_link_at_the_destination_is_replaced_not_followed() {
    let rig = rig("exit 0");
    let victim = rig.dir.path().join("victim");
    fs::write(&victim, b"keep").expect("the victim");
    fs::create_dir_all(rig.home.join(".local/bin")).expect("bin");
    symlink(&victim, rig.installed()).expect("a link");
    assert_eq!(rig.install(WIDE), Ok(rig.installed()));
    assert_eq!(fs::read(&victim).ok(), Some(b"keep".to_vec()));
    let installed = fs::symlink_metadata(rig.installed()).expect("installed");
    assert!(installed.is_file(), "the link was followed");
}
