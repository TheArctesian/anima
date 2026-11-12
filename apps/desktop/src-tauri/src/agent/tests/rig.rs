//! A temporary HOME in the crate's own `target/`, outside `/tmp` (as T6a's rig), and a stand-in
//! agent: a shell script that records the path it was started as, its arguments, the variables
//! it got, its folder and its standard input. No real home is written.

use std::{
    ffi::OsString,
    fs,
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
    time::Duration,
};

use tempfile::TempDir;

use super::CODE;
use crate::agent::{
    AgentError, Here, Limits, install_here,
    request::{EnrolmentCode, HubOrigin},
    run::agent_env,
    trust::User,
};

const HUB: &str = "https://hub.test";
pub const WIDE: Duration = Duration::from_secs(30);
pub static NEVER: AtomicBool = AtomicBool::new(false);

pub struct Rig {
    pub dir: TempDir,
    pub home: PathBuf,
    pub source: PathBuf,
    pub record: PathBuf,
    pub user: User,
}

/// Panics when a folder above `path` lets others write: the install would rightly refuse it,
/// and every test would fail as a misleading `Untrusted`.
fn require_private_checkout(path: &Path) {
    for folder in path.ancestors() {
        let mode = fs::metadata(folder).expect("a folder").mode();
        assert!(
            mode & 0o002 == 0,
            "checkout must not lie under a world-writable folder ({} is): move it out of /tmp",
            folder.display()
        );
    }
}

/// A HOME and a stand-in agent that records each run, then runs `then`. The user's private
/// group is taken to be the HOME's group, so a folder made under umask 002 is trusted.
pub fn rig(then: &str) -> Rig {
    let scratch = concat!(env!("CARGO_MANIFEST_DIR"), "/target/agent-rig");
    fs::create_dir_all(scratch).expect("the scratch folder");
    require_private_checkout(Path::new(scratch));
    let dir = tempfile::tempdir_in(scratch).expect("a folder");
    let home = dir.path().join("home");
    let app = dir.path().join("app");
    fs::create_dir(&home).expect("home");
    fs::create_dir(&app).expect("app");
    let record = dir.path().join("record");
    let source = app.join("anima-agent");
    let script = format!(
        "#!/bin/sh\n{{ printf 'program:%s\\nargs:%s\\nhome:%s\\nxdg:%s\\ncargo:%s\\npwd:%s\\nstdin:' \"$0\" \"$*\" \"$HOME\" \"${{XDG_RUNTIME_DIR-unset}}\" \"${{CARGO_PKG_NAME-unset}}\" \"$(pwd)\"; /bin/cat; printf '|\\n'; }} >> '{}'\n{then}\n",
        record.display()
    );
    fs::write(&source, script).expect("the stand-in");
    fs::set_permissions(&source, fs::Permissions::from_mode(0o700)).expect("chmod");
    let user = User {
        uid: nix::unistd::getuid().as_raw(),
        private_group: Some(fs::metadata(&home).expect("home").gid()),
    };
    Rig {
        dir,
        home,
        source,
        record,
        user,
    }
}

impl Rig {
    pub fn here(&self) -> Here {
        let lookup =
            |name: &str| (name == "XDG_RUNTIME_DIR").then(|| OsString::from("/run/user/test"));
        Here {
            source: self.source.clone(),
            home: self.home.clone(),
            user: self.user,
            env: agent_env(&self.home, lookup),
        }
    }

    pub fn install_within(
        &self,
        enrol: Duration,
        stop: &AtomicBool,
    ) -> Result<PathBuf, AgentError> {
        let code = EnrolmentCode::parse(CODE).expect("a code");
        let hub = HubOrigin::parse(HUB).expect("an origin");
        let limits = Limits {
            enrol,
            install: WIDE,
            stop,
        };
        install_here(&self.here(), &code, &hub, &limits)
    }

    pub fn install(&self) -> Result<PathBuf, AgentError> {
        self.install_within(WIDE, &NEVER)
    }

    pub fn installed(&self) -> PathBuf {
        self.home.join(".local/bin/anima-agent")
    }

    pub fn runs(&self) -> String {
        fs::read_to_string(&self.record).unwrap_or_default()
    }

    pub fn run_of(&self, args: &str, stdin: &str) -> String {
        format!(
            "program:{}\nargs:{args}\nhome:{}\nxdg:/run/user/test\ncargo:unset\npwd:/\nstdin:{stdin}|\n",
            self.installed().display(),
            self.home.display()
        )
    }

    /// Makes `~/.local/bin` (and `~/.local`) before the install.
    pub fn local_bin(&self) -> PathBuf {
        let bin = self.home.join(".local/bin");
        fs::create_dir_all(&bin).expect("bin");
        bin
    }
}

pub fn names_in(folder: &Path) -> Vec<String> {
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
