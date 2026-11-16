use std::{fs, process::Command};

use super::rig::Rig;
use crate::stay_awake::{
    lid_set_up::{self, Lid, SetUpError},
    platform::Os,
};

#[test]
fn a_mac_can_hold_the_lid_only_after_the_set_up_step() {
    assert_eq!(
        lid_set_up::lid(&Rig::new(Os::Mac).platform),
        Lid::NeedsSetUp
    );
    assert_eq!(lid_set_up::lid(&Rig::mac_set_up().platform), Lid::Ready);
}

#[test]
fn linux_needs_no_set_up_except_on_a_desktop_that_decides_the_lid_itself() {
    let mut rig = Rig::new(Os::Linux);
    for (desktop, lid) in [
        ("GNOME", Lid::Ready),
        ("ubuntu:GNOME", Lid::Ready),
        ("", Lid::Ready),
        ("KDE", Lid::Unsupported),
        ("plasma:kde", Lid::Unsupported),
    ] {
        rig.platform.desktop = desktop.into();
        assert_eq!(lid_set_up::lid(&rig.platform), lid, "{desktop}");
    }
}

#[test]
fn the_rule_names_the_user_by_number_and_only_the_two_pmset_commands() {
    assert_eq!(
        lid_set_up::rule(501),
        "#501 ALL=(root) NOPASSWD: /usr/bin/pmset -a disablesleep 0, /usr/bin/pmset -a disablesleep 1"
    );
}

/// `visudo -c` reads a file and changes nothing; it needs no rights.
#[cfg(target_os = "macos")]
#[test]
fn the_rule_is_one_that_visudo_accepts() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("rule");
    fs::write(&file, format!("{}\n", lid_set_up::rule(501))).unwrap();
    let checked = Command::new("/usr/sbin/visudo")
        .arg("-cf")
        .arg(&file)
        .output()
        .unwrap();
    assert!(checked.status.success(), "{checked:?}");
}

#[test]
fn set_up_asks_macos_for_an_administrator_with_a_script_that_checks_then_installs_the_rule() {
    let rig = Rig::new(Os::Mac);
    lid_set_up::set_up(&rig.platform).unwrap();
    let [call] = &rig.calls()[..] else {
        panic!("one run of osascript, got {:?}", rig.calls())
    };
    assert!(call.starts_with("osascript -e on run argv -e do shell script (item 1 of argv) with prompt (item 2 of argv) with administrator privileges -e end run "));
    let rule = lid_set_up::rule(501);
    let target = rig.platform.rule.display();
    let check_then_install = format!(
        "/bin/echo '{rule}' > \"$t\" && /usr/sbin/visudo -cf \"$t\" > /dev/null && /usr/bin/install -o root -g wheel -m 0440 \"$t\" '{target}'"
    );
    assert!(call.contains(&check_then_install), "{call}");
}

#[test]
fn the_set_up_script_is_a_whole_shell_script() {
    let rig = Rig::new(Os::Mac);
    let seen = rig.dir.path().join("script");
    let mut platform = rig.platform.clone();
    // The stand-in keeps the script (its seventh argument) as it was given.
    platform.osascript = rig.stand_in(
        "osascript",
        &format!("printf '%s' \"$7\" > '{}'", seen.display()),
    );
    lid_set_up::set_up(&platform).unwrap();
    let parsed = Command::new("/bin/sh")
        .arg("-n")
        .arg(&seen)
        .status()
        .unwrap();
    assert!(parsed.success());
}

#[test]
fn remove_deletes_only_the_rule_file() {
    let rig = Rig::mac_set_up();
    lid_set_up::remove(&rig.platform, false).unwrap();
    let call = rig.calls().pop().unwrap();
    let delete = format!("/bin/rm -f '{}'", rig.platform.rule.display());
    assert!(call.contains(&format!(" end run {delete} ")), "{call}");
}

#[test]
fn remove_sets_sleep_back_as_root_when_anima_could_not() {
    let rig = Rig::mac_set_up();
    lid_set_up::remove(&rig.platform, true).unwrap();
    let call = rig.calls().pop().unwrap();
    let delete = format!("/bin/rm -f '{}'", rig.platform.rule.display());
    let both = format!(" end run /usr/bin/pmset -a disablesleep 0; {delete} ");
    assert!(call.contains(&both), "{call}");
}

#[test]
fn a_cancelled_dialog_is_refused_and_linux_has_no_set_up_step() {
    let mut rig = Rig::new(Os::Mac);
    rig.platform.osascript = rig.stand_in("osascript", "exit 1");
    assert_eq!(lid_set_up::set_up(&rig.platform), Err(SetUpError::Refused));
    let linux = Rig::new(Os::Linux);
    assert_eq!(
        lid_set_up::set_up(&linux.platform),
        Err(SetUpError::Unsupported)
    );
    assert_eq!(
        lid_set_up::remove(&linux.platform, false),
        Err(SetUpError::Unsupported)
    );
    assert_eq!(linux.calls(), Vec::<String>::new());
}
