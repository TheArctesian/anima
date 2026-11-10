//! Decides which file the unit goes to and which commands start it (MET-019, D-10): pure, from
//! the platform, the user and the environment.

use std::path::PathBuf;

use super::render::Unit;
use crate::readers::Platform;

/// Whose unit: the operator's own, or the whole computer's (Linux, needs root).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    User,
    System,
}

/// The account the system unit runs as; the operator makes it (the unit says `User=` this).
pub const ACCOUNT: &str = "anima-agent";

/// The system unit's state folder, which holds the token (`ANIMA_AGENT_HOME` in the unit).
pub const SYSTEM_HOME: &str = "/var/lib/anima-agent";

/// One fixed command to run after the file is written.
#[derive(Debug, PartialEq, Eq)]
pub struct Step {
    pub program: &'static str,
    pub args: Vec<String>,
    /// A failure is expected and fine (`launchctl bootout` of an agent that is not loaded).
    pub may_fail: bool,
}

/// What `install` does: write `contents of unit` as `file`, then run `steps` in order.
#[derive(Debug, PartialEq, Eq)]
pub struct Plan {
    pub unit: Unit,
    pub file: PathBuf,
    pub steps: Vec<Step>,
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum PlanError {
    #[error("`install --system` is for Linux; on macOS run `anima-agent install`")]
    SystemOnMacos,
    #[error(
        "`install --system` writes /etc and needs root: run it with sudo (this command never calls sudo itself)"
    )]
    NeedsRoot,
    #[error("no home folder: set HOME")]
    NoHome,
}

fn step(program: &'static str, args: &[&str]) -> Step {
    Step {
        program,
        args: args.iter().map(|arg| (*arg).to_owned()).collect(),
        may_fail: false,
    }
}

/// The plan for `target` on `platform`, run by user `uid`; `env` looks up environment variables.
///
/// # Errors
/// [`PlanError`] for `--system` on macOS or without root, and for no `HOME`.
pub fn plan(
    target: Target,
    platform: Platform,
    uid: u32,
    env: impl Fn(&str) -> Option<String>,
) -> Result<Plan, PlanError> {
    let home = || {
        env("HOME")
            .filter(|home| !home.is_empty())
            .map(PathBuf::from)
            .ok_or(PlanError::NoHome)
    };
    match (target, platform) {
        (Target::System, Platform::Macos) => Err(PlanError::SystemOnMacos),
        (Target::System, Platform::Linux) if uid != 0 => Err(PlanError::NeedsRoot),
        (Target::System, Platform::Linux) => Ok(Plan {
            unit: Unit::SystemdSystem,
            file: PathBuf::from("/etc/systemd/system/anima-agent.service"),
            steps: vec![
                step("/usr/bin/systemctl", &["daemon-reload"]),
                step(
                    "/usr/bin/systemctl",
                    &["enable", "--now", "anima-agent.service"],
                ),
            ],
        }),
        (Target::User, Platform::Linux) => {
            let config = match env("XDG_CONFIG_HOME").filter(|dir| !dir.is_empty()) {
                Some(dir) => PathBuf::from(dir),
                None => home()?.join(".config"),
            };
            Ok(Plan {
                unit: Unit::SystemdUser,
                file: config.join("systemd/user/anima-agent.service"),
                steps: vec![
                    step("/usr/bin/systemctl", &["--user", "daemon-reload"]),
                    step(
                        "/usr/bin/systemctl",
                        &["--user", "enable", "--now", "anima-agent.service"],
                    ),
                ],
            })
        }
        (Target::User, Platform::Macos) => {
            let file = home()?.join("Library/LaunchAgents/ing.anima.agent.plist");
            let domain = format!("gui/{uid}");
            // `bootstrap` fails on an agent that is already loaded, so a re-run first takes the
            // old one out; that `bootout` fails, harmlessly, when nothing was loaded.
            let mut bootout = step("/bin/launchctl", &["bootout"]);
            bootout.args.push(format!("{domain}/ing.anima.agent"));
            bootout.may_fail = true;
            let mut bootstrap = step("/bin/launchctl", &["bootstrap"]);
            bootstrap.args.push(domain);
            bootstrap.args.push(file.display().to_string());
            Ok(Plan {
                unit: Unit::LaunchAgent,
                file,
                steps: vec![bootout, bootstrap],
            })
        }
    }
}

