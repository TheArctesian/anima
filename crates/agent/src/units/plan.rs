//! Decides which file the unit goes to and which commands start it (MET-019, D-10): pure, from
//! the platform, the user and the environment.

use std::path::PathBuf;
use std::time::Duration;

use super::render::Unit;
use super::step::Step;
use crate::readers::Platform;

/// Whose unit: the operator's own, or the whole computer's (Linux, needs root).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    User,
    System,
}

/// The account the system unit runs as; the operator makes it (the unit says `User=` this).
pub(super) const ACCOUNT: &str = "anima-agent";

/// The system unit's state folder, which holds the token (`ANIMA_AGENT_HOME` in the unit).
pub(super) const SYSTEM_HOME: &str = "/var/lib/anima-agent";

/// How long `launchctl bootstrap` waits before its one retry.
const BOOTSTRAP_RETRY: Duration = Duration::from_secs(1);

/// What `install` does: write `contents of unit` as `file`, then run `steps` in order. `home` is
/// the agent folder the started agent finds: the unit sets no `ANIMA_AGENT_HOME` for the user.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Plan {
    pub unit: Unit,
    pub file: PathBuf,
    pub home: PathBuf,
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

/// `systemctl [--user]` reload, enable, then restart: `restart` starts a stopped unit and
/// restarts a running one, so an upgrade (a new binary, then `install`) runs the new binary.
fn systemctl(user: bool) -> Vec<Step> {
    let scope: &[&str] = if user { &["--user"] } else { &[] };
    ["daemon-reload", "enable", "restart"]
        .iter()
        .map(|verb| {
            let mut step = Step::new("/usr/bin/systemctl", scope);
            step.args.push((*verb).to_owned());
            if *verb != "daemon-reload" {
                step.args.push("anima-agent.service".to_owned());
            }
            step
        })
        .collect()
}

/// The plan for `target` on `platform`, run by user `uid`; `env` looks up environment variables.
///
/// # Errors
/// [`PlanError`] for `--system` on macOS or without root, and for no `HOME`.
pub(super) fn plan(
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
            home: PathBuf::from(SYSTEM_HOME),
            steps: systemctl(false),
        }),
        (Target::User, Platform::Linux) => {
            let config = match env("XDG_CONFIG_HOME").filter(|dir| !dir.is_empty()) {
                Some(dir) => PathBuf::from(dir),
                None => home()?.join(".config"),
            };
            Ok(Plan {
                unit: Unit::SystemdUser,
                file: config.join("systemd/user/anima-agent.service"),
                // The user manager's `%E` (the unit's `ReadWritePaths`) is `~/.config`.
                home: home()?.join(".config/anima-agent"),
                steps: systemctl(true),
            })
        }
        (Target::User, Platform::Macos) => {
            let home = home()?;
            let file = home.join("Library/LaunchAgents/ing.anima.agent.plist");
            let domain = format!("gui/{uid}");
            // `bootstrap` fails on an agent that is already loaded, so a re-run first takes the
            // old one out; that `bootout` fails, harmlessly, when nothing was loaded.
            let mut bootout = Step::new("/bin/launchctl", &["bootout"]);
            bootout.args.push(format!("{domain}/ing.anima.agent"));
            bootout.may_fail = true;
            let mut bootstrap = Step::new("/bin/launchctl", &["bootstrap"]);
            bootstrap.args.push(domain);
            bootstrap.args.push(file.display().to_string());
            bootstrap.retry_after = Some(BOOTSTRAP_RETRY);
            Ok(Plan {
                unit: Unit::LaunchAgent,
                file,
                home: home.join("Library/Application Support/anima-agent"),
                steps: vec![bootout, bootstrap],
            })
        }
    }
}

/// Whether a folder on the binary's or the unit's path (`mode`, owned by `owner`) may stay
/// there when user `uid` installs. For the user unit: owned by root or the user, and either
/// sticky or not writable by others; its group may write only to the user's own folder (a
/// private group per user, as with `umask 002`). For the system unit, which runs the binary as
/// another account: owned by root and writable by no one else, sticky or not.
#[must_use]
pub(super) const fn folder_trusted(target: Target, mode: u32, owner: u32, uid: u32) -> bool {
    match target {
        Target::User => {
            let own = owner == uid;
            let sticky = mode & 0o1000 != 0;
            (own || owner == 0) && (sticky || (mode & 0o002 == 0 && (own || mode & 0o020 == 0)))
        }
        Target::System => owner == 0 && mode & 0o022 == 0,
    }
}

