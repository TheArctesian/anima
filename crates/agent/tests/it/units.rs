//! The unit files parse and name the binary and `run` (MET-019, MET-020).

use anima_agent::units::{LAUNCH_AGENT, SYSTEMD_SYSTEM, SYSTEMD_USER};

/// The `key=value` lines of a unit with the section they are in.
fn unit(text: &str) -> Vec<(String, String, String)> {
    let mut section = String::new();
    let mut found = Vec::new();
    for line in text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
    {
        if let Some(name) = line
            .strip_prefix('[')
            .and_then(|rest| rest.strip_suffix(']'))
        {
            name.clone_into(&mut section);
        } else {
            let (key, value) = line.split_once('=').expect("every unit line is key=value");
            found.push((section.clone(), key.to_owned(), value.to_owned()));
        }
    }
    found
}

fn value<'a>(lines: &'a [(String, String, String)], key: &str) -> Option<&'a str> {
    lines
        .iter()
        .find(|(_, name, _)| name == key)
        .map(|(_, _, value)| value.as_str())
}

#[test]
fn met_019_the_systemd_units_parse_and_run_the_binary_with_run() {
    for text in [SYSTEMD_USER, SYSTEMD_SYSTEM] {
        let lines = unit(text);
        let exec = value(&lines, "ExecStart").expect("an ExecStart");
        assert!(exec.ends_with("/anima-agent run"), "{exec}");
        assert_eq!(value(&lines, "Type"), Some("simple"));
        assert!(lines.iter().any(|(section, ..)| section == "Install"));
    }
}

#[test]
fn met_019_the_system_unit_is_sandboxed_and_listens_on_nothing() {
    let lines = unit(SYSTEMD_SYSTEM);
    assert_eq!(value(&lines, "NoNewPrivileges"), Some("yes"));
    assert_eq!(value(&lines, "ProtectSystem"), Some("strict"));
    assert_eq!(value(&lines, "PrivateTmp"), Some("yes"));
    assert_eq!(value(&lines, "ProtectHome"), Some("yes"));
    assert_eq!(value(&lines, "User"), Some("anima-agent"));
    assert_eq!(value(&lines, "StateDirectory"), Some("anima-agent"));
    assert_eq!(
        value(&lines, "Environment"),
        Some("ANIMA_AGENT_HOME=/var/lib/anima-agent")
    );
    let families = value(&lines, "RestrictAddressFamilies").expect("restricted");
    for family in families.split_whitespace() {
        assert!(
            ["AF_INET", "AF_INET6", "AF_UNIX"].contains(&family),
            "{family}"
        );
    }
    assert!(
        lines.iter().all(|(_, key, _)| !key.starts_with("Listen")),
        "no socket is listened on"
    );
}

#[test]
fn met_019_the_user_unit_keeps_the_basic_limits_and_the_network_families() {
    let lines = unit(SYSTEMD_USER);
    assert_eq!(value(&lines, "NoNewPrivileges"), Some("yes"));
    assert_eq!(value(&lines, "PrivateTmp"), Some("yes"));
    assert_eq!(
        value(&lines, "RestrictAddressFamilies"),
        Some("AF_INET AF_INET6 AF_UNIX")
    );
    assert_eq!(value(&lines, "WantedBy"), Some("default.target"));
    assert_eq!(value(&lines, "ProtectSystem"), Some("strict"));
    assert_eq!(value(&lines, "ProtectHome"), Some("read-only"));
    assert_eq!(value(&lines, "ReadWritePaths"), Some("%E/anima-agent"));
    assert_eq!(value(&lines, "SocketBindDeny"), Some("any"));
}

#[test]
fn met_019_the_launch_agent_parses_and_runs_the_binary_with_run() {
    let list = plist::Value::from_reader_xml(LAUNCH_AGENT.as_bytes()).expect("a property list");
    let dict = list.as_dictionary().expect("a dictionary");
    assert_eq!(
        dict.get("Label").and_then(plist::Value::as_string),
        Some("ing.anima.agent")
    );
    let arguments: Vec<&str> = dict
        .get("ProgramArguments")
        .and_then(plist::Value::as_array)
        .expect("arguments")
        .iter()
        .filter_map(plist::Value::as_string)
        .collect();
    assert!(arguments[0].ends_with("/anima-agent"));
    assert_eq!(arguments[1..], ["run"]);
    assert_eq!(
        dict.get("KeepAlive").and_then(plist::Value::as_boolean),
        Some(true)
    );
    assert!(
        !dict.contains_key("Sockets"),
        "the agent listens on no port"
    );
}

/// `anima-agent install` against a temporary HOME and a recording stand-in for the commands, so
/// no systemd or launchd is touched (D-10). The system unit needs root and `/etc`, so only its
/// pure parts are tested (`units::plan`, `units::render`).
#[cfg(target_os = "linux")]
mod install_run {
    use std::fs;
    use std::io;
    use std::os::unix::fs::PermissionsExt as _;
    use std::path::{Path, PathBuf};

    use anima_agent::config::home::Home;
    use anima_agent::readers::Platform;
    use anima_agent::token_store::{Credential, TokenStore};
    use anima_agent::units::install::{Host, InstallError, install};
    use anima_agent::units::plan::{Step, Target};
    use chrono::TimeDelta;
    use tempfile::TempDir;

    use crate::kit;

    struct Rig {
        dir: TempDir,
        program: PathBuf,
        home: Home,
    }