/// Whether a folder on the binary's path (`mode`, owned by `owner`) may stay there: not
/// writable by others without the sticky bit, and for the system unit (which runs the binary
/// as another account) owned by root and not writable by its group either.
#[must_use]
pub const fn folder_trusted(target: Target, mode: u32, owner: u32) -> bool {
    if crate::config::private_file::parent_replaceable(mode) {
        return false;
    }
    match target {
        Target::User => true,
        Target::System => owner == 0 && mode & 0o020 == 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(name: &str) -> Option<String> {
        match name {
            "HOME" => Some("/home/ada".to_owned()),
            _ => None,
        }
    }

    #[test]
    fn met_019_a_linux_user_unit_goes_under_the_config_folder_and_is_enabled_now() {
        let plan = plan(Target::User, Platform::Linux, 1000, env).expect("a plan");
        assert_eq!(plan.unit, Unit::SystemdUser);
        assert_eq!(
            plan.file,
            PathBuf::from("/home/ada/.config/systemd/user/anima-agent.service")
        );
        let commands: Vec<_> = plan
            .steps
            .iter()
            .map(|s| (s.program, s.args.join(" ")))
            .collect();
        assert_eq!(
            commands,
            [
                ("/usr/bin/systemctl", "--user daemon-reload".to_owned()),
                (
                    "/usr/bin/systemctl",
                    "--user enable --now anima-agent.service".to_owned()
                ),
            ]
        );
    }

    #[test]
    fn met_019_the_user_unit_follows_xdg_config_home_as_the_agent_s_own_folder_does() {
        let xdg = |name: &str| match name {
            "XDG_CONFIG_HOME" => Some("/srv/cfg".to_owned()),
            other => env(other),
        };
        let plan = plan(Target::User, Platform::Linux, 1000, xdg).expect("a plan");
        assert_eq!(
            plan.file,
            PathBuf::from("/srv/cfg/systemd/user/anima-agent.service")
        );
    }

    #[test]
    fn met_019_the_system_unit_needs_root_and_never_runs_sudo() {
        assert_eq!(
            plan(Target::System, Platform::Linux, 1000, env),
            Err(PlanError::NeedsRoot)
        );
        assert!(PlanError::NeedsRoot.to_string().contains("sudo"));
        let plan = plan(Target::System, Platform::Linux, 0, |_| None).expect("a plan");
        assert_eq!(plan.unit, Unit::SystemdSystem);
        assert_eq!(
            plan.file,
            PathBuf::from("/etc/systemd/system/anima-agent.service")
        );
        assert!(plan.steps.iter().all(|s| s.program == "/usr/bin/systemctl"));
        assert!(
            plan.steps
                .iter()
                .all(|s| !s.args.contains(&"--user".to_owned()))
        );
    }

    #[test]
    fn met_019_system_is_refused_on_macos_even_for_root() {
        assert_eq!(
            plan(Target::System, Platform::Macos, 0, env),
            Err(PlanError::SystemOnMacos)
        );
    }

    #[test]
    fn met_019_a_macos_launch_agent_is_reloaded_then_bootstrapped_in_the_users_domain() {
        let plan = plan(Target::User, Platform::Macos, 501, env).expect("a plan");
        assert_eq!(plan.unit, Unit::LaunchAgent);
        assert_eq!(
            plan.file,
            PathBuf::from("/home/ada/Library/LaunchAgents/ing.anima.agent.plist")
        );
        assert_eq!(plan.steps.len(), 2);
        assert!(plan.steps[0].may_fail);
        assert_eq!(plan.steps[0].args, ["bootout", "gui/501/ing.anima.agent"]);
        assert!(!plan.steps[1].may_fail);
        assert_eq!(
            plan.steps[1].args,
            [
                "bootstrap",
                "gui/501",
                "/home/ada/Library/LaunchAgents/ing.anima.agent.plist"
            ]
        );
    }

    #[test]
    fn met_019_a_folder_others_can_replace_in_is_not_trusted_and_the_system_unit_wants_root_s() {
        assert!(folder_trusted(Target::User, 0o040_755, 1000));
        assert!(
            folder_trusted(Target::User, 0o041_777, 0),
            "sticky, like /tmp"
        );
        assert!(!folder_trusted(Target::User, 0o040_777, 0));
        assert!(!folder_trusted(Target::User, 0o040_702, 1000));
        assert!(folder_trusted(Target::System, 0o040_755, 0));
        assert!(
            !folder_trusted(Target::System, 0o040_755, 1000),
            "not root's"
        );
        assert!(
            !folder_trusted(Target::System, 0o040_775, 0),
            "group may write"
        );
        assert!(
            !folder_trusted(Target::System, 0o041_777, 0),
            "sticky is not enough"
        );
    }

    #[test]
    fn met_019_a_user_install_without_home_is_an_error() {
        assert_eq!(
            plan(Target::User, Platform::Linux, 1000, |_| None),
            Err(PlanError::NoHome)
        );
    }
}
