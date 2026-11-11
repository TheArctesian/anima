//! The whole install against a stand-in agent: a shell script that records the path it was
//! started as, its arguments, the variables it got, its folder and its standard input. HOME is
//! a temporary folder in the crate's own `target/`, outside `/tmp` (as T6a's rig), so no real
//! home is written.

use std::{
    ffi::OsString,
    fs,
    os::unix::fs::{MetadataExt, PermissionsExt, symlink},
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicBool, Ordering},
    thread,
    time::{Duration, Instant},
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
const WIDE: Duration = Duration::from_secs(30);
static NEVER: AtomicBool = AtomicBool::new(false);

struct Rig {
    dir: TempDir,
    home: PathBuf,
    source: PathBuf,
    record: PathBuf,
    user: User,
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
fn rig(then: &str) -> Rig {
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
    fn here(&self) -> Here {
        let lookup =
            |name: &str| (name == "XDG_RUNTIME_DIR").then(|| OsString::from("/run/user/test"));
        Here {
            source: self.source.clone(),
            home: self.home.clone(),
            user: self.user,
            env: agent_env(&self.home, lookup),
        }
    }

    fn install_within(&self, enrol: Duration, stop: &AtomicBool) -> Result<PathBuf, AgentError> {
        let code = EnrolmentCode::parse(CODE).expect("a code");
        let hub = HubOrigin::parse(HUB).expect("an origin");
        let limits = Limits {
            enrol,
            install: WIDE,
            stop,
        };
        install_here(&self.here(), &code, &hub, &limits)
    }

    fn install(&self) -> Result<PathBuf, AgentError> {
        self.install_within(WIDE, &NEVER)
    }

    fn installed(&self) -> PathBuf {
        self.home.join(".local/bin/anima-agent")
    }

    fn runs(&self) -> String {
        fs::read_to_string(&self.record).unwrap_or_default()
    }

    fn run_of(&self, args: &str, stdin: &str) -> String {
        format!(
            "program:{}\nargs:{args}\nhome:{}\nxdg:/run/user/test\ncargo:unset\npwd:/\nstdin:{stdin}|\n",
            self.installed().display(),
            self.home.display()
        )
    }

    /// Makes `~/.local/bin` (and `~/.local`) before the install.
    fn local_bin(&self) -> PathBuf {
        let bin = self.home.join(".local/bin");
        fs::create_dir_all(&bin).expect("bin");
        bin
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
    assert_eq!(rig.install(), Ok(rig.installed()));
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
fn met_019_the_agent_runs_from_the_real_path_that_was_checked() {
    let rig = rig("exit 0");
    let real = rig.dir.path().join("real-bin");
    fs::create_dir(&real).expect("the real folder");
    fs::create_dir(rig.home.join(".local")).expect(".local");
    symlink(&real, rig.home.join(".local/bin")).expect("a link");
    assert_eq!(rig.install(), Ok(real.join("anima-agent")));
    let program = format!("program:{}\n", real.join("anima-agent").display());
    assert_eq!(rig.runs().matches(&program).count(), 2, "{}", rig.runs());
}

#[test]
fn met_019_a_failed_enrolment_stops_before_install() {
    let rig = rig("[ \"$1\" = enrol ] && exit 3; exit 0");
    assert_eq!(rig.install(), Err(AgentError::Enrol));
    assert!(!rig.runs().contains("args:install"), "{}", rig.runs());
}

#[test]
fn met_019_a_failed_install_after_enrolment_is_its_own_error() {
    let rig = rig("[ \"$1\" = install ] && exit 1; exit 0");
    assert_eq!(rig.install(), Err(AgentError::Install));
    assert!(rig.runs().contains("args:enrol"), "{}", rig.runs());
}

/// A stand-in that writes its pid to `pid` and then sleeps.
fn slow_rig() -> (Rig, PathBuf) {
    let rig = rig("");
    let pid_file = rig.dir.path().join("pid");
    let script = format!(
        "{}\necho $$ > '{}'; exec /bin/sleep 30\n",
        fs::read_to_string(&rig.source).expect("the stand-in"),
        pid_file.display()
    );
    fs::write(&rig.source, script).expect("the slow stand-in");
    (rig, pid_file)
}

/// Whether the process in `pid_file` is gone; one only killed would still be a zombie.
fn reaped(pid_file: &Path) -> bool {
    let pid = fs::read_to_string(pid_file).expect("the stand-in wrote its pid");
    let listed = Command::new("/bin/ps")
        .args(["-o", "stat=", "-p", pid.trim()])
        .output()
        .expect("ps");
    listed.stdout.is_empty()
}

#[test]
fn met_019_an_agent_past_its_deadline_is_killed_and_reaped() {
    let (rig, pid_file) = slow_rig();
    let started = Instant::now();
    let answer = rig.install_within(Duration::from_secs(2), &NEVER);
    assert_eq!(answer, Err(AgentError::TimedOut));
    assert!(started.elapsed() < Duration::from_secs(10));
    assert!(reaped(&pid_file), "the agent is still there");
    assert!(!rig.runs().contains("args:install"));
}

#[test]
fn met_019_an_agent_running_when_the_app_exits_is_killed_and_reaped() {
    let (rig, pid_file) = slow_rig();
    let stop = AtomicBool::new(false);
    let started = Instant::now();
    let answer = thread::scope(|scope| {
        scope.spawn(|| {
            while !pid_file.exists() && started.elapsed() < Duration::from_secs(10) {
                thread::sleep(Duration::from_millis(10));
            }
            stop.store(true, Ordering::Release);
        });
        rig.install_within(WIDE, &stop)
    });
    assert_eq!(answer, Err(AgentError::Failed));
    assert!(
        started.elapsed() < Duration::from_secs(15),
        "the stop was not seen"
    );
    assert!(reaped(&pid_file), "the agent is still there");
}

#[test]
fn met_019_a_folder_others_can_write_is_refused_before_anything_runs() {
    for open in [".local", ".local/bin", ""] {
        let rig = rig("exit 0");
        let folder = rig.home.join(open);
        fs::create_dir_all(&folder).expect("the folder");
        fs::set_permissions(&folder, fs::Permissions::from_mode(0o777)).expect("chmod");
        assert_eq!(rig.install(), Err(AgentError::Untrusted), "{open}");
        assert!(!rig.installed().exists(), "{open}");
        assert_eq!(rig.runs(), "", "{open}");
    }
}

#[test]
fn met_019_a_folder_a_shared_group_can_write_is_refused_and_the_private_group_is_not() {
    let rig = rig("exit 0");
    let bin = rig.local_bin();
    fs::set_permissions(&bin, fs::Permissions::from_mode(0o775)).expect("chmod");
    assert_eq!(
        rig.install(),
        Ok(rig.installed()),
        "the user's private group"
    );
    // The same folder when its group is not the user's private one (`chgrp adm`, 0775).
    let shared = Rig {
        user: User {
            private_group: None,
            ..rig.user
        },
        ..rig
    };
    fs::remove_file(&shared.record).expect("the record");
    assert_eq!(shared.install(), Err(AgentError::Untrusted));
    assert_eq!(shared.runs(), "");
}

/// `setfacl -m u:nobody:rwx` on `folder`, written as the kernel's ACL attribute (version 2, then
/// tag, permissions and ID per entry), so no `setfacl` is needed.
#[cfg(target_os = "linux")]
fn grant_nobody_write(folder: &Path) {
    let mut acl = 2_u32.to_le_bytes().to_vec();
    let undefined = u32::MAX;
    for (tag, perm, id) in [
        (0x01_u16, 7_u16, undefined),
        (0x02, 7, 65_534),
        (0x04, 5, undefined),
        (0x10, 7, undefined),
        (0x20, 5, undefined),
    ] {
        acl.extend_from_slice(&tag.to_le_bytes());
        acl.extend_from_slice(&perm.to_le_bytes());
        acl.extend_from_slice(&id.to_le_bytes());
    }
    rustix::fs::setxattr(
        folder,
        "system.posix_acl_access",
        &acl,
        rustix::fs::XattrFlags::empty(),
    )
    .expect("the ACL (the file system must support POSIX ACLs)");
}

#[cfg(target_os = "linux")]
#[test]
fn met_019_a_folder_another_user_may_write_through_an_acl_is_refused() {
    let rig = rig("exit 0");
    let bin = rig.local_bin();
    grant_nobody_write(&bin);
    assert_eq!(
        fs::metadata(&bin).expect("bin").mode() & 0o070,
        0o070,
        "mask rwx"
    );
    assert_eq!(rig.install(), Err(AgentError::Untrusted));
    assert_eq!(rig.runs(), "", "the code reached the agent");
}

#[test]
fn met_019_a_missing_or_linked_agent_is_not_bundled() {
    let rig = rig("exit 0");
    let moved = rig.dir.path().join("elsewhere");
    fs::rename(&rig.source, &moved).expect("moved");
    assert_eq!(rig.install(), Err(AgentError::NotBundled));
    symlink(&moved, &rig.source).expect("a link");
    assert_eq!(rig.install(), Err(AgentError::NotBundled));
    assert_eq!(rig.runs(), "");
}

#[test]
fn met_019_a_link_at_the_destination_is_replaced_not_followed() {
    let rig = rig("exit 0");
    let victim = rig.dir.path().join("victim");
    fs::write(&victim, b"keep").expect("the victim");
    rig.local_bin();
    symlink(&victim, rig.installed()).expect("a link");
    assert_eq!(rig.install(), Ok(rig.installed()));
    assert_eq!(fs::read(&victim).ok(), Some(b"keep".to_vec()));
    let installed = fs::symlink_metadata(rig.installed()).expect("installed");
    assert!(installed.is_file(), "the link was followed");
}
