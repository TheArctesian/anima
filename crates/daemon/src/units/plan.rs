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
pub(super) const ACCOUNT: &str = "anima-daemon";

/// The system unit's state folder, which holds the token (`ANIMA_DAEMON_HOME` in the unit).
pub(super) const SYSTEM_HOME: &str = "/var/lib/anima-daemon";

/// How long `launchctl bootstrap` waits before its one retry.
const BOOTSTRAP_RETRY: Duration = Duration::from_secs(1);

/// What `install` does: write `contents of unit` as `file`, then run `steps` in order. `home` is
/// the daemon folder the started daemon finds: the unit sets no `ANIMA_DAEMON_HOME` for the user.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Plan {
    pub unit: Unit,
    pub file: PathBuf,
    pub home: PathBuf,
    pub steps: Vec<Step>,
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum PlanError {
    #[error("`install --system` is for Linux; on macOS run `anima-daemon install`")]
    SystemOnMacos,
    #[error(
        "`install --system` writes /etc and needs root: run it with sudo (this command never calls sudo itself)"
    )]
    NeedsRoot,
    #[error("no home folder: set HOME")]
    NoHome,
}

/// The name of the program before R-126. `install` takes its unit out, so that one machine
/// never runs the old program beside the new one.
const OLD_UNIT: &str = "anima-agent.service";
const OLD_LABEL: &str = "ing.anima.agent";

/// Stops and disables the unit of the old program and deletes its file. Each step may fail:
/// on a machine that never had it, and when the old unit is slow to stop. Only the unit of
/// the scope of this install is taken out: a user install cannot stop a system unit.
fn retire_old(stop: Step, file: &std::path::Path) -> Vec<Step> {
    let mut stop = stop;
    stop.may_fail = true;
    let mut delete = Step::new("/bin/rm", &["-f"]);
    delete.args.push(file.display().to_string());
    delete.may_fail = true;
    vec![stop, delete]
}

