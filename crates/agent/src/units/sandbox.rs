//! What the units' sandboxes hide from the agent (MET-019): a binary under a hidden folder never
//! starts, so `install` refuses it. Read from the unit templates' `ProtectHome=yes` and
//! `PrivateTmp=yes`; the test holds the two together.

use std::path::{Path, PathBuf};

use super::render::Unit;

/// The folders `unit`'s sandbox hides: `ProtectHome=yes` hides `/home`, `/root` and
/// `/run/user`; `PrivateTmp=yes` gives the agent its own `/tmp` and `/var/tmp`. A
/// `LaunchAgent` has no sandbox.
pub(super) const fn hidden_roots(unit: Unit) -> &'static [&'static str] {
    match unit {
        Unit::SystemdSystem => &["/home", "/root", "/run/user", "/tmp", "/var/tmp"],
        Unit::SystemdUser => &["/tmp", "/var/tmp"],
        Unit::LaunchAgent => &[],
    }
}

/// The hidden folder `program` (a real path) is under, if any. `roots` pairs each of
/// [`hidden_roots`] with its real path (`None` when it does not exist), since a root may be a
/// link: on Fedora Atomic `/home` is `/var/home`.
pub(super) fn hidden_root(
    roots: &[(&'static str, Option<PathBuf>)],
    program: &Path,
) -> Option<&'static str> {
    roots
        .iter()
        .find(|(root, real)| {
            program.starts_with(root) || real.as_ref().is_some_and(|real| program.starts_with(real))
        })
        .map(|(root, _)| *root)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::{LAUNCH_AGENT, SYSTEMD_SYSTEM, SYSTEMD_USER};

    fn plain(unit: Unit) -> Vec<(&'static str, Option<PathBuf>)> {
        hidden_roots(unit)
            .iter()
            .map(|root| (*root, None))
            .collect()
    }

    #[test]
    fn met_019_the_hidden_folders_are_the_ones_the_unit_templates_hide() {
        for (unit, template) in [
            (Unit::SystemdSystem, SYSTEMD_SYSTEM),
            (Unit::SystemdUser, SYSTEMD_USER),
            (Unit::LaunchAgent, LAUNCH_AGENT),
        ] {
            let set = |line: &str| template.lines().any(|l| l.trim() == line);
            let mut expected = Vec::new();
            if set("ProtectHome=yes") {
                expected.extend(["/home", "/root", "/run/user"]);
            }
            if set("PrivateTmp=yes") {
                expected.extend(["/tmp", "/var/tmp"]);
            }
            assert_eq!(hidden_roots(unit), expected, "{unit:?}");
        }
    }

    #[test]
    fn met_019_a_binary_the_unit_s_sandbox_hides_is_found() {
        let system = plain(Unit::SystemdSystem);
        let at = |roots: &[_], path: &str| hidden_root(roots, Path::new(path));
        assert_eq!(
            at(&system, "/home/ada/.local/bin/anima-agent"),
            Some("/home")
        );
        assert_eq!(at(&system, "/root/anima-agent"), Some("/root"));
        assert_eq!(at(&system, "/run/user/1000/anima-agent"), Some("/run/user"));
        assert_eq!(at(&system, "/usr/local/bin/anima-agent"), None);
        assert_eq!(at(&system, "/homestead/anima-agent"), None, "whole parts");
        for unit in [Unit::SystemdSystem, Unit::SystemdUser] {
            assert_eq!(at(&plain(unit), "/tmp/x/anima-agent"), Some("/tmp"));
            assert_eq!(at(&plain(unit), "/var/tmp/anima-agent"), Some("/var/tmp"));
        }
        let user = plain(Unit::SystemdUser);
        assert_eq!(at(&user, "/home/ada/.local/bin/anima-agent"), None);
        assert_eq!(at(&plain(Unit::LaunchAgent), "/tmp/anima-agent"), None);
    }

    #[test]
    fn met_019_a_hidden_folder_that_is_a_link_is_found_by_its_real_path_too() {
        let mut atomic = plain(Unit::SystemdSystem);
        atomic[0].1 = Some(PathBuf::from("/var/home"));
        let program = Path::new("/var/home/ada/bin/anima-agent");
        assert_eq!(hidden_root(&atomic, program), Some("/home"));
        assert_eq!(hidden_root(&plain(Unit::SystemdSystem), program), None);
    }
}