    /// A HOME with an enrolled agent (unless `enrolled` is false) and a stand-in binary.
    fn rig(enrolled: bool) -> Rig {
        let dir = tempfile::tempdir().expect("a folder");
        let program = dir.path().join("bin/anima-agent");
        fs::create_dir(dir.path().join("bin")).expect("bin");
        fs::write(&program, b"#!/bin/true\n").expect("binary");
        let home = Home::at(dir.path().join("config/anima-agent"));
        if enrolled {
            let credential = Credential {
                token: kit::token(1),
                expires_at: kit::at(0) + TimeDelta::days(300),
                hub: kit::origin(),
            };
            TokenStore::open(&home)
                .save_new(&credential)
                .expect("stored");
        }
        Rig { dir, program, home }
    }

    impl Rig {
        fn env(&self) -> impl Fn(&str) -> Option<String> + use<> {
            let home = self.dir.path().display().to_string();
            move |name| (name == "HOME").then(|| home.clone())
        }

        fn host(&self) -> Host<'_> {
            Host {
                platform: Platform::Linux,
                uid: 1000,
                program: &self.program,
                home: &self.home,
            }
        }

        fn unit(&self) -> PathBuf {
            self.dir
                .path()
                .join(".config/systemd/user/anima-agent.service")
        }

        /// Runs `install` for the user, returning the commands it ran.
        fn install(&self, succeed: bool) -> (Result<PathBuf, InstallError>, Vec<String>) {
            let mut ran = Vec::new();
            let result = install(Target::User, &self.host(), self.env(), |step: &Step| {
                ran.push(format!("{} {}", step.program, step.args.join(" ")));
                Ok(succeed)
            });
            (result, ran)
        }
    }

    fn read(path: &Path) -> String {
        fs::read_to_string(path).expect("a file")
    }

    #[test]
    fn met_019_install_writes_the_user_unit_with_this_binary_s_path_then_enables_it() {
        let rig = rig(true);
        let (result, ran) = rig.install(true);
        assert_eq!(result.expect("installed"), rig.unit());
        let program = rig.program.canonicalize().expect("path");
        let text = read(&rig.unit());
        assert!(text.contains(&format!("\nExecStart={} run\n", program.display())));
        assert_eq!(text.matches(program.to_str().expect("utf-8")).count(), 1);
        assert_eq!(
            fs::metadata(rig.unit()).expect("meta").permissions().mode() & 0o777,
            0o644
        );
        assert_eq!(
            ran,
            [
                "/usr/bin/systemctl --user daemon-reload",
                "/usr/bin/systemctl --user enable --now anima-agent.service"
            ]
        );
    }

    #[test]
    fn met_019_install_run_twice_replaces_the_unit_whole_and_leaves_no_temporary_file() {
        let rig = rig(true);
        rig.install(true).0.expect("first");
        fs::write(rig.unit(), "stale and longer than the unit\n".repeat(500)).expect("stale");
        let first = {
            let (result, _) = rig.install(true);
            result.expect("second");
            read(&rig.unit())
        };
        rig.install(true).0.expect("third");
        assert_eq!(read(&rig.unit()), first);
        assert!(!first.contains("stale"));
        let names: Vec<_> = fs::read_dir(rig.unit().parent().expect("dir"))
            .expect("dir")
            .map(|entry| entry.expect("entry").file_name())
            .collect();
        assert_eq!(names, ["anima-agent.service"]);
    }

    #[test]
    fn met_019_install_refuses_an_agent_that_is_not_enrolled_and_writes_nothing() {
        let rig = rig(false);
        let (result, ran) = rig.install(true);
        let error = result.expect_err("refused");
        assert!(matches!(error, InstallError::NotEnrolled));
        assert!(error.to_string().contains("anima-agent enrol -"));
        assert!(!rig.unit().exists());
        assert_eq!(ran.len(), 0);
    }

    #[test]
    fn met_019_install_refuses_a_binary_in_a_folder_other_users_can_replace_in() {
        let rig = rig(true);
        let bin = rig.dir.path().join("bin");
        fs::set_permissions(&bin, fs::Permissions::from_mode(0o777)).expect("chmod");
        let (result, ran) = rig.install(true);
        assert!(matches!(
            result.expect_err("refused"),
            InstallError::UntrustedFolder(folder) if folder == bin.canonicalize().expect("path")
        ));
        assert!(!rig.unit().exists());
        assert_eq!(ran.len(), 0);
    }

    #[test]
    fn met_019_install_refuses_a_binary_whose_path_systemd_would_misread() {
        let rig = rig(true);
        let odd = rig.dir.path().join("bin/anima agent%h");
        fs::rename(&rig.program, &odd).expect("rename");
        let host = Host {
            program: &odd,
            ..rig.host()
        };
        let result = install(Target::User, &host, rig.env(), |_| Ok(true));
        assert!(matches!(result, Err(InstallError::Program(_))));
        assert!(!rig.unit().exists());
    }

    #[test]
    fn met_019_a_failing_command_is_an_error_naming_it_and_a_missing_one_is_too() {
        let rig = rig(true);
        let (result, ran) = rig.install(false);
        let error = result.expect_err("failed");
        assert!(matches!(&error, InstallError::Step { .. }));
        assert!(error.to_string().contains("daemon-reload"));
        assert_eq!(ran.len(), 1, "the later command never ran");
        let missing = install(Target::User, &rig.host(), rig.env(), |_| {
            Err(io::Error::from(io::ErrorKind::NotFound))
        });
        assert!(matches!(missing, Err(InstallError::Run { .. })));
    }

    #[test]
    fn met_019_system_install_without_root_is_refused_before_anything_is_touched() {
        let rig = rig(true);
        let mut ran = false;
        let result = install(Target::System, &rig.host(), rig.env(), |_| {
            ran = true;
            Ok(true)
        });
        let error = result.expect_err("refused");
        assert!(error.to_string().contains("sudo"));
        assert!(!ran);
    }
}