fn retire_old_systemd(user: bool, file: &std::path::Path) -> Vec<Step> {
    let scope: &[&str] = if user { &["--user"] } else { &[] };
    let mut stop = Step::new("/usr/bin/systemctl", scope);
    stop.args
        .extend(["disable", "--now", OLD_UNIT].map(str::to_owned));
    retire_old(stop, file)
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
                step.args.push("anima-daemon.service".to_owned());
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
            file: PathBuf::from("/etc/systemd/system/anima-daemon.service"),
            home: PathBuf::from(SYSTEM_HOME),
            steps: retire_old_systemd(false, &PathBuf::from("/etc/systemd/system").join(OLD_UNIT))
                .into_iter()
                .chain(systemctl(false))
                .collect(),
        }),
        (Target::User, Platform::Linux) => {
            // The user manager reads its units from, and sets `%E` (the unit's
            // `ReadWritePaths`) to, its own config folder: `~/.config` unless its environment
            // says otherwise, which this shell cannot see. This shell's `XDG_CONFIG_HOME` is
            // therefore not followed; a daemon folder found through it is refused later.
            let config = home()?.join(".config");
            Ok(Plan {
                unit: Unit::SystemdUser,
                file: config.join("systemd/user/anima-daemon.service"),
                home: config.join("anima-daemon"),
                steps: retire_old_systemd(true, &config.join("systemd/user").join(OLD_UNIT))
                    .into_iter()
                    .chain(systemctl(true))
                    .collect(),
            })
        }
        (Target::User, Platform::Macos) => {
            let home = home()?;
            let file = home.join("Library/LaunchAgents/ing.anima.daemon.plist");
            let domain = format!("gui/{uid}");
            // `bootstrap` fails on a daemon that is already loaded, so a re-run first takes the
            // old one out; that `bootout` fails, harmlessly, when nothing was loaded.
            let mut old = Step::new("/bin/launchctl", &["bootout"]);
            old.args.push(format!("{domain}/{OLD_LABEL}"));
            let retire = retire_old(
                old,
                &home.join(format!("Library/LaunchAgents/{OLD_LABEL}.plist")),
            );
            let mut bootout = Step::new("/bin/launchctl", &["bootout"]);
            bootout.args.push(format!("{domain}/ing.anima.daemon"));
            bootout.may_fail = true;
            let mut bootstrap = Step::new("/bin/launchctl", &["bootstrap"]);
            bootstrap.args.push(domain);
            bootstrap.args.push(file.display().to_string());
            bootstrap.retry_after = Some(BOOTSTRAP_RETRY);
            Ok(Plan {
                unit: Unit::LaunchAgent,
                file,
                home: home.join("Library/Application Support/anima-daemon"),
                steps: retire.into_iter().chain([bootout, bootstrap]).collect(),
            })
        }
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
            PathBuf::from("/home/ada/.config/systemd/user/anima-daemon.service")
        );
        assert_eq!(plan.home, PathBuf::from("/home/ada/.config/anima-daemon"));
        assert_eq!(
            commands(&plan),
            [
                "/usr/bin/systemctl --user disable --now anima-agent.service",
                "/bin/rm -f /home/ada/.config/systemd/user/anima-agent.service",
                "/usr/bin/systemctl --user daemon-reload",
                "/usr/bin/systemctl --user enable anima-daemon.service",
                "/usr/bin/systemctl --user restart anima-daemon.service",
            ]
        );
    }

    #[test]
    fn met_019_an_upgrade_restarts_the_system_unit_after_enabling_it() {
        let plan = plan(Target::System, Platform::Linux, 0, |_| None).expect("a plan");
        assert_eq!(
            commands(&plan),
            [
                "/usr/bin/systemctl disable --now anima-agent.service",
                "/bin/rm -f /etc/systemd/system/anima-agent.service",
                "/usr/bin/systemctl daemon-reload",
                "/usr/bin/systemctl enable anima-daemon.service",
                "/usr/bin/systemctl restart anima-daemon.service",
            ]
        );
        assert_eq!(plan.home, PathBuf::from(SYSTEM_HOME));
    }

    #[test]
    fn met_019_the_user_unit_and_its_daemon_folder_come_from_one_config_folder() {
        let xdg = |name: &str| match name {
            "XDG_CONFIG_HOME" => Some("/srv/cfg".to_owned()),
            other => env(other),
        };
        let plan = plan(Target::User, Platform::Linux, 1000, xdg).expect("a plan");
        assert_eq!(
            plan.file,
            PathBuf::from("/home/ada/.config/systemd/user/anima-daemon.service")
        );
        assert_eq!(plan.home, PathBuf::from("/home/ada/.config/anima-daemon"));
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
            PathBuf::from("/etc/systemd/system/anima-daemon.service")
        );
        assert!(plan.steps.iter().all(|s| s.program != "sudo"));
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
            PathBuf::from("/home/ada/Library/LaunchAgents/ing.anima.daemon.plist")
        );
        // R-126: the unit of the old program goes first; each of its two steps may fail.
        assert_eq!(plan.steps[0].args, ["bootout", "gui/501/ing.anima.agent"]);
        assert_eq!(
            plan.steps[1].args,
            ["-f", "/home/ada/Library/LaunchAgents/ing.anima.agent.plist"]
        );
        assert!(plan.steps[..2].iter().all(|step| step.may_fail));
        let plan = Plan {
            steps: plan.steps.into_iter().skip(2).collect(),
            ..plan
        };
        assert_eq!(plan.steps.len(), 2);
        assert_eq!(
            plan.home,
            PathBuf::from("/home/ada/Library/Application Support/anima-daemon")
        );
        assert!(plan.steps[0].may_fail);
        assert_eq!(plan.steps[0].retry_after, None);
        assert_eq!(plan.steps[1].retry_after, Some(BOOTSTRAP_RETRY));
        assert_eq!(plan.steps[0].args, ["bootout", "gui/501/ing.anima.daemon"]);
        assert!(!plan.steps[1].may_fail);
        assert_eq!(
            plan.steps[1].args,
            [
                "bootstrap",
                "gui/501",
                "/home/ada/Library/LaunchAgents/ing.anima.daemon.plist"
            ]
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
