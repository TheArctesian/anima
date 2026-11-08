//! The systemd reader over `systemctl list-units --all --plain --no-legend --no-pager`
//! output (D-3).

use super::{assert_unreadable_outputs, finished, verdict};
use crate::{checks::probe::systemd, ssh::FixedCommand};

/// Rows as `systemctl list-units --all --plain --no-legend` prints them.
const LISTING: &str = "\
accounts-daemon.service          loaded    active   running Accounts Service
apparmor.service                 loaded    active   exited  Load AppArmor profiles
cron.service                     loaded    inactive dead    Regular background program processing daemon
docker.socket                    loaded    active   running Docker Socket for the API
ghost.service                    not-found inactive dead    ghost.service
nginx.service                    loaded    active   running A high performance web server and a reverse proxy server
nginx-extra.service              loaded    failed   failed  Extra helper for nginx.service
postgresql@16-main.service       loaded    active   running PostgreSQL Cluster 16-main
ssh.service                      loaded    failed   failed  OpenBSD Secure Shell server
sshd-keygen.service              loaded    activating start  start sshd key generation
systemd-journald.socket          loaded    active   running Journal Socket
backup.timer                     loaded    active   waiting Nightly backup
";

fn read(listing: &str, unit: &str) -> (&'static str, Option<String>) {
    let result = systemd::read(&finished(listing), unit);
    let (word, message) = verdict(&result);
    (word, message.map(str::to_owned))
}

fn down(message: &str) -> (&'static str, Option<String>) {
    ("down", Some(message.to_owned()))
}

fn up() -> (&'static str, Option<String>) {
    ("up", None)
}

#[test]
fn chk_012_the_systemd_command_is_static_text_and_never_names_the_unit() {
    let expected = FixedCommand::new("systemctl")
        .arg("list-units")
        .arg("--all")
        .arg("--plain")
        .arg("--no-legend")
        .arg("--no-pager");
    assert_eq!(systemd::command(), expected);
}

#[test]
fn d_3_an_active_unit_is_up_with_no_response_time() {
    let result = systemd::read(&finished(LISTING), "nginx.service");
    assert_eq!(verdict(&result), ("up", None));
    assert_eq!(result.response_time, None);
}

#[test]
fn d_3_a_unit_given_without_a_type_is_a_service() {
    assert_eq!(read(LISTING, "nginx"), up());
    assert_eq!(read(LISTING, "postgresql@16-main"), up());
    assert_eq!(read(LISTING, "docker"), down("the unit was not found"));
}

#[test]
fn d_3_a_unit_that_names_its_type_is_looked_up_as_written() {
    assert_eq!(read(LISTING, "docker.socket"), up());
    assert_eq!(read(LISTING, "backup.timer"), up());
    assert_eq!(read(LISTING, "cron.timer"), down("the unit was not found"));
}

#[test]
fn d_3_every_unit_type_suffix_is_kept_as_written() {
    for suffix in [
        "service",
        "socket",
        "target",
        "timer",
        "mount",
        "automount",
        "swap",
        "path",
        "slice",
        "scope",
        "device",
    ] {
        let listing = format!("thing.{suffix}   loaded active running Thing\n");
        assert_eq!(read(&listing, &format!("thing.{suffix}")), up(), "{suffix}");
        if suffix != "service" {
            assert_eq!(
                read(&listing, "thing"),
                down("the unit was not found"),
                "{suffix}"
            );
        }
    }
}

#[test]
fn d_3_a_name_with_a_dot_and_no_type_is_a_service() {
    let listing = "my.app.service loaded active running My app\n";
    assert_eq!(read(listing, "my.app"), up());
}

#[test]
fn d_3_a_unit_that_is_not_active_is_down_with_its_state_class() {
    let states = [
        ("inactive", "inactive"),
        ("failed", "failed"),
        ("activating", "activating"),
        ("deactivating", "deactivating"),
        ("reloading", "reloading"),
        ("maintenance", "in maintenance"),
        ("refreshing", "not active"),
    ];
    for (active, class) in states {
        let listing = format!("app.service loaded {active} dead An app\n");
        assert_eq!(
            read(&listing, "app"),
            down(&format!("the unit is {class}")),
            "{active}"
        );
    }
}

#[test]
fn d_3_recorded_rows_of_failed_inactive_and_starting_units_are_down() {
    assert_eq!(read(LISTING, "ssh"), down("the unit is failed"));
    assert_eq!(read(LISTING, "cron"), down("the unit is inactive"));
    assert_eq!(read(LISTING, "sshd-keygen"), down("the unit is activating"));
}

#[test]
fn d_3_an_active_oneshot_that_has_exited_is_up() {
    assert_eq!(read(LISTING, "apparmor"), up());
}

#[test]
fn d_3_a_unit_systemd_only_references_is_not_found() {
    assert_eq!(read(LISTING, "ghost"), down("the unit was not found"));
}

#[test]
fn d_3_an_absent_unit_is_not_found() {
    assert_eq!(read(LISTING, "mysql"), down("the unit was not found"));
    assert_eq!(read("", "nginx"), down("the unit was not found"));
}

#[test]
fn d_3_only_the_first_column_names_the_unit() {
    let listing = "other.service loaded active running nginx.service helper\n\
                   nginx-extra.service loaded active running Extra\n";
    assert_eq!(read(listing, "nginx"), down("the unit was not found"));
    assert_eq!(read(listing, "nginx-extra"), up());
}

#[test]
fn d_3_a_bullet_before_the_unit_is_skipped() {
    let listing = "\u{25cf} ssh.service loaded failed failed Secure Shell\n\
                   * cron.service loaded active running Cron\n";
    assert_eq!(read(listing, "ssh"), down("the unit is failed"));
    assert_eq!(read(listing, "cron"), up());
}

#[test]
fn d_3_a_row_with_fewer_than_three_columns_is_ignored() {
    let listing = "nginx.service\nnginx.service loaded\nnginx.service loaded active\n";
    assert_eq!(read(listing, "nginx"), up());
    assert_eq!(
        read("nginx.service\nnginx.service loaded\n", "nginx"),
        down("the unit was not found")
    );
}

#[test]
fn d_3_output_that_cannot_be_read_is_down_with_a_fixed_message() {
    assert_unreadable_outputs(|output| systemd::read(output, "nginx"), LISTING);
}
