//! The rights of the services of a recipe (SF-NEW-revcatroot): each fault gives its own reason.

use super::{codes, sound, with_compose};
use crate::{Code, Privilege, Recipe, check};

const VARIABLE: &str = "      SECRET: ${NOTES_SECRET}";
const DROP_ALL: &str = "    cap_drop:\n      - ALL\n";
const EXCEPTION_DROP: &str = "    cap_drop:\n      - AUDIT_WRITE\n      - FSETID\n      - KILL\n      - MKNOD\n      - NET_BIND_SERVICE\n      - NET_RAW\n      - SETFCAP\n      - SETPCAP\n      - SYS_CHROOT\n";

/// [`sound`] with one more variable in the environment of its service.
fn with_variable(line: &str) -> Recipe {
    with_compose(VARIABLE, &format!("{VARIABLE}\n      {line}"))
}

/// [`sound`] whose entry names the exception of an init as root.
fn with_exception(recipe: Recipe) -> Recipe {
    Recipe {
        entry: recipe.entry.map(|text| {
            text.replacen(
                r#""marks": []"#,
                r#""marks": [], "exceptions": ["init-capabilities"]"#,
                1,
            )
        }),
        ..recipe
    }
}

fn refused(privilege: Privilege) -> Vec<Code> {
    vec![Code::PrivilegeNotPermitted(privilege)]
}

#[test]
fn sf_revcatroot_a_setting_that_changes_the_code_at_the_start_is_refused() {
    for (line, privilege) in [
        ("DOCKER_MODS: linuxserver/mods", Privilege::ImageMods),
        ("INSTALL_PACKAGES: curl", Privilege::Packages),
        ("INSTALL_PIP_PACKAGES: requests", Privilege::PipPackages),
        ("VERSION: latest", Privilege::VersionNotFixed),
        (
            "VERSION: ${NOTES_VERSION:-docker}",
            Privilege::VersionNotFixed,
        ),
    ] {
        assert_eq!(codes(&with_variable(line)), refused(privilege), "{line}");
    }
    assert_eq!(check(&with_variable("VERSION: docker")), []);
    let found = check(&with_variable("DOCKER_MODS: linuxserver/mods"));
    assert_eq!(
        found[0].to_string(),
        "notes: privilege_not_permitted (image_mods)"
    );
}

#[test]
fn sf_revcatroot_a_service_that_is_root_by_a_setting_is_refused() {
    for line in ["PUID: \"0\"", "PGID: \"0\"", "PUID: ${NOTES_PUID:-1000}"] {
        assert_eq!(
            codes(&with_variable(line)),
            refused(Privilege::RootIdentity),
            "{line}"
        );
    }
    assert_eq!(check(&with_variable("PUID: \"1000\"")), []);
    for user in [
        "\"0\"",
        "root",
        "\"1000:0\"",
        "app:root",
        "${NOTES_USER:-1000}",
    ] {
        let recipe = with_compose(
            "    security_opt:",
            &format!("    user: {user}\n    security_opt:"),
        );
        assert_eq!(codes(&recipe), refused(Privilege::RootUser), "{user}");
    }
    let named = with_compose(
        "    security_opt:",
        "    user: \"1000:1000\"\n    security_opt:",
    );
    assert_eq!(check(&named), []);
}

#[test]
fn sf_revcatroot_each_service_refuses_new_rights_and_drops_each_capability() {
    let no_option = with_compose("    security_opt:\n      - no-new-privileges:true\n", "");
    assert_eq!(codes(&no_option), refused(Privilege::NewPrivileges));
    let no_drop = with_compose(DROP_ALL, "");
    assert_eq!(codes(&no_drop), refused(Privilege::CapabilitiesKept));
    let one_drop = with_compose(DROP_ALL, "    cap_drop:\n      - NET_RAW\n");
    assert_eq!(codes(&one_drop), refused(Privilege::CapabilitiesKept));
}

#[test]
fn sf_revcatroot_an_init_as_root_keeps_exactly_five_capabilities_when_the_entry_names_it() {
    let init = with_compose(DROP_ALL, EXCEPTION_DROP);
    assert_eq!(codes(&init), refused(Privilege::CapabilitiesKept));
    assert_eq!(check(&with_exception(init)), []);
    let keeps_kill = with_compose(DROP_ALL, &EXCEPTION_DROP.replace("      - KILL\n", ""));
    assert_eq!(
        codes(&with_exception(keeps_kill)),
        [
            Code::PrivilegeNotPermitted(Privilege::CapabilitiesKept),
            Code::PrivilegeNotPermitted(Privilege::ExceptionNotNeeded)
        ]
    );
    assert_eq!(
        codes(&with_exception(sound())),
        refused(Privilege::ExceptionNotNeeded)
    );
}