/// Whether the binary itself (`mode`, owned by `owner`) may be started by the unit: writable
/// by no one but its owner, who is root or, for the user unit, user `uid`.
#[must_use]
pub(super) const fn binary_trusted(target: Target, mode: u32, owner: u32, uid: u32) -> bool {
    let owner_trusted = match target {
        Target::User => owner == 0 || owner == uid,
        Target::System => owner == 0,
    };
    owner_trusted && mode & 0o022 == 0
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

    fn commands(plan: &Plan) -> Vec<String> {
        plan.steps
            .iter()
            .map(|s| format!("{} {}", s.program, s.args.join(" ")))
            .collect()
    }

    #[test]
    fn met_019_a_linux_user_unit_goes_under_the_config_folder_and_is_enabled_then_restarted() {
        let plan = plan(Target::User, Platform::Linux, 1000, env).expect("a plan");
        assert_eq!(plan.unit, Unit::SystemdUser);
        assert_eq!(
            plan.file,
            PathBuf::from("/home/ada/.config/systemd/user/anima-agent.service")
        );
        assert_eq!(plan.home, PathBuf::from("/home/ada/.config/anima-agent"));
        assert_eq!(
            commands(&plan),
            [
                "/usr/bin/systemctl --user daemon-reload",
                "/usr/bin/systemctl --user enable anima-agent.service",
                "/usr/bin/systemctl --user restart anima-agent.service",
            ]
        );
    }

    #[test]
    fn met_019_an_upgrade_restarts_the_system_unit_after_enabling_it() {
        let plan = plan(Target::System, Platform::Linux, 0, |_| None).expect("a plan");
        assert_eq!(
            commands(&plan),
            [
                "/usr/bin/systemctl daemon-reload",
                "/usr/bin/systemctl enable anima-agent.service",
                "/usr/bin/systemctl restart anima-agent.service",
            ]
        );
        assert_eq!(plan.home, PathBuf::from(SYSTEM_HOME));
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
        assert_eq!(
            plan.home,
            PathBuf::from("/home/ada/Library/Application Support/anima-agent")
        );
        assert!(plan.steps[0].may_fail);
        assert_eq!(plan.steps[0].retry_after, None);
        assert_eq!(plan.steps[1].retry_after, Some(BOOTSTRAP_RETRY));
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
        assert!(folder_trusted(Target::User, 0o040_755, 1000, 1000));
        assert!(
            folder_trusted(Target::User, 0o041_777, 0, 1000),
            "sticky, like /tmp"
        );
        assert!(!folder_trusted(Target::User, 0o040_777, 0, 1000));
        assert!(!folder_trusted(Target::User, 0o040_702, 1000, 1000));
        assert!(folder_trusted(Target::System, 0o040_755, 0, 0));
        assert!(
            !folder_trusted(Target::System, 0o040_755, 1000, 0),
            "not root's"
        );
        assert!(
            !folder_trusted(Target::System, 0o040_775, 0, 0),
            "group may write"
        );
        assert!(
            !folder_trusted(Target::System, 0o041_777, 0, 0),
            "sticky is not enough"
        );
    }

    #[test]
    fn met_019_a_user_folder_must_be_root_s_or_the_user_s_and_only_the_user_s_may_be_group_writable()
     {
        assert!(
            !folder_trusted(Target::User, 0o040_755, 1001, 1000),
            "another user's folder"
        );
        assert!(
            !folder_trusted(Target::User, 0o041_777, 1001, 1000),
            "another user's sticky folder: its owner may still rename what is in it"
        );
        assert!(
            folder_trusted(Target::User, 0o040_775, 1000, 1000),
            "umask 002 with a private group, like ~/.local/bin"
        );
        assert!(
            !folder_trusted(Target::User, 0o042_775, 0, 1000),
            "root's folder that its group may write"
        );
        assert!(folder_trusted(Target::User, 0o040_755, 0, 1000));
    }

    #[test]
    fn met_019_the_binary_is_writable_only_by_its_owner_who_is_root_or_the_user() {
        assert!(binary_trusted(Target::User, 0o100_755, 1000, 1000));
        assert!(binary_trusted(Target::User, 0o100_755, 0, 1000));
        assert!(!binary_trusted(Target::User, 0o100_777, 1000, 1000));
        assert!(!binary_trusted(Target::User, 0o100_775, 1000, 1000));
        assert!(!binary_trusted(Target::User, 0o100_755, 1001, 1000));
        assert!(binary_trusted(Target::System, 0o100_755, 0, 0));
        assert!(!binary_trusted(Target::System, 0o100_755, 1000, 0));
    }

    #[test]
    fn met_019_a_user_install_without_home_is_an_error() {
        assert_eq!(
            plan(Target::User, Platform::Linux, 1000, |_| None),
            Err(PlanError::NoHome)
        );
    }
}
